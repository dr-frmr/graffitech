use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    window, ErrorEvent, MessageEvent, Request, RequestInit, RequestMode, Response, WebSocket,
};

type CanvasContext = Rc<web_sys::CanvasRenderingContext2d>;

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(start)]
async fn start() -> Result<(), JsValue> {
    console_log!("hello from wasm");

    let my_color = http_get("color").await?;

    let ws = connect_to_node()?;
    let (canvas, canvas_context) = make_canvas()?;

    enable_draw(&ws, my_color, &canvas, canvas_context.clone())?;
    enable_recv(&ws, canvas_context.clone())?;

    Ok(())
}

async fn http_get(path: &str) -> Result<JsValue, JsValue> {
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let url = format!("/{}/{}", graffitech_lib::APP_NAME, path);

    let request = Request::new_with_str_and_init(&url, &opts)?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    let resp: Response = resp_value.dyn_into().unwrap();
    let json = JsFuture::from(resp.json()?).await?;
    Ok(json)
}

/// put a canvas on the page and return drawable context
fn make_canvas() -> Result<(web_sys::HtmlCanvasElement, CanvasContext), JsValue> {
    let window = window().unwrap();

    // produce a canvas element, all of which our drawing will be done on
    let document = window.document().unwrap();
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    document.body().unwrap().append_child(&canvas)?;
    canvas.set_width(50);
    canvas.set_height(50);
    canvas
        .style()
        .set_property("image-rendering", "pixelated")?;
    canvas.style().set_property("width", "500px")?;
    canvas.style().set_property("height", "500px")?;
    canvas.style().set_property("margin", "20px auto")?;
    canvas.style().set_property("display", "block")?;
    canvas.style().set_property("border", "1px solid black")?;
    canvas.style().set_property("border-radius", "10px")?;

    // get the 2d context for the canvas so we can draw on it
    let context = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;
    Ok((canvas, Rc::new(context)))
}

/// open a WebSocket connection to the node
fn connect_to_node() -> Result<WebSocket, JsValue> {
    let window = window().unwrap();

    let protocol = if window.location().protocol().unwrap() == "https:" {
        "wss://"
    } else {
        "ws://"
    };
    let ws = WebSocket::new(&format!(
        "{}{}/{}/ws",
        protocol,
        window.location().host().unwrap(),
        graffitech_lib::APP_NAME
    ))?;

    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    Ok(ws)
}

/// create handlers for drawing events on the canvas
fn enable_draw(
    ws: &WebSocket,
    color: JsValue,
    canvas: &web_sys::HtmlCanvasElement,
    context: CanvasContext,
) -> Result<(), JsValue> {
    // keep track of whether the mouse is pressed
    let pressed = Rc::new(Cell::new(false));

    // set the line width and color
    context.set_line_width(2.0);

    context.set_stroke_style(&color);
    context.set_fill_style(&color);

    // handle mouse press events
    {
        let pressed = pressed.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::MouseEvent| {
            pressed.set(true);
        });
        canvas.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // handle mouse move events if the mouse is pressed
    {
        let context = context.clone();
        let pressed = pressed.clone();
        let ws = ws.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
            if pressed.get() {
                let x = event.offset_x() as f64 / 10.0;
                let y = event.offset_y() as f64 / 10.0;

                // context.line_to(x, y);
                // context.stroke();
                // context.begin_path();
                // context.move_to(x, y);
                context.fill_rect(x, y, 1.0, 1.0);
                let message = graffitech_lib::CanvasMessage {
                    x,
                    y,
                    color: color.as_string().unwrap_or_default(),
                };
                // console_log!("sending message: {}", message);
                let message_bytes: Vec<u8> = serde_json::to_vec(&message).unwrap();
                let uint8_array = js_sys::Uint8Array::from(&message_bytes[..]);
                ws.send_with_array_buffer(&uint8_array.buffer()).unwrap();
            }
        });
        canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // handle mouse release events
    {
        let closure = Closure::<dyn FnMut(_)>::new(move |_event: web_sys::MouseEvent| {
            pressed.set(false);
        });
        canvas.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}

/// create handlers for receiving websocket messages and drawing on the canvas
fn enable_recv(ws: &WebSocket, context: CanvasContext) -> Result<(), JsValue> {
    // create callback
    let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        // console_log!("message event, received: {:?}", e.data());
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let array = js_sys::Uint8Array::new(&abuf);
            let message =
                serde_json::from_slice::<graffitech_lib::CanvasMessage>(&array.to_vec()).unwrap();
            console_log!("received message: {}", message);
            // draw the received message
            context.set_fill_style(&message.color.into());
            context.fill_rect(message.x, message.y, 1.0, 1.0);
            // context.set_stroke_style(&message.color.into());
            // context.line_to(message.x, message.y);
            // context.stroke();
            // context.begin_path();
            // context.move_to(message.x, message.y);
        }
    });

    // set message event handler on WebSocket
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();

    let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
        console_log!("error event: {:?}", e);
    });
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    Ok(())
}
