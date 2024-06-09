use leptos::*;
use serde_json::json;

use ui_common::{Topic, WsMessage};

use crate::WebsocketContext;

#[component]
pub fn Settings() -> impl IntoView {
    let (refresh_token, set_refresh_token) = create_signal("".to_string());

    let websocket = expect_context::<WebsocketContext>();

    let send_refresh_token = move |_| {
        let data = json!({"token": refresh_token.get()});
        let msg = WsMessage::command(Topic::RefreshToken, Some(data));
        logging::log!("{:?}", msg);
        // TODO: Send token to the backend via websocket
        websocket.send(&msg.to_string().unwrap());
    };

    view! {
        <div class="max-w-sm mx-auto">
            <div class="mb-5">
                <label for="refresh_token" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">Refresh token</label>
                <input type="text" id="refresh_token" on:input=move |ev| set_refresh_token(event_target_value(&ev)) class="bg-gray-50 border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"/>
            </div>

            <div class="mb-5">
                <label for="distance_unit" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">Distance</label>
                <select id="distance_unit" class="block w-full p-2 mb-6 text-sm text-gray-900 border border-gray-300 rounded-lg bg-gray-50 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500">
                    <option>mi</option>
                    <option>km</option>
                </select>
            </div>

            <div class="mb-5">
                <label for="temperature_unit" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">Temperature</label>
                <select id="temperature_unit" class="block w-full p-2 mb-6 text-sm text-gray-900 border border-gray-300 rounded-lg bg-gray-50 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500">
                    <option>"°F"</option>
                    <option>"°C"</option>
                </select>
            </div>

            <div class="mb-5">
                <label for="pressure_unit" class="block mb-2 text-sm font-medium text-gray-900 dark:text-white">Pressure</label>
                <select id="pressure_unit" class="block w-full p-2 mb-6 text-sm text-gray-900 border border-gray-300 rounded-lg bg-gray-50 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500">
                    <option>psi</option>
                    <option>nar</option>
                </select>
            </div>
            <div class="grid">
                <button type="button" on:click=send_refresh_token class="text-white bg-blue-700 hover:bg-blue-800 focus:outline-none focus:ring-4 focus:ring-blue-300 font-medium rounded-full text-sm px-5 py-2.5 text-center me-2 mb-2 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800">Save</button>
            </div>
        </div>
    }
}
