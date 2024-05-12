#![feature(let_chains)]
use kinode_process_lib::{await_message, call_init, println, Address};
use std::collections::HashSet;

mod frontend;
mod state;

wit_bindgen::generate!({
    path: "wit",
    world: "process",
});

call_init!(init);
fn init(our: Address) {
    // let mut state = if let Some(state) = kinode_process_lib::get_state()
    //     && let Ok(state) = serde_json::from_slice::<State>(&state)
    // {
    //     println!("loading saved state");
    //     state
    // } else {
    //     println!("generating new state");
    //     let state = State::new(&our);
    //     state
    // };

    let mut ws_channels: HashSet<u32> = HashSet::new();
    frontend::serve(&our);

    loop {
        handle_message(&our, &mut ws_channels)
            .map_err(|e| println!("error: {:?}", e))
            .ok();
    }
}

fn handle_message(our: &Address, ws_channels: &mut HashSet<u32>) -> anyhow::Result<()> {
    match await_message() {
        Ok(message) => {
            if message.is_local(our) {
                if message.is_process("http_server:distro:sys") {
                    // handle http requests
                    frontend::handle_http_request(our, message, ws_channels)
                } else {
                    Ok(())
                }
            } else {
                Ok(())
            }
        }
        Err(send_error) => Ok(()),
    }
}
