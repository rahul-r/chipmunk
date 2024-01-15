use ui_common::{Json, WsMessage};

pub mod server;

pub fn get_default_wsmsg(id: i32) -> String {
    let msg = WsMessage { id: id.to_string(), ..Default::default() };
    msg.to_string().unwrap()
}
