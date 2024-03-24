use std::sync::{Arc, Mutex};

use async_channel::{unbounded, Receiver, Sender};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use gloo_net::websocket::{futures::WebSocket, Message};
use ui_common::{Json, WsMessage};
use wasm_bindgen_futures::spawn_local;

use crate::get_host;

pub struct Ws {
    // write: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    // read: Arc<Mutex<SplitStream<WebSocket>>>,
    pub tx: Sender<WsMessage>,
    pub rx: Receiver<WsMessage>,
}

impl Ws {
    pub fn start() -> Self {
        let host = match get_host() {
            Ok(h) => h,
            Err(e) => {
                log::error!("Error creating WebSocket URL `{e}`");
                panic!();
            }
        };

        let ws = match WebSocket::open(&format!("{}/websocket", host)) {
            Ok(ws) => ws,
            Err(e) => {
                log::error!("WebSocket connection error: {e}");
                panic!();
            }
        };
        let (write, read) = ws.split();

        let write = Arc::new(Mutex::new(write));
        let read = Arc::new(Mutex::new(read));
        let tx = Self::transmit(write);
        let rx = Self::receive(read);
        Self {
            // read,
            // write,
            tx,
            rx,
        }
    }

    fn transmit(write: Arc<Mutex<SplitSink<WebSocket, Message>>>) -> Sender<WsMessage> {
        let (ws_tx, ws_rx) = unbounded::<WsMessage>();
        let write = write;

        spawn_local(async move {
            while let Ok(msg) = ws_rx.recv().await {
                let msg_str = match msg.to_string() {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("{e}");
                        continue;
                    }
                };

                match write.lock() {
                    Ok(mut w) => {
                        if let Err(e) = w.send(Message::Text(msg_str)).await {
                            log::error!("Error sending WebSocket message `{:?}`: {e}", msg);
                        }
                    }
                    Err(e) => log::error!("Cannot get mutex lock: {e}"),
                }
            }
            log::info!("WebSocket sender exited");
        });

        ws_tx
    }

    fn receive(read: Arc<Mutex<SplitStream<WebSocket>>>) -> Receiver<WsMessage> {
        let (ws_tx, ws_rx) = unbounded::<WsMessage>();
        let read = read;

        spawn_local(async move {
            while let Some(msg) = read.lock().unwrap().next().await {
                let ws_tx_clone = ws_tx.clone();
                let handle_msg = async || -> anyhow::Result<()> {
                    match msg? {
                        Message::Text(t) => ws_tx_clone.send(WsMessage::from_string(&*t)?).await?,
                        Message::Bytes(_) => log::warn!("Binary data received via WebSocket"),
                    };
                    Ok(())
                };

                if let Err(e) = handle_msg().await {
                    log::error!("{e}");
                }
            }

            log::info!("WebSocket Closed");
        });

        ws_rx
    }
}
