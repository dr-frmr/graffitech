#![feature(let_chains)]
use graffitech_lib::{GraffiRequest, GraffiResponse};
use kinode_process_lib::{await_message, call_init, println, Address, Response};
use std::collections::HashSet;

mod frontend;
mod state;

wit_bindgen::generate!({
    path: "wit",
    world: "process",
});

call_init!(init);
fn init(our: Address) {
    let mut ws_channels: HashSet<u32> = HashSet::new();
    let mut friends: HashSet<Address> = HashSet::new();
    frontend::serve(&our);

    loop {
        handle_message(&our, &mut ws_channels, &mut friends)
            .map_err(|e| println!("error: {:?}", e))
            .ok();
    }
}

fn handle_message(
    our: &Address,
    ws_channels: &mut HashSet<u32>,
    friends: &mut HashSet<Address>,
) -> anyhow::Result<()> {
    match await_message() {
        Ok(message) => {
            if message.is_local(our) {
                if message.is_process("http_server:distro:sys") {
                    // handle http requests
                    frontend::handle_http_request(our, message, ws_channels, friends)
                } else {
                    Ok(())
                }
            } else {
                // handle remote requests
                if message.is_request() {
                    let req = serde_json::from_slice::<GraffiRequest>(&message.body())?;
                    match req {
                        GraffiRequest::AddPlayer(address) => {
                            friends.insert(address.parse().unwrap());
                            println!("Added player: {}", address);
                            Response::new()
                                .body(serde_json::to_vec(&GraffiResponse::Cool)?)
                                .send()
                                .unwrap();
                        }
                        GraffiRequest::RemovePlayer(address) => {
                            friends.remove(&address.parse().unwrap());
                            println!("Removed player: {}", address);
                            Response::new()
                                .body(serde_json::to_vec(&GraffiResponse::Cool)?)
                                .send()
                                .unwrap();
                        }
                        GraffiRequest::Draw(draw) => {
                            println!("Remote draw received: {}", draw);
                            frontend::send_ws_updates(draw, ws_channels)
                        }
                    }
                }
                Ok(())
            }
        }
        Err(_send_error) => Ok(()),
    }
}
