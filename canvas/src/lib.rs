use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{window, ErrorEvent, MessageEvent, WebSocket};

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
fn start() -> Result<(), JsValue> {
    console_log!("hello from wasm");

    let ws = connect_to_node()?;
    let (canvas, canvas_context) = make_canvas()?;

    enable_draw(&ws, &canvas, canvas_context.clone())?;
    enable_recv(&ws, canvas_context.clone())?;

    Ok(())
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
    canvas.set_width(window.inner_width().unwrap().as_f64().unwrap() as u32);
    canvas.set_height(window.inner_height().unwrap().as_f64().unwrap() as u32);

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
    canvas: &web_sys::HtmlCanvasElement,
    context: CanvasContext,
) -> Result<(), JsValue> {
    // keep track of whether the mouse is pressed
    let pressed = Rc::new(Cell::new(false));

    // set the line width and color
    context.set_line_width(5.0);
    context.set_stroke_style(&"red".into());
    context.set_line_cap("square");

    // handle mouse press events
    {
        let context = context.clone();
        let pressed = pressed.clone();
        let ws = ws.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
            context.begin_path();
            context.move_to(event.offset_x() as f64, event.offset_y() as f64);
            pressed.set(true);
            // ws.send_with_str(&format!(
            //     "mouse got pressed at ({}, {})",
            //     event.offset_x(),
            //     event.offset_y()
            // ))
            // .unwrap();
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
                let x = event.offset_x() as f64;
                let y = event.offset_y() as f64;
                context.line_to(x, y);
                context.stroke();
                context.begin_path();
                context.move_to(x, y);
                let message = graffitech_lib::CanvasMessage { x, y };
                let message_bytes: Vec<u8> = bincode::serialize(&message).unwrap();
                // Create a Uint8Array view directly over the slice.
                let uint8_array = unsafe { js_sys::Uint8Array::view(&message_bytes) };
                // Send the ArrayBuffer part of the Uint8Array.
                ws.send_with_array_buffer(&uint8_array.buffer()).unwrap();
                drop(message_bytes);
            }
        });
        canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // handle mouse release events
    {
        let ws = ws.clone();
        let closure = Closure::<dyn FnMut(_)>::new(move |event: web_sys::MouseEvent| {
            pressed.set(false);
            context.line_to(event.offset_x() as f64, event.offset_y() as f64);
            context.stroke();
            // ws.send_with_str("mouse got released").unwrap();
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
        // Handle difference Text/Binary,..
        console_log!("message event, received: {:?}", e.data());
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            let array = js_sys::Uint8Array::new(&abuf);
            let message = serde_json::from_slice::<String>(&array.to_vec())
                .unwrap_or("parse error".to_string());
            // clear last message without clearing the canvas
            context.clear_rect(0.0, 0.0, 300.0, 30.0);
            // write the message on the canvas
            context.set_font("16px Arial");
            let _ = context.fill_text(&message, 10.0, 20.0);
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
