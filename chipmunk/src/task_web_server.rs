use crate::config::Config;
use crate::database::tables::Tables;
use crate::server::{DataToServer, MpscTopic, TeslaServer};
use crate::set_config;
use tokio::sync::mpsc::{self, unbounded_channel};
use tokio::sync::{broadcast, oneshot};
use tokio_util::sync::CancellationToken;

pub async fn web_server_task(
    mut data_rx: broadcast::Receiver<Tables>,
    config: Config,
    tables: &Tables,
    cancellation_token: CancellationToken,
) {
    use broadcast::error::*;
    let name = "web_server_task";

    let (data_from_server_tx, mut data_from_server_rx) = unbounded_channel();
    let (server_exit_signal_tx, server_exit_signal_rx) = oneshot::channel();
    let (data_to_server_tx, data_to_server_rx) = broadcast::channel::<DataToServer>(1);

    let message_handler_task = tokio::task::spawn({
        let config = config.clone();
        async move {
            let name = format!("{name}::message_handler_task");
            loop {
                match data_from_server_rx.try_recv() {
                    Ok(value) => match value {
                        MpscTopic::Logging(value) => {
                            set_config!(config.logging_enabled, value);
                        }
                        MpscTopic::RefreshToken(refresh_token) => {
                            if let Err(e) =
                                tesla_api::auth::refresh_access_token(refresh_token.as_str())
                                    .await
                                    .map(|t| {
                                        set_config!(config.access_token, t.access_token);
                                        set_config!(config.refresh_token, t.refresh_token);
                                    })
                            {
                                log::error!("{e}");
                                continue;
                            }
                        }
                    },
                    Err(e) => match e {
                        mpsc::error::TryRecvError::Disconnected => {
                            log::error!("server_rx channel closed, exiting {name}");
                            break;
                        }
                        mpsc::error::TryRecvError::Empty => (),
                    },
                }

                match data_rx.try_recv() {
                    Ok(data) => {
                        if let Err(e) = data_to_server_tx.send(DataToServer::Tables(data)) {
                            log::error!("Error sending data to web server: {e}");
                        }
                    }
                    Err(TryRecvError::Closed) => {
                        // don't log error message if the channel was closed because of a cancellation request
                        if !cancellation_token.is_cancelled() {
                            log::error!("data_rx channel closed, exiting {name}");
                        }
                        break;
                    }
                    Err(TryRecvError::Empty) => (),
                    Err(TryRecvError::Lagged(n)) => {
                        log::warn!("{name} lagged too far behind; {n} messages skipped")
                    }
                }
                if cancellation_token.is_cancelled() {
                    if let Err(e) = server_exit_signal_tx.send(()) {
                        log::error!("Error sending exit signal to server: {e:?}")
                    }
                    break;
                }
                tokio::task::yield_now().await;
            }
        }
    });

    tokio::select! {
        result = TeslaServer::start(config, tables, data_from_server_tx, data_to_server_rx, server_exit_signal_rx) => {
            match result {
                Ok(_) => log::warn!("web server exited"),
                Err(e) => log::error!("Web server exited: {e}"),
            }
        }
        status = message_handler_task => log::warn!("message handler task exited: {status:?}"),
    }
    tracing::warn!("exiting {name}");
}
