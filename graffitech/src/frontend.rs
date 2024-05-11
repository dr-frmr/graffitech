use kinode_process_lib::{
    http,
    http::{HttpServerRequest, IncomingHttpRequest, Method, StatusCode},
    println, Address, Message,
};
use std::collections::HashSet;

pub fn serve(our: &Address) {
    http::serve_ui(our, "canvas/dist", true, false, vec!["/"]).expect("couldn't serve UI");
    // http::bind_http_path("/state", true, false).expect("couldn't bind HTTP state path");
    // http::bind_http_path("/post", true, false).expect("couldn't bind HTTP post path");
    http::bind_ws_path("/ws", true, false).expect("couldn't bind WS updates path");

    // add icon to homepage
    kinode_process_lib::homepage::add_to_homepage("Graffi.tech", None, Some("/"), None);
}

pub fn send_ws_updates(value: serde_json::Value, ws_channels: &HashSet<u32>) {
    if ws_channels.is_empty() {
        return;
    }
    let bytes = value.to_string().as_bytes().to_vec();
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
        HttpServerRequest::WebSocketPush { .. } => {
            // for now just print blob as string
            let blob = kinode_process_lib::get_blob();
            if let Some(blob) = blob {
                let string = String::from_utf8_lossy(&blob.bytes);
                println!("WS message received: {}", string);
                // echo back what we got to all channels
                send_ws_updates(string.into(), ws_channels);
            }
        }
    }
    Ok(())
}

pub fn serve_http_paths(
    our: &Address,
    req: IncomingHttpRequest,
    ws_channels: &mut HashSet<u32>,
) -> anyhow::Result<(StatusCode, Vec<u8>)> {
    let method = req.method()?;
    // strips first section of path, which is the process name
    let bound_path = req.path()?;
    let _url_params = req.url_params();

    match bound_path.as_str() {
        _ => Ok((StatusCode::NOT_FOUND, vec![])),
    }
}
