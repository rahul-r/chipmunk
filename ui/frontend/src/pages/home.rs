use leptos::*;

use leptos_use::core::ConnectionReadyState;
use ui_common::{Status, Topic, WsMessage};

use crate::{components::map::Map, WebsocketContext};

fn log_stat(status: Status) -> impl IntoView {
    view! {
        <div class="flex flex-row w-full bg-bkg md:border md:border-border md:rounded-lg shadow md:max-w-md">
            <div class="flex items-center">
                <img class="object-scale-down h-auto max-h-96 md:h-auto md:max-h-96" src="/public/model3-red.jpeg" alt="car"/>
            </div>
            <div class="flex flex-col p-5 leading-normal text-center w-full">
                <div class="pb-4 text-center">
                    <div class="flex items-center justify-center">
                        <p class="pr-2 text-2xl font-bold text-content-1">Title</p>
                        <svg class="fill-yellow-200" viewBox="0 0 8 8" height="25px" width="25px" xmlns="http://www.w3.org/2000/svg" class="size-5">
                            <path d="M3 0c-1.1 0-2 .9-2 2h1c0-.56.44-1 1-1s1 .44 1 1v2h-4v4h6v-4h-1v-2c0-1.1-.9-2-2-2z" />
                        </svg>
                        <svg class="currentColor" viewBox="0 0 8 8" height="25px" width="25px" xmlns="http://www.w3.org/2000/svg" class="size-5">
                            <path d="M 4,0 C 2.9,0 2,1.0277807 2,2.2839571 V 3.4259357 H 1 V 7.9938499 H 7 V 3.4259357 H 6 V 2.2839571 C 6,1.0277807 5.1,0 4,0 Z m 0,1.1419786 c 0.56,0 1,0.5024705 1,1.1419785 V 3.4259357 H 3 V 2.2839571 C 3,1.6444491 3.44,1.1419786 4,1.1419786 Z" />
                        </svg>
                    </div>
                    <p class="font-thin text-content-2">Cybertruck</p>
                    <p class="font-thin text-content-2">Driving for 10 minutes</p>
                </div>
                <div class="flex justify-evenly pb-4 text-center">
                    <div class="pb-2 text-center">
                        <p class="font-normal text-content-2">Battery</p>
                        <p class="text-xl font-normal text-content-1">35%</p>
                    </div>
                    <div class="pb-2 text-center">
                        <p class="font-normal text-content-2">Range</p>
                        <p class="text-xl font-normal text-content-1">150mi</p>
                    </div>
                    <div class="pb-2 text-center">
                        <p class="font-normal text-content-2">Odometer</p>
                        <p class="text-xl font-normal text-content-1">65465mi</p>
                    </div>
                    </div>
                    <div class="flex justify-evenly text-center">
                    <div class="pb-2 text-center">
                        <p class="font-normal text-content-2">Interior</p>
                        <p class="text-xl font-normal text-content-1">"65°F"</p>
                    </div>
                    <div class="pb-2 text-center">
                        <p class="font-normal text-content-2">Exterior</p>
                        <p class="text-xl font-normal text-content-1">"67°F"</p>
                    </div>
                    <div class="flex flex-col items-center pb-2 text-center">
                        <p class="font-normal text-content-2">Climate</p>
                        <svg fill="red" class="fill-content-1" viewBox="0 0 24 24" height="25px" width="25px" xmlns="http://www.w3.org/2000/svg" class="size-6">
                            <path d="M19 9c.667 1.06 1 2.394 1 4 0 3-3.5 4-5 9-.667-.575-1-1.408-1-2.5 0-3.482 5-5.29 5-10.5zm-4.5-4a8.31 8.31 0 0 1 1 4c0 5-6 6-4 13C9.833 20.84 9 19.173 9 17c0-3.325 5.5-6 5.5-12zM10 1c.667 1.333 1 2.833 1 4.5 0 6-9 7.5-3 16.5-2.5-.5-4.5-3-4.5-6C3.5 9.5 10 8.5 10 1z" />
                        </svg>
                        <svg fill="blue" viewBox="0 0 512 512" height="25px" width="25px" xmlns="http://www.w3.org/2000/svg">
                            <path
                                d="M422.804,332.819c-34.87-7.132-11.07-25.884,15.846-33.085c26.899-7.201,16.591-45.641-11.708-35.981
                            c-28.308,9.634-56.711,41.967-71.333,33.514c-11.69-6.746-31.178-17.982-38.502-22.217c1.881-6.02,2.888-12.425,2.888-19.049
                            c0-6.624-1.006-13.029-2.888-19.041c7.324-4.226,26.811-15.48,38.502-22.226c14.622-8.435,43.025,23.888,71.333,33.531
                            c28.298,9.643,38.606-28.798,11.708-35.999c-26.916-7.202-50.717-25.936-15.846-33.086c34.861-7.114,66.187-31.65,54.899-51.18
                            c-11.288-19.531-48.17-4.673-71.797,21.955c-23.582,26.618-27.913-3.369-20.712-30.267c7.202-26.908-31.212-37.189-37.014-7.858
                            c-5.819,29.332,7.98,70.116-6.633,78.543c-11.717,6.764-31.212,18.018-38.528,22.244c-8.637-9.38-20.056-16.145-32.954-19.05
                            c0-8.435,0-30.959,0-44.469c0-16.871,42.186-25.315,64.709-45.004c22.497-19.688-5.626-47.828-25.332-28.141
                            c-19.697,19.706-47.812,30.95-36.559-2.817C284.128,39.385,278.554,0,255.987,0c-22.55,0-28.132,39.385-16.88,73.135
                            c11.253,33.767-16.862,22.523-36.55,2.817c-19.706-19.688-47.83,8.453-25.332,28.141c22.515,19.689,64.708,28.133,64.708,45.004
                            c0,13.51,0,36.034,0,44.469c-12.898,2.905-24.326,9.669-32.954,19.05c-7.315-4.226-26.811-15.48-38.528-22.244
                            c-14.613-8.426-0.84-49.211-6.632-78.543c-5.802-29.331-44.225-19.05-37.014,7.858c7.193,26.898,2.896,56.886-20.712,30.267
                            C82.468,123.327,45.585,108.469,34.297,128c-11.288,19.531,20.038,44.067,54.899,51.18c34.853,7.15,11.052,25.884-15.855,33.086
                            c-26.881,7.201-16.591,45.642,11.708,35.999c28.308-9.643,56.72-41.966,71.333-33.531c11.7,6.746,31.186,18,38.493,22.226
                            c-1.873,6.012-2.87,12.416-2.87,19.041c0,6.624,0.997,13.029,2.87,19.049c-7.306,4.236-26.793,15.471-38.493,22.217
                            c-14.613,8.453-43.026-23.879-71.333-33.514c-28.299-9.66-38.589,28.78-11.708,35.981c26.907,7.202,50.725,25.954,15.855,33.085
                            c-34.861,7.115-66.188,31.65-54.899,51.181c11.288,19.54,48.171,4.673,71.797-21.955c23.608-26.618,27.904,3.369,20.712,30.268
                            c-7.21,26.907,31.213,37.188,37.014,7.858c5.792-29.323-7.981-70.091,6.632-78.543c11.717-6.764,31.213-18.018,38.528-22.235
                            c8.628,9.38,20.056,16.136,32.954,19.041c0,8.435,0,30.959,0,44.469c0,16.87-42.194,25.315-64.708,45.003
                            c-22.498,19.689,5.626,47.83,25.332,28.141c19.688-19.706,47.803-30.95,36.55,2.818c-11.253,33.758-5.67,73.135,16.88,73.135
                            c22.567,0,28.141-39.377,16.897-73.135c-11.253-33.768,16.862-22.524,36.559-2.818c19.706,19.688,47.829-8.452,25.332-28.141
                            c-22.523-19.688-64.709-28.133-64.709-45.003c0-13.51,0-36.034,0-44.469c12.898-2.905,24.317-9.66,32.954-19.041
                            c7.315,4.218,26.811,15.471,38.528,22.235c14.613,8.452,0.814,49.22,6.633,78.543c5.802,29.331,44.215,19.05,37.014-7.858
                            c-7.201-26.899-2.896-56.886,20.712-30.268c23.627,26.628,60.509,41.494,71.797,21.955
                            C488.991,364.469,457.665,339.934,422.804,332.819z M255.987,292.27c-20.012,0-36.253-16.232-36.253-36.27
                            c0-20.03,16.241-36.262,36.253-36.262c20.038,0,36.27,16.232,36.27,36.262C292.257,276.038,276.025,292.27,255.987,292.27z"
                            />
                        </svg>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn DriveDetails() -> impl IntoView {
    view! {
        <div class="block max-w-sm rounded-lg border border-border bg-bkg p-6 shadow">
            <div class="pb-4 text-center">
                <div class="flex items-center justify-center">
                    <p class="pr-2 text-2xl font-bold text-content-1">Title</p>
                    <svg class="fill-yellow-200" viewBox="0 0 8 8" height="25px" width="25px" xmlns="http://www.w3.org/2000/svg" class="size-5">
                        <path d="M3 0c-1.1 0-2 .9-2 2h1c0-.56.44-1 1-1s1 .44 1 1v2h-4v4h6v-4h-1v-2c0-1.1-.9-2-2-2z" />
                    </svg>
                    <svg class="fill-current" viewBox="0 0 8 8" height="25px" width="25px" xmlns="http://www.w3.org/2000/svg" class="size-5">
                        <path d="M 4,0 C 2.9,0 2,1.0277807 2,2.2839571 V 3.4259357 H 1 V 7.9938499 H 7 V 3.4259357 H 6 V 2.2839571 C 6,1.0277807 5.1,0 4,0 Z m 0,1.1419786 c 0.56,0 1,0.5024705 1,1.1419785 V 3.4259357 H 3 V 2.2839571 C 3,1.6444491 3.44,1.1419786 4,1.1419786 Z" />
                    </svg>
                </div>
                <p class="font-thin text-content-2">Cybertruck</p>
                <p class="font-thin text-content-2">Driving for 10 minutes</p>
            </div>
            <div class="flex justify-evenly pb-4 text-center">
                <div class="pb-2 text-center">
                    <p class="font-normal text-content-2">Battery</p>
                    <p class="text-xl font-normal text-content-1">35%</p>
                </div>
                <div class="pb-2 text-center">
                    <p class="font-normal text-content-2">Range</p>
                    <p class="text-xl font-normal text-content-1">150mi</p>
                </div>
                <div class="pb-2 text-center">
                    <p class="font-normal text-content-2">Odometer</p>
                    <p class="text-xl font-normal text-content-1">65465mi</p>
                </div>
                </div>
                <div class="flex justify-evenly text-center">
                <div class="pb-2 text-center">
                    <p class="font-normal text-content-2">Interior</p>
                    <p class="text-xl font-normal text-content-1">"65°F"</p>
                </div>
                <div class="pb-2 text-center">
                    <p class="font-normal text-content-2">Exterior</p>
                    <p class="text-xl font-normal text-content-1">"67°F"</p>
                </div>
                <div class="flex flex-col items-center pb-2 text-center">
                    <p class="font-normal text-content-2">Climate</p>
                    <svg fill="red" class="fill-content-1" viewBox="0 0 24 24" height="25px" width="25px" xmlns="http://www.w3.org/2000/svg" class="size-6">
                        <path d="M19 9c.667 1.06 1 2.394 1 4 0 3-3.5 4-5 9-.667-.575-1-1.408-1-2.5 0-3.482 5-5.29 5-10.5zm-4.5-4a8.31 8.31 0 0 1 1 4c0 5-6 6-4 13C9.833 20.84 9 19.173 9 17c0-3.325 5.5-6 5.5-12zM10 1c.667 1.333 1 2.833 1 4.5 0 6-9 7.5-3 16.5-2.5-.5-4.5-3-4.5-6C3.5 9.5 10 8.5 10 1z" />
                    </svg>
                    <svg fill="blue" viewBox="0 0 512 512" height="25px" width="25px" xmlns="http://www.w3.org/2000/svg">
                        <path
                            d="M422.804,332.819c-34.87-7.132-11.07-25.884,15.846-33.085c26.899-7.201,16.591-45.641-11.708-35.981
                        c-28.308,9.634-56.711,41.967-71.333,33.514c-11.69-6.746-31.178-17.982-38.502-22.217c1.881-6.02,2.888-12.425,2.888-19.049
                        c0-6.624-1.006-13.029-2.888-19.041c7.324-4.226,26.811-15.48,38.502-22.226c14.622-8.435,43.025,23.888,71.333,33.531
                        c28.298,9.643,38.606-28.798,11.708-35.999c-26.916-7.202-50.717-25.936-15.846-33.086c34.861-7.114,66.187-31.65,54.899-51.18
                        c-11.288-19.531-48.17-4.673-71.797,21.955c-23.582,26.618-27.913-3.369-20.712-30.267c7.202-26.908-31.212-37.189-37.014-7.858
                        c-5.819,29.332,7.98,70.116-6.633,78.543c-11.717,6.764-31.212,18.018-38.528,22.244c-8.637-9.38-20.056-16.145-32.954-19.05
                        c0-8.435,0-30.959,0-44.469c0-16.871,42.186-25.315,64.709-45.004c22.497-19.688-5.626-47.828-25.332-28.141
                        c-19.697,19.706-47.812,30.95-36.559-2.817C284.128,39.385,278.554,0,255.987,0c-22.55,0-28.132,39.385-16.88,73.135
                        c11.253,33.767-16.862,22.523-36.55,2.817c-19.706-19.688-47.83,8.453-25.332,28.141c22.515,19.689,64.708,28.133,64.708,45.004
                        c0,13.51,0,36.034,0,44.469c-12.898,2.905-24.326,9.669-32.954,19.05c-7.315-4.226-26.811-15.48-38.528-22.244
                        c-14.613-8.426-0.84-49.211-6.632-78.543c-5.802-29.331-44.225-19.05-37.014,7.858c7.193,26.898,2.896,56.886-20.712,30.267
                        C82.468,123.327,45.585,108.469,34.297,128c-11.288,19.531,20.038,44.067,54.899,51.18c34.853,7.15,11.052,25.884-15.855,33.086
                        c-26.881,7.201-16.591,45.642,11.708,35.999c28.308-9.643,56.72-41.966,71.333-33.531c11.7,6.746,31.186,18,38.493,22.226
                        c-1.873,6.012-2.87,12.416-2.87,19.041c0,6.624,0.997,13.029,2.87,19.049c-7.306,4.236-26.793,15.471-38.493,22.217
                        c-14.613,8.453-43.026-23.879-71.333-33.514c-28.299-9.66-38.589,28.78-11.708,35.981c26.907,7.202,50.725,25.954,15.855,33.085
                        c-34.861,7.115-66.188,31.65-54.899,51.181c11.288,19.54,48.171,4.673,71.797-21.955c23.608-26.618,27.904,3.369,20.712,30.268
                        c-7.21,26.907,31.213,37.188,37.014,7.858c5.792-29.323-7.981-70.091,6.632-78.543c11.717-6.764,31.213-18.018,38.528-22.235
                        c8.628,9.38,20.056,16.136,32.954,19.041c0,8.435,0,30.959,0,44.469c0,16.87-42.194,25.315-64.708,45.003
                        c-22.498,19.689,5.626,47.83,25.332,28.141c19.688-19.706,47.803-30.95,36.55,2.818c-11.253,33.758-5.67,73.135,16.88,73.135
                        c22.567,0,28.141-39.377,16.897-73.135c-11.253-33.768,16.862-22.524,36.559-2.818c19.706,19.688,47.829-8.452,25.332-28.141
                        c-22.523-19.688-64.709-28.133-64.709-45.003c0-13.51,0-36.034,0-44.469c12.898-2.905,24.317-9.66,32.954-19.041
                        c7.315,4.218,26.811,15.471,38.528,22.235c14.613,8.452,0.814,49.22,6.633,78.543c5.802,29.331,44.215,19.05,37.014-7.858
                        c-7.201-26.899-2.896-56.886,20.712-30.268c23.627,26.628,60.509,41.494,71.797,21.955
                        C488.991,364.469,457.665,339.934,422.804,332.819z M255.987,292.27c-20.012,0-36.253-16.232-36.253-36.27
                        c0-20.03,16.241-36.262,36.253-36.262c20.038,0,36.27,16.232,36.27,36.262C292.257,276.038,276.025,292.27,255.987,292.27z"
                        />
                    </svg>
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
            <button on:click=move |_| logging_status(!websocket.is_logging.get()) disabled=move || !connected() class="text-white bg-blue-700 hover:bg-blue-800 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 me-2 mb-2 dark:bg-blue-600 dark:hover:bg-blue-700 focus:outline-none dark:focus:ring-blue-800">
                {move || if websocket.is_logging.get() { "Stop" } else { "Start"}}
            </button>
            <div class="flex justify-center items-center md:pt-5">
              {move || log_stat(websocket.logging_status.get())}
            </div>
            <div class="my-dark">
                <DriveDetails />
            </div>
            <div class="my-light">
                <DriveDetails />
            </div>
        </>
    }
}
