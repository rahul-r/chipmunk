use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use leptos_use::{
    core::ConnectionReadyState, use_websocket_with_options, UseWebSocketOptions, UseWebsocketReturn,
};

mod components;
mod pages;

use crate::pages::geofence::Geofence;
use crate::pages::home::Home;
use crate::pages::not_found::NotFound;
use crate::pages::settings::Settings;

use leptos_leaflet::Position;
use std::rc::Rc;
use ui_common::{units::TemperatureUnit, Status, Topic, WsMessage};
#[derive(Clone)]
pub struct WebsocketContext {
    pub message: Signal<Option<String>>,
    send: Rc<dyn Fn(&str)>, // use Rc to make it easily cloneable
    ready_state: Signal<ConnectionReadyState>,
    logging_status: ReadSignal<Status>,
    is_logging: ReadSignal<bool>,
    location: ReadSignal<Position>,
    temperature_unit: ReadSignal<TemperatureUnit>,
}

impl WebsocketContext {
    pub fn new(
        message: Signal<Option<String>>,
        send: Rc<dyn Fn(&str)>,
        ready_state: Signal<ConnectionReadyState>,
        logging_status: ReadSignal<Status>,
        is_logging: ReadSignal<bool>,
        location: ReadSignal<Position>,
        temperature_unit: ReadSignal<TemperatureUnit>,
    ) -> Self {
        Self {
            message,
            send,
            ready_state,
            logging_status,
            is_logging,
            location,
            temperature_unit,
        }
    }

    // create a method to avoid having to use parentheses around the field
    #[inline(always)]
    pub fn send(&self, message: &str) {
        (self.send)(message)
    }
}

pub fn get_host() -> anyhow::Result<String> {
    let Some(window) = web_sys::window() else {
        anyhow::bail!("Cannot get window");
    };

    let Ok(hostname) = window.location().hostname() else {
        anyhow::bail!("Cannot get hostname");
    };

    let Ok(port) = window.location().port() else {
        anyhow::bail!("Cannot get port");
    };

    let Ok(protocol) = window.location().protocol() else {
        anyhow::bail!("Cannot get protocol");
    };

    let ws_protocol = if protocol == "http:" {
        "ws:"
    } else if protocol == "https:" {
        "wss:"
    } else {
        anyhow::bail!("Unknown protocol {protocol}");
    };

    Ok(format!("{ws_protocol}//{hostname}:{port}"))
}

