use kinode_process_lib::{
    await_message, call_init, println, Address, ProcessId, Request, Response,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

mod state;

wit_bindgen::generate!({
    path: "wit",
    world: "process",
});

call_init!(init);
fn init(our: Address) {
    println!("begin");

    loop {}
}
