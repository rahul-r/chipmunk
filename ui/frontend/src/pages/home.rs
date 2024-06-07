use leptos::*;

use leptos_use::core::ConnectionReadyState;
use ui_common::{Topic, WsMessage};

use crate::{components::map::Map, WebsocketContext};

#[component]
pub fn Home() -> impl IntoView {
    let websocket = expect_context::<WebsocketContext>();

    let ws_send = websocket.send.clone();
    let logging_status = move |enable: bool| {
        let topic = match enable {
            true => Topic::StartLogging,
            false => Topic::StopLogging,
        };

        let msg = WsMessage::command(topic, None);

        ws_send(&msg.to_string().unwrap());
    };

    let connected = move || websocket.ready_state.get() == ConnectionReadyState::Open;

    let (is_logging, set_is_logging) = create_signal(false);

    view! {
        <div>
            <Map lat=37.49 lon=-121.94/>

            <button on:click=move |_| logging_status(!is_logging.get()) disabled=move || !connected()>
                {move || if is_logging() { "Stop" } else { "Start"}}
            </button>

            <p>{move || {
                if let Some(msg) = websocket.message.get() {
                    match WsMessage::from_string(&*msg) {
                        Ok(m) => {
                            if let Topic::LoggingStatus = m.topic {
                                let status = ui_common::Status::from_value(m.data.unwrap()).unwrap();
                                set_is_logging(status.logging.enabled);
                                Some(view! {
                                    <div>
                                        <div>{"charge_state: "}{status.charging.map(|c| c.charge_added)}</div>
                                        <div>{"current_points: "}{status.logging.current_num_points}</div>
                                        <div>{"is_logging: "}{status.logging.enabled}</div>
                                        <div>{"is_user_present: "}{status.vehicle.is_user_nearby}</div>
                                        <div>{"odometer: "}{status.vehicle.odometer}</div>
                                        <div>{"total_points: "}{status.logging.total_num_points}</div>
                                    </div>
                                })
                            } else {
                                None
                            }
                        }
                        Err(_e) => Some(view!{ <div>format!("Cannot convert websocket message to a known message type: {}", e) </div>}),
                    }
                } else {
                    None
                }
            }}</p>
        </div>
    }
}
