use leptos::*;

use leptos_use::core::ConnectionReadyState;
use ui_common::{Status, Topic, WsMessage};

use crate::{components::map::Map, WebsocketContext};

fn log_stat(status: Status) -> impl IntoView {
    view! {
        <div class="flex flex-row w-full bg-white md:border md:border-gray-200 md:rounded-lg shadow md:max-w-md dark:border-gray-700 dark:bg-gray-800">
            <div class="flex items-center">
                <img class="object-scale-down h-auto max-h-96 md:h-auto md:max-h-96" src="/public/model3-red.jpeg" alt="car"/>
            </div>
            <div class="flex flex-col p-5 leading-normal text-center w-full">
                <h5 class="mb-2 text-2xl font-bold tracking-tight text-gray-900 dark:text-white inline-block align-top">car name</h5>
                <div class="space-y-0">
                    <div class="font-normal text-gray-700 dark:text-gray-400">{"charge_state: "}{status.charging.map(|c| c.charge_added)}</div>
                    <div class="font-normal text-gray-700 dark:text-gray-400">{"current_points: "}{status.logging.current_num_points}</div>
                    <div class="font-normal text-gray-700 dark:text-gray-400">{"is_logging: "}{status.logging.enabled}</div>
                    <div class="font-normal text-gray-700 dark:text-gray-400">{"is_user_present: "}{status.vehicle.is_user_nearby}</div>
                    <div class="font-normal text-gray-700 dark:text-gray-400">{"odometer: "}{status.vehicle.odometer}</div>
                    <div class="font-normal text-gray-700 dark:text-gray-400">{"total_points: "}{status.logging.total_num_points}</div>
                </div>
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
        <>
            <Map lat=37.49 lon=-121.94/>
            <div class="flex justify-center items-center md:pt-5">
              {move || log_stat(websocket.logging_status.get())}
            </div>
            <button on:click=move |_| logging_status(!websocket.is_logging.get()) disabled=move || !connected() class="text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800">
                {move || if websocket.is_logging.get() { "Stop" } else { "Start"}}
            </button>
        </>
    }
}
