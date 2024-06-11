use leptos::*;

use leptos_use::core::ConnectionReadyState;
use ui_common::{Status, Topic, WsMessage};

use crate::{components::map::Map, WebsocketContext};

fn log_stat(status: Status) -> impl IntoView {
    view! {
        <div class="flex flex-col items-center bg-white border border-gray-200 rounded-lg shadow md:flex-row md:max-w-xl dark:border-gray-700 dark:bg-gray-800 ">
            <img class="object-cover w-full rounded-t-lg h-96 md:h-auto md:w-48 md:rounded-none md:rounded-s-lg" src="/docs/images/blog/image-4.jpg" alt="car"/>
            <div class="flex flex-col justify-between p-4 leading-normal">
                <h5 class="mb-2 text-2xl font-bold tracking-tight text-gray-900 dark:text-white">car name</h5>
                <div>{"charge_state: "}{status.charging.map(|c| c.charge_added)}</div>
                <div>{"current_points: "}{status.logging.current_num_points}</div>
                <div>{"is_logging: "}{status.logging.enabled}</div>
                <div>{"is_user_present: "}{status.vehicle.is_user_nearby}</div>
                <div>{"odometer: "}{status.vehicle.odometer}</div>
                <div>{"total_points: "}{status.logging.total_num_points}</div>
            </div>
        </div>
    }
}

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

    view! {
        <div>
            <Map lat=37.49 lon=-121.94/>
            <div class="flex flex-col h-screen my-auto items-center bgimg bg-cover pt-5">
                <p>{move || log_stat(websocket.logging_status.get())}</p>
                <button on:click=move |_| logging_status(!websocket.is_logging.get()) disabled=move || !connected() class="text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800">
                    {move || if websocket.is_logging.get() { "Stop" } else { "Start"}}
                </button>
            </div>
        </div>
    }
}
