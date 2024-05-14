use graffitech_lib::{CanvasMessage, GraffiRequest};
use kinode_process_lib::{
    http,
    http::{HttpServerRequest, IncomingHttpRequest, Method, StatusCode},
    println, Address, Message, Request,
};
use std::collections::HashSet;

pub fn serve(our: &Address) {
    http::serve_ui(our, "dist", true, false, vec!["/"]).expect("couldn't serve UI");
    http::bind_http_path("/color", true, false).expect("couldn't bind HTTP state path");
    // http::bind_http_path("/post", true, false).expect("couldn't bind HTTP post path");
    http::bind_ws_path("/ws", true, false).expect("couldn't bind WS updates path");

    // add icon to homepage
    kinode_process_lib::homepage::add_to_homepage("Graffi.tech", None, Some("/"), None);
}

pub fn send_ws_updates(value: CanvasMessage, ws_channels: &HashSet<u32>) {
    if ws_channels.is_empty() {
        return;
    }
    let bytes = serde_json::to_vec(&value).unwrap();
    for channel_id in ws_channels.iter() {
        http::send_ws_push(
            *channel_id,
            http::WsMessageType::Binary,
            kinode_process_lib::LazyLoadBlob {
                mime: Some("application/json".to_string()),
                bytes: bytes.clone(),
            },
        );
    }
}

pub fn handle_http_request(
    our: &Address,
    message: Message,
    ws_channels: &mut HashSet<u32>,
    friends: &mut HashSet<Address>,
) -> anyhow::Result<()> {
    if !message.is_request() {
        return Ok(());
    }
    let Ok(req) = serde_json::from_slice::<HttpServerRequest>(message.body()) else {
        return Err(anyhow::anyhow!("failed to parse incoming http request"));
    };

    match req {
        HttpServerRequest::Http(req) => match serve_http_paths(our, req, ws_channels) {
            Ok((status_code, body)) => http::send_response(
                status_code,
                Some(std::collections::HashMap::from([(
                    String::from("Content-Type"),
                    String::from("application/json"),
                )])),
                body,
            ),
            Err(e) => {
                http::send_response(StatusCode::INTERNAL_SERVER_ERROR, None, vec![]);
                return Err(e);
            }
        },
        HttpServerRequest::WebSocketOpen { channel_id, .. } => {
            // save channel id for pushing
            ws_channels.insert(channel_id);
        }
        HttpServerRequest::WebSocketClose(channel_id) => {
            // remove channel id
            ws_channels.remove(&channel_id);
        }
        HttpServerRequest::WebSocketPush {
            message_type,
            channel_id,
        } => {
            match message_type {
                http::WsMessageType::Close => {
                    // remove channel id
                    ws_channels.remove(&channel_id);
                }
                _ => {
                    let blob = kinode_process_lib::get_blob();
                    if let Some(blob) = blob {
                        let message: CanvasMessage = serde_json::from_slice(&blob.bytes)?;
                        println!("WS message received: {}", message);
                        // forward message to friends
                        let body = serde_json::to_vec(&GraffiRequest::Draw(message))?;
                        for friend in friends.iter() {
                            Request::to(friend).body(body.clone()).send()?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn serve_http_paths(
    _our: &Address,
    req: IncomingHttpRequest,
    _ws_channels: &mut HashSet<u32>,
) -> anyhow::Result<(StatusCode, Vec<u8>)> {
    let method = req.method()?;
    // strips first section of path, which is the process name
    let bound_path = req.path()?;
    let _url_params = req.url_params();

    match bound_path.as_str() {
        "/color" => match method {
            Method::GET => {
                let color = "red";
                Ok((
                    StatusCode::OK,
                    serde_json::to_vec(&serde_json::json!(color))?,
                ))
            }
            _ => Ok((StatusCode::METHOD_NOT_ALLOWED, vec![])),
        },
        _ => Ok((StatusCode::NOT_FOUND, vec![])),
    }
}