#[component]
fn Navbar() -> impl IntoView {
    let (show_menu, set_show_menu) = create_signal(false);
    let menu = move || {
        view! {
            <ul class="flex flex-col z-21 sm:p-0 font-medium border border-border rounded-lg bg-bkg-1 sm:space-x-8 rtl:space-x-reverse sm:flex-row sm:mt-0 sm:border-0 sm:bg-bkg-1">
                <li>
                    <a href="/" class="block py-2 px-3 text-content-1 bg-blue-700 rounded sm:bg-transparent sm:text-content-1 sm:p-0" aria-current="page">Home</a>
                </li>
                <li>
                    <a href="#" class="block py-2 px-3 text-content-1 rounded hover:bg-gray-100 sm:hover:bg-transparent sm:hover:text-blue-700 sm:p-0">Dashboards</a>
                </li>
                <li>
                    <a href="/geofence" class="block py-2 px-3 text-content-1 rounded hover:bg-gray-100 sm:hover:bg-transparent sm:hover:text-blue-700 sm:p-0 ">Geo-Fence</a>
                </li>
                <li>
                    <a href="/settings" class="block py-2 px-3 text-content-1 rounded hover:bg-gray-100 sm:hover:bg-transparent sm:hover:text-blue-700 sm:p-0 ">Settings</a>
                </li>
            </ul>
        }
    };

    view! {
        <nav class="bg-bkg-1 fixed w-full z-20 top-0 start-0 border-b border-bkg-2">
            <div class="max-w-screen-xl flex flex-wrap items-center justify-between mx-auto p-4">
                <a href="/" class="flex">
                    <img src="/public/logo.svg" class="h-8 pr-1 text-content-1" alt="logo"/>
                    <span class="self-center text-2xl font-semibold whitespace-nowrap text-content-1">Chipmunk</span>
                </a>
                <div class="flex md:order-2 space-x-3 md:space-x-0 rtl:space-x-reverse">
                    <button on:click=move |_| set_show_menu(!show_menu.get()) type="button" class="inline-flex items-center p-2 w-10 h-10 justify-center text-sm text-content-1 rounded-lg sm:hidden hover:bg-gray-100 focus:outline-none focus:ring-2 focus:ring-gray-200" aria-controls="navbar-sticky" aria-expanded="false">
                        <span class="sr-only">Open main menu</span>
                        <svg class="w-5 h-5" aria-hidden="true" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 17 14">
                            <path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M1 1h15M1 7h15M1 13h15"/>
                        </svg>
                    </button>
                </div>
                <Show when=move || show_menu.get() fallback=move || view! {<div class="hidden sm:block">{menu}</div>}>
                    <div class="items-center justify-between w-full md:flex md:w-auto md:order-1" id="navbar-sticky">
                        {menu}
                    </div>
                </Show>
            </div>
        </nav>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let (is_logging, set_is_logging) = create_signal(false);
    let (logging_status, set_logging_status) = create_signal(Status::default());

    let (temperature_unit, set_temperature_unit) = create_signal(TemperatureUnit::default());

    // let (is_dark_mode, set_is_dark_mode) = create_signal(true);

    let (location, set_location) = create_signal(Position::new(0.0, 0.0));
    // let (location, set_location) = create_signal(Position::new(37.49, -121.94));

    let on_message_callback = move |msg: String| match WsMessage::from_string(&*msg) {
        Ok(m) => {
            if let Topic::LoggingStatus = m.topic {
                let status = ui_common::Status::from_value(m.data.unwrap()).unwrap();
                set_is_logging(status.logging.enabled);
                set_logging_status(status.clone());
                set_temperature_unit(status.logging.unit_of_temperature);
                if let Some(l) = status.vehicle.location.coords {
                    set_location(Position::new(l.0 as f64, l.1 as f64))
                }
            }
        }
        Err(e) => logging::log!(
            "Cannot convert websocket message to a known message type: {}",
            e
        ),
    };

    let UseWebsocketReturn {
        ready_state,
        message,
        send,
        ..
    } = use_websocket_with_options(
        &format!("{}/websocket", get_host().unwrap()),
        UseWebSocketOptions::default()
            .immediate(true)
            // .on_open(on_open_callback.clone())
            // .on_close(on_close_callback.clone())
            // .on_error(on_error_callback.clone())
            // .on_message_bytes(on_message_bytes_callback.clone())
            .on_message(on_message_callback),
    );

    provide_context(WebsocketContext::new(
        message,
        Rc::new(send.clone()),
        ready_state,
        logging_status,
        is_logging,
        location,
        temperature_unit,
    ));

    view! {
        <Html lang="en" dir="ltr" class="bg-bkg-2"/>

        <Title text="Chipmunk for Tesla"/>

        <Meta charset="UTF-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>

        <Link rel="apple-touch-icon" href="/public/apple-touch-icon.png" />
        <Meta name="apple-mobile-web-app-title" content="Chipmunk" />
        <Meta name="apple-mobile-web-app-capable" content="yes" />
        <Meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />

        // <div class:light=move || !is_dark_mode.get() class:dark=move || is_dark_mode.get()>
        <div>
            <Navbar/>
            <Router>
                <main class="pt-[4rem]">
                    <Routes>
                        <Route path="/" view=Home/>
                        <Route path="/settings" view=Settings/>
                        <Route path="/geofence" view=Geofence/>
                        <Route path="/*" view=NotFound/>
                    </Routes>
                </main>
            </Router>
        </div>
    }
}
