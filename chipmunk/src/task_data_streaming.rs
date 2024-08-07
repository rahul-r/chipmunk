use crate::config::Config;
use crate::get_config;
use crate::tasks::DataTypes;
use tesla_api::stream::StreamingData;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub async fn data_streaming_task(
    data_tx: mpsc::Sender<DataTypes>,
    config: Config,
    cancellation_token: CancellationToken,
    vehicle_id: u64,
) {
    use mpsc::error::*;
    let name = "data_stream_task";
    let (streaming_data_tx, mut streaming_data_rx) = tokio::sync::mpsc::channel::<StreamingData>(1);

    let access_token = match get_config!(config.access_token) {
        Ok(v) => v,
        Err(e) => {
            log::error!("Error getting config `access_token`: {e}");
            return;
        }
    };

    let streaming_data_tx = streaming_data_tx.clone();
    let cancellation_token_clone = cancellation_token.clone();

    let stream_data_handler_task = tokio::task::spawn({
        async move {
            let name = format!("{name}::stream_data_handler_task");
            loop {
                match streaming_data_rx.try_recv() {
                    Ok(data) => {
                        if let Err(e) = data_tx.send(DataTypes::StreamingData(data)).await {
                            // don't log error message if the channel was closed because of a cancellation request
                            if !cancellation_token.is_cancelled() {
                                log::error!("{name}: cannot send data over data_tx: {e}");
                            }
                        }
                    }
                    Err(TryRecvError::Empty) => (),
                    Err(TryRecvError::Disconnected) => {
                        // don't log error message if the channel was disconnected because of cancellation request
                        if !cancellation_token.is_cancelled() {
                            log::error!("streaming_data_rx channel disconnected, exiting {name}");
                        }
                        break;
                    }
                }
                if cancellation_token.is_cancelled() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        }
    });

    tokio::select! {
        result = tesla_api::stream::start(
            &access_token,
            vehicle_id,
            streaming_data_tx,
            cancellation_token_clone,
        ) => {
            match result {
                Ok(_) => log::warn!("Vehicle data streaming stopped"),
                Err(e) => log::error!("Error streaming: {e}"),
            }
        }
        // status = streaming_task => log::warn!("stream data task exited: {status:?}"),
        status = stream_data_handler_task => log::warn!("stream data handler task exited: {status:?}"),
    }

    tracing::warn!("exiting {name}");
}
