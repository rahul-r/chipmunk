use leptos::*;
use leptos::prelude::*;
use serde_json::json;

use ui_common::{
    units::{DistanceUnit, Measurement, PressureUnit, TemperatureUnit},
    Topic, WsMessage,
};

use crate::WebsocketContext;

#[component]
pub fn Settings() -> impl IntoView {
    let (refresh_token, set_refresh_token) = signal("".to_string());

    let websocket = expect_context::<WebsocketContext>();

    let ws_send = websocket.send.clone();
    let send_refresh_token = move |_| {
        let data = json!({"token": refresh_token.get()});
        let msg = WsMessage::command(Topic::RefreshToken, Some(data));
        logging::log!("{:?}", msg);
        ws_send(&msg.to_string().unwrap());
    };

    let ws_send = websocket.send.clone();
    let send_enable_logging = move |is_checked: bool| {
        let topic = match is_checked {
            true => Topic::StartLogging,
            false => Topic::StopLogging,
        };
        let msg = WsMessage::command(topic, None);
        ws_send(&msg.to_string().unwrap());
    };

    let ws_send = websocket.send.clone();
    let send_unit = move |unit| {
        let data = serde_json::to_value(unit)
            .map_err(|e| log::error!("{e}"))
            .ok();

        match WsMessage::command(Topic::SetUnit, data).to_string() {
            Ok(msg) => ws_send(&msg),
            Err(e) => log::error!("{e}"),
        };
    };
    let send_dist_unit_1 = send_unit.clone();
    let send_dist_unit_2 = send_unit.clone();
    let send_temp_unit_1 = send_unit.clone();
    let send_temp_unit_2 = send_unit.clone();
    let send_pressure_unit_1 = send_unit.clone();
    let send_pressure_unit_2 = send_unit;

    view! {
        <div class="mx-auto max-w-sm pt-8">
            <div class="mb-5">
                <label class="me-5 inline-flex cursor-pointer items-center">
                <input type="checkbox" value="" on:input=move |ev| send_enable_logging(event_target_checked(&ev)) class="peer sr-only" checked />
                <div class="peer relative h-6 w-11 rounded-full bg-gray-200 after:absolute after:start-[2px] after:top-0.5 after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:bg-blue-600 peer-checked:after:translate-x-full peer-checked:after:border-white peer-focus:ring-4 peer-focus:ring-blue-300 rtl:peer-checked:after:-translate-x-full"></div>
                <span class="ms-3 text-sm font-medium text-content-1">Enable logging</span>
                </label>
            </div>
            <div class="mb-5">
                <label for="refresh_token" class="mb-2 block text-sm font-medium text-content-1">Refresh token</label>
                <div class="flex">
                <input type="text" id="refresh_token" on:input=move |ev| set_refresh_token(event_target_value(&ev)) class="block w-full rounded-lg border border-content-2 bg-bkg-2 text-sm text-content-1 focus:border-blue-500 focus:ring-blue-500" />
                <button type="button" on:click=send_refresh_token class="ml-2 rounded-lg bg-blue-700 px-5 py-2.5 text-center text-sm font-medium text-bkg-2 hover:bg-blue-800 focus:outline-none focus:ring-4 focus:ring-blue-300">Save</button>
                </div>
            </div>
            <div class="mb-5">
                <label for="distance_unit" class="mb-2 block text-sm font-medium text-content-1">Distance</label>
                <div class="flex">
                <ul class="grid w-full grid-cols-2 gap-1">
                    <li>
                    <input type="radio" on:input=move |_| send_dist_unit_1(Measurement::Distance(DistanceUnit::Mi)) id="dist-unit-mi" name="dist-unit" value="dist-unit-mi" class="peer hidden" required />
                    <label for="dist-unit-mi" class="inline-flex w-full cursor-pointer items-center justify-between rounded-lg border border-gray-200 bg-bkg-2 p-1 px-3 text-gray-500 hover:bg-gray-100 hover:text-gray-600 peer-checked:border-blue-600 peer-checked:text-blue-600">
                        <div class="block text-center">
                        <div class="w-full text-lg font-semibold">mi</div>
                        </div>
                    </label>
                    </li>
                    <li>
                    <input type="radio" on:input=move |_| send_dist_unit_2(Measurement::Distance(DistanceUnit::Km)) id="dist-unit-km" name="dist-unit" value="dist-unit-km" class="peer hidden" />
                    <label for="dist-unit-km" class="inline-flex w-full cursor-pointer items-center justify-between rounded-lg border border-gray-200 bg-bkg-2 p-1 px-3 text-gray-500 hover:bg-gray-100 hover:text-gray-600 peer-checked:border-blue-600 peer-checked:text-blue-600">
                        <div class="block">
                        <div class="w-full text-lg font-semibold">km</div>
                        </div>
                    </label>
                    </li>
                </ul>
                </div>
            </div>

            <div class="mb-5">
                <label for="temperature_unit" class="mb-2 block text-sm font-medium text-content-1">Temperature</label>
                <div class="flex">
                <ul class="grid w-full grid-cols-2 gap-1">
                    <li>
                    <input type="radio" on:input=move |_| send_temp_unit_1(Measurement::Temperature(TemperatureUnit::C)) id="temp-unit-c" name="temp-unit" value="temp-unit-c" class="peer hidden" required />
                    <label for="temp-unit-c" class="inline-flex w-full cursor-pointer items-center justify-between rounded-lg border border-gray-200 bg-bkg-2 p-1 px-3 text-gray-500 hover:bg-gray-100 hover:text-gray-600 peer-checked:border-blue-600 peer-checked:text-blue-600">
                        <div class="block text-center">
                        <div class="w-full text-lg font-semibold">Celsius</div>
                        </div>
                    </label>
                    </li>
                    <li>
                    <input type="radio" on:input=move |_| send_temp_unit_2(Measurement::Temperature(TemperatureUnit::F)) id="temp-unit-f" name="temp-unit" value="temp-unit-f" class="peer hidden" />
                    <label for="temp-unit-f" class="inline-flex w-full cursor-pointer items-center justify-between rounded-lg border border-gray-200 bg-bkg-2 p-1 px-3 text-gray-500 hover:bg-gray-100 hover:text-gray-600 peer-checked:border-blue-600 peer-checked:text-blue-600">
                        <div class="block">
                        <div class="w-full text-lg font-semibold">Fahrenheit</div>
                        </div>
                    </label>
                    </li>
                </ul>
                </div>
            </div>

            <div class="mb-5">
                <label for="pressure_unit" class="mb-2 block text-sm font-medium text-content-1">Pressure</label>
                <div class="flex">
                <ul class="grid w-full grid-cols-2 gap-1">
                    <li>
                    <input type="radio" on:input=move |_| send_pressure_unit_1(Measurement::Pressure(PressureUnit::Psi)) id="pres-unit-psi" name="pres-unit" value="pres-unit-psi" class="peer hidden" required />
                    <label for="pres-unit-psi" class="inline-flex w-full cursor-pointer items-center justify-between rounded-lg border border-gray-200 bg-bkg-2 p-1 px-3 text-gray-500 hover:bg-gray-100 hover:text-gray-600 peer-checked:border-blue-600 peer-checked:text-blue-600">
                        <div class="block text-center">
                        <div class="w-full text-lg font-semibold">PSI</div>
                        </div>
                    </label>
                    </li>
                    <li>
                    <input type="radio" on:input=move |_| send_pressure_unit_2(Measurement::Pressure(PressureUnit::Bar)) id="pres-unit-bar" name="pres-unit" value="pres-unit-bar" class="peer hidden" />
                    <label for="pres-unit-bar" class="inline-flex w-full cursor-pointer items-center justify-between rounded-lg border border-gray-200 bg-bkg-2 p-1 px-3 text-gray-500 hover:bg-gray-100 hover:text-gray-600 peer-checked:border-blue-600 peer-checked:text-blue-600">
                        <div class="block">
                        <div class="w-full text-lg font-semibold">Bar</div>
                        </div>
                    </label>
                    </li>
                </ul>
                </div>
            </div>
        </div>
    }
}
