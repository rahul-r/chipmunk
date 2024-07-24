pub mod status;

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use serde_json::json;
use status::LoggingStatus;
use tokio::sync::{broadcast, mpsc, oneshot, watch, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;
use warp::ws::{Message, WebSocket};
use warp::Filter;

use ui_common::{units::Measurement, MessageType, Topic, WsMessage, WsMessageToken};

use crate::{
    config::Config,
    database::{
        tables::Tables,
        types::{UnitOfLength, UnitOfPressure, UnitOfTemperature},
    },
};

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
    logging_enabled_watcher: watch::Receiver<bool>,
    unit_of_length_watcher: watch::Receiver<UnitOfLength>,
    unit_of_temperature_watcher: watch::Receiver<UnitOfTemperature>,
    unit_of_pressure_watcher: watch::Receiver<UnitOfPressure>,
}

#[derive(Clone)]
pub enum DataToServer {
    Tables(Tables),
}

fn project_root() -> anyhow::Result<PathBuf> {
    let mut dir = std::env::current_exe()?;
    loop {
        let mut cargo_lock = dir.clone();
        cargo_lock.push("Cargo.lock");
        if cargo_lock.exists() {
            return Ok(dir);
        } else if !dir.pop() {
            anyhow::bail!("Cannot determine root of project. Cargo.toml not found");
        }
    }
}

fn find_dist_dir() -> anyhow::Result<PathBuf> {
    let root_dir = project_root()?;

    let dist_dir = root_dir.join("target/dist");
    if dist_dir.join("index.html").exists() {
        return Ok(dist_dir);
    }

    let dist_dir_alt = root_dir.join("ui/frontend/dist");
    log::warn!("Cannot find index.html in {dist_dir:?}, trying {dist_dir_alt:?}");
    if dist_dir_alt.join("index.html").exists() {
        return Ok(dist_dir_alt);
    }

    log::error!("Cannot find index.html in either {dist_dir:?} or {dist_dir_alt:?}");
    anyhow::bail!("Cannot find index.html in either {dist_dir:?} or {dist_dir_alt:?}");
}

