pub mod status;

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use serde_json::json;
use status::LoggingStatus;
use tokio::sync::Mutex;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};
use warp::Filter;

use ui_common::{MessageType, Topic, WsMessage, WsMessageToken};

use crate::{config::Config, database::tables::Tables};

// static SERVER: OnceLock<TeslaServer> = OnceLock::new();

// pub fn get_server(port: u16, tx: mpsc::UnboundedSender<MpscTopic>) -> &'static TeslaServer {
//     SERVER.get_or_init(|| {
//         TeslaServer::start(port, tx)
//     })
// }

/// Unique client id counter.
static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

/// State of currently connected clients.
/// - Key is client id
/// - Value is sender of `warp::ws::Message`
type Clients = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

#[derive(Debug)]
pub enum MpscTopic {
    Logging(bool),
    RefreshToken(String),
}

pub struct TeslaServer {
    clients: Clients,
    status: LoggingStatus,
    config: Config,
}

#[derive(Clone)]
pub enum DataToServer {
    Tables(Tables),
}

impl TeslaServer {
    pub async fn start(
        config: Config,
        port: u16,
        data_from_srv_tx: mpsc::UnboundedSender<MpscTopic>,
        mut data_to_srv_rx: broadcast::Receiver<DataToServer>,
        exit_signal_rx: oneshot::Receiver<()>,
    ) -> anyhow::Result<Arc<Mutex<TeslaServer>>> {
        let clients = Clients::default(); // Keep track of all connected clients
        let clients_copy = clients.clone();
        let with_clients = warp::any().map(move || clients_copy.clone());

        let websocket = warp::path("websocket")
            .and(warp::ws())
            .and(with_clients)
            .map(move |ws: warp::ws::Ws, clients: Clients| {
                let tx = data_from_srv_tx.clone();
                ws.on_upgrade(move |socket| TeslaServer::client_connected(socket, clients, tx))
            });

        // create path to "dist" directory and "index.html"
        let mut dist_dir = std::env::current_exe().expect("Cannot get executable path");
        dist_dir.pop();
        dist_dir.push("dist");
        let mut index_html = dist_dir.clone();
        index_html.push("index.html");

        if !index_html.exists() {
            log::error!("{:?} does not exist", index_html);
            anyhow::bail!("{:?} does not exist", index_html);
        }

        // handle path "/"
        let index = warp::get()
            .and(warp::path::end())
            .and(warp::fs::file(index_html));

        // handle path "/xxxx" (e.g. http://hostname/index.html loads static/index.html)
        let static_dir = warp::fs::dir(dist_dir);

        // TODO: Fix CORS
        // let cors = warp::cors()
        //     .allow_any_origin()
        //     .allow_methods(vec!["GET", "POST", "DELETE"])
        //     .allow_headers(vec!["Authorization", "Content-Type", "Access-Control-Allow-Origin"])
        //     .build();
        // let routes = index.or(static_dir).or(websocket).with(cors);
        let routes = index.or(static_dir).or(websocket);

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        log::info!("Listening on http://{}", address);
        let signal = async {
            exit_signal_rx.await.ok();
        };
        let (_addr, server) =
            match warp::serve(routes).try_bind_with_graceful_shutdown(address, signal) {
                Ok(r) => r,
                Err(e) => anyhow::bail!(e),
            };

        let _srv = TeslaServer {
            clients,
            status: LoggingStatus::default(),
            config: config.clone(),
        };

        //TODO: Subscribe to config.is_logging and update LoggingStatus when the config changes
        //match config.logging_enabled.lock() {
        //    Ok(mut v) => {
        //        v.subscribe_closure(|status| {
        //            log::info!("Logging enabled status changed to `{status}`. Updating server status with the new value");
        //            _srv.status.set_logging_status(status);
        //        });
        //    }
        //    Err(e) => log::error!("Error subscribing to config value `logging_enabled`: {e}"),
        //}

        let srv = Arc::new(Mutex::new(_srv));

        // Handle the messages coming from other tasks
        let message_handler_task = {
            let srv = srv.clone();
            tokio::task::spawn(async move {
                loop {
                    match data_to_srv_rx.recv().await {
                        Ok(v) => match v {
                            DataToServer::Tables(tables) => srv.lock().await.status.update(&tables),
                        },
                        Err(e) => {
                            log::warn!("{e}");
                            tokio::time::sleep(Duration::from_millis(500)).await;
                        }
                    }
                }
            })
        };

        // Send the logging status to all connected web interface clients
        let status_reporter = tokio::task::spawn({
            let srv = srv.clone();
            async move {
                loop {
                    let srv = srv.lock().await;
                    // We receive the status report from the logger task in regular interval and
                    // store it in the `status` variable. This task will read this variable and
                    // sends it to the clients. Since the logger task only sends the status updates
                    // if it gets data from the vehicle, there is a chance that we don't have the
                    // latest status. Use the `timestamp` field of the status struct to determine
                    // how old the data is.
                    let msg = srv.get_status_str();
                    srv.broadcast(msg).await;
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        });

        tokio::select! {
            _ = server => log::error!("Server exited"),
            status = message_handler_task => log::error!("Message handler exited: {status:?}"),
            _ = status_reporter => (),
        }

        tracing::error!("Exiting server task");

        Ok(srv)
    }

    /// Broadcast message to all connected clients
    pub async fn broadcast(&self, msg: String) {
        for (&_client_id, tx) in self.clients.read().await.iter() {
            if let Err(disconnected) = tx.send(Message::text(&msg)) {
                log::error!("Error {disconnected}");
            }
        }
    }

    async fn client_connected(
        ws: WebSocket,
        clients: Clients,
        tx: mpsc::UnboundedSender<MpscTopic>,
    ) {
        let client_id = NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed);

        // Split the socket into a sender and receive of messages.
        let (mut client_ws_tx, mut client_ws_rx) = ws.split();

        // Use an unbounded channel to handle buffering and flushing of messages
        // to the websocket...
        let (client_tx, client_rx) = mpsc::unbounded_channel();
        let mut rx = UnboundedReceiverStream::new(client_rx);

        tokio::task::spawn(async move {
            while let Some(message) = rx.next().await {
                client_ws_tx
                    .send(message)
                    .unwrap_or_else(|e| {
                        log::error!("websocket send error: {}", e);
                    })
                    .await;
            }
        });

        // Save the sender in our list of connected clients.
        clients.write().await.insert(client_id, client_tx.clone());

        while let Some(result) = client_ws_rx.next().await {
            let msg = match result {
                Ok(msg) => msg,
                Err(e) => {
                    log::error!("websocket error(uid={}): {}", client_id, e);
                    break;
                }
            };

            if let Err(e) = TeslaServer::handle_messages(&client_tx, msg, tx.clone()).await {
                // log::error!("{} {}", e, e.backtrace());
                log::error!("{}", e);
            }
        }

        // client_ws_rx stream will keep processing as long as the client stays
        // connected. Once they disconnect, remove from the client list
        clients.write().await.remove(&client_id);
    }

    fn send(client: &mpsc::UnboundedSender<Message>, msg: &WsMessage) -> anyhow::Result<()> {
        let msg_str = msg.to_string()?;

        if let Err(_disconnected) = client.send(Message::text(msg_str)) {
            log::info!("Error {_disconnected}");
        }

        Ok(())
    }

    /// Clients send commands in the following format
    /// {
    ///  "id": "<some id to connect the commands and responses>",
    ///  "type": "command",
    ///  "command": "<command>",
    ///  "params": ["<param1>", "<param2>"]
    /// }
    ///
    /// The server will send responses in the following format
    /// {
    ///   "id": "<same id as the command>",
    ///   "type": "response",
    ///   "response": <response to the command>
    /// }
    async fn handle_messages(
        client: &mpsc::UnboundedSender<Message>,
        msg: Message,
        tx: mpsc::UnboundedSender<MpscTopic>,
    ) -> anyhow::Result<()> {
        if msg.is_close() {
            let frame = msg.close_frame();
            let (code, reason) = match frame {
                Some(cr) => (cr.0.to_string(), cr.1),
                None => ("".to_string(), ""),
            };
            log::info!("WebSocket closing - code `{}`, reason `{}`", code, reason);
            return Ok(());
        }

        let Ok(msg) = msg.to_str() else {
            anyhow::bail!("Non text message received: {msg:?}");
        };

        let ws_msg = WsMessage::from_string(msg)?;

        match ws_msg.topic {
            Topic::StartLogging => {
                let response = match tx.send(MpscTopic::Logging(true)) {
                    Ok(()) => json!({"status": true}),
                    Err(e) => json!({"status": false, "reason": e.to_string()}),
                };
                let resp = ws_msg.response_with_data(response);
                TeslaServer::send(client, &resp)?;
            }
            Topic::StopLogging => {
                let response = match tx.send(MpscTopic::Logging(false)) {
                    Ok(()) => json!({"status": true}),
                    Err(e) => json!({"status": false, "reason": e.to_string()}),
                };
                let resp = ws_msg.response_with_data(response);
                TeslaServer::send(client, &resp)?;
            }
            Topic::LoggingStatus => (),
            Topic::GetServerSettings => (),
            Topic::SetSettings => (),
            Topic::GetSettings => (),
            Topic::RefreshToken => {
                let Some(token_value) = ws_msg.clone().data else {
                    let resp = ws_msg.response_with_data(
                        json!({"status": false, "reason": "No token provided"}),
                    );
                    TeslaServer::send(client, &resp)?;
                    anyhow::bail!("No token provided");
                };

                let token = match WsMessageToken::from_value(token_value) {
                    Ok(t) => t.token,
                    Err(e) => {
                        let resp = ws_msg
                            .response_with_data(json!({"status": false, "reason": e.to_string()}));
                        TeslaServer::send(client, &resp)?;
                        anyhow::bail!("Cannot parse token");
                    }
                };

                let response = match tx.send(MpscTopic::RefreshToken(token)) {
                    Ok(()) => json!({"status": true}),
                    Err(e) => json!({"status": false, "reason": e.to_string()}),
                };

                let resp = ws_msg.response_with_data(response);
                TeslaServer::send(client, &resp)?;
            }
            Topic::Unknown => log::error!("Unknown command received"),
        }

        Ok(())
    }

    /// Handle start logging
    /// Command:
    /// {
    ///  "id": "<some id to connect the commands and responses>",
    ///  "type": "command",
    ///  "command": "start",
    ///  "params": []
    /// }
    ///
    /// Response on success:
    /// {
    ///   "id": "<same id as the command>",
    ///   "type": "response",
    ///   "response": {
    ///     "status": true
    ///   }
    /// }
    ///
    /// Response on failure:
    /// {
    ///   "id": "<same id as the command>",
    ///   "type": "response",
    ///   "response": {
    ///     "status": false,
    ///     "message": "error message or reason for failure"
    ///   }
    /// }
    fn _handle_start(
        client: &mpsc::UnboundedSender<Message>,
        cmd: WsMessage,
    ) -> anyhow::Result<()> {
        log::info!("Start logging");

        let response = json!({"status": true});
        let resp = cmd.response_with_data(response);
        TeslaServer::send(client, &resp)?;

        Ok(())
    }

    fn _handle_stop(client: &mpsc::UnboundedSender<Message>, cmd: WsMessage) -> anyhow::Result<()> {
        log::info!("Stop logging");

        let response = json!({"status": true});
        let resp = cmd.response_with_data(response);
        TeslaServer::send(client, &resp)?;

        Ok(())
    }

    pub fn get_status_str(&self) -> String {
        let msg = WsMessage {
            id: Uuid::new_v4().to_string(),
            r#type: MessageType::Response,
            topic: Topic::LoggingStatus,
            data: match self.status.to_value() {
                Ok(v) => Some(v),
                Err(e) => {
                    log::error!("{e}");
                    None
                }
            },
        };
        msg.to_string().unwrap()
    }
}