impl TeslaServer {
    pub async fn start(
        config: Config,
        tables: &Tables,
        port: u16,
        data_from_srv_tx: mpsc::UnboundedSender<MpscTopic>,
        mut data_to_srv_rx: broadcast::Receiver<DataToServer>,
        exit_signal_rx: oneshot::Receiver<()>,
    ) -> anyhow::Result<()> {
        let clients = Clients::default(); // Keep track of all connected clients
        let clients_copy = clients.clone();
        let with_clients = warp::any().map(move || clients_copy.clone());

        let config_clone = config.clone();
        let websocket = warp::path("websocket")
            .and(warp::ws())
            .and(with_clients)
            .map(move |ws: warp::ws::Ws, clients: Clients| {
                let tx = data_from_srv_tx.clone();
                let config = config_clone.clone();
                ws.on_upgrade(move |socket| {
                    TeslaServer::client_connected(socket, clients, tx, config)
                })
            });

        let mut dist_dir = find_dist_dir()?;

        // handle path "/"
        let mut index_html = dist_dir.clone();
        index_html.push("index.html");
        let index = warp::get()
            .and(warp::path::end())
            .and(warp::fs::file(index_html));

        // handle path "/xxxx" (e.g. http://hostname/index.html loads static/index.html)
        let static_dir = warp::fs::dir(dist_dir.clone());

        // handle path "/public" (e.g. http://hostname/public/image.png loads static/public/image.png)
        dist_dir.push("public");
        let public_dir = warp::fs::dir(dist_dir);

        // TODO: Fix CORS
        // let cors = warp::cors()
        //     .allow_any_origin()
        //     .allow_methods(vec!["GET", "POST", "DELETE"])
        //     .allow_headers(vec!["Authorization", "Content-Type", "Access-Control-Allow-Origin"])
        //     .build();
        // let routes = index.or(static_dir).or(websocket).with(cors);
        let routes = index.or(static_dir).or(public_dir).or(websocket);

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

        let status = LoggingStatus::new(&config, tables);

        let unit_of_length_watcher = match config.unit_of_length.lock() {
            Ok(v) => v.watch(),
            Err(e) => {
                log::error!("Error subscribing to config value `unit_of_length`: {e}");
                panic!("{e}");
            }
        };

        let unit_of_temperature_watcher = match config.unit_of_temperature.lock() {
            Ok(v) => v.watch(),
            Err(e) => {
                log::error!("Error subscribing to config value `unit_of_temperature`: {e}");
                panic!("{e}");
            }
        };

        let unit_of_pressure_watcher = match config.unit_of_pressure.lock() {
            Ok(v) => v.watch(),
            Err(e) => {
                log::error!("Error subscribing to config value `unit_of_pressure`: {e}");
                panic!("{e}");
            }
        };

        let logging_enabled_watcher = match config.logging_enabled.lock() {
            Ok(v) => v.watch(),
            Err(e) => {
                log::error!("Error subscribing to config value `logging_enabled`: {e}");
                panic!("{e}");
            }
        };

        let srv = Arc::new(tokio::sync::Mutex::new(TeslaServer {
            clients,
            status,
            logging_enabled_watcher,
            unit_of_length_watcher,
            unit_of_temperature_watcher,
            unit_of_pressure_watcher,
        }));

        // Handle the messages coming from other tasks
        let message_handler_task = {
            let srv = srv.clone();
            tokio::task::spawn(async move {
                loop {
                    match data_to_srv_rx.recv().await {
                        Ok(v) => match v {
                            DataToServer::Tables(tables) => {
                                srv.lock().await.status.update(&tables, &config)
                            }
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
            async move {
                loop {
                    // We receive the status report from the logger task in regular interval and
                    // store it in the `status` variable. This task will read this variable and
                    // sends it to the clients. Since the logger task only sends the status updates
                    // if it gets data from the vehicle, there is a chance that we don't have the
                    // latest status. Use the `timestamp` field of the status struct to determine
                    // how old the data is.
                    {
                        let mut srv_locked = srv.lock().await;
                        let status_msg = srv_locked.get_status_msg();
                        srv_locked.broadcast(status_msg).await;
                    }
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

        Ok(())
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
        config: Config,
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

            if let Err(e) =
                TeslaServer::handle_messages(&client_tx, msg, tx.clone(), config.clone()).await
            {
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

        if let Err(disconnected) = client.send(Message::text(msg_str)) {
            log::info!("Error {disconnected}");
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
        config: Config,
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
            Topic::Unknown => log::error!("Unknown command received"),
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
            Topic::SetUnit => {
                let Some(data) = ws_msg.clone().data else {
                    let resp = ws_msg.response_with_data(
                        json!({"status": false, "reason": "Invalid measurement unit"}),
                    );
                    TeslaServer::send(client, &resp)?;
                    anyhow::bail!("No token provided");
                };

                let measurement: Measurement = match serde_json::from_value(data) {
                    Ok(v) => v,
                    Err(e) => {
                        let response = json!({"status": false, "reason": e.to_string()});
                        let resp = ws_msg.response_with_data(response);
                        TeslaServer::send(client, &resp)?;
                        anyhow::bail!(e);
                    }
                };

                let result = match measurement {
                    Measurement::Distance(unit) => match config.unit_of_length.lock() {
                        Ok(mut l) => {
                            l.set(UnitOfLength::from_ui_struct(&unit));
                            Ok(())
                        }
                        Err(e) => Err(format!("{e}")),
                    },

                    Measurement::Pressure(unit) => match config.unit_of_pressure.lock() {
                        Ok(mut l) => {
                            l.set(UnitOfPressure::from_ui_struct(&unit));
                            Ok(())
                        }
                        Err(e) => Err(format!("{e}")),
                    },
                    Measurement::Temperature(unit) => match config.unit_of_temperature.lock() {
                        Ok(mut l) => {
                            l.set(UnitOfTemperature::from_ui_struct(&unit));
                            Ok(())
                        }
                        Err(e) => Err(format!("{e}")),
                    },
                };

                let response = if let Err(e) = result {
                    json!({"status": false, "reason": e.to_string()})
                } else {
                    json!({"status": true})
                };
                let resp = ws_msg.response_with_data(response);
                TeslaServer::send(client, &resp)?;
            }
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

    pub fn get_status_msg(&mut self) -> String {
        if self
            .unit_of_length_watcher
            .has_changed()
            .map_err(|e| log::error!("{e}"))
            .unwrap_or(false)
        {
            let new_value = *self.unit_of_length_watcher.borrow_and_update();
            self.status.set_unit_of_length(new_value);
        }

        if self
            .unit_of_temperature_watcher
            .has_changed()
            .map_err(|e| log::error!("{e}"))
            .unwrap_or(false)
        {
            let new_value = *self.unit_of_temperature_watcher.borrow_and_update();
            self.status.set_unit_of_temperature(new_value);
        }

        if self
            .unit_of_pressure_watcher
            .has_changed()
            .map_err(|e| log::error!("{e}"))
            .unwrap_or(false)
        {
            let new_value = *self.unit_of_pressure_watcher.borrow_and_update();
            self.status.set_unit_of_pressure(new_value);
        }

        if self
            .logging_enabled_watcher
            .has_changed()
            .map_err(|e| log::error!("{e}"))
            .unwrap_or(false)
        {
            let new_status = *self.logging_enabled_watcher.borrow_and_update();
            self.status.set_logging_status(new_status);
        }

        let msg = WsMessage {
            id: Uuid::new_v4().to_string(),
            r#type: MessageType::Response,
            topic: Topic::LoggingStatus,
            data: self.status.to_value().map_err(|e| log::error!("{e}")).ok(),
        };

        msg.to_string()
            .map_err(|e| log::error!("Error converting WsMessage to string: {e}"))
            .unwrap_or("".into())
    }
}
