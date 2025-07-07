use leptos::server::codee::string::FromToStringCodec;
use leptos::*;
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::*;
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_use::{
    core::ConnectionReadyState, use_websocket_with_options, UseWebSocketOptions, UseWebSocketReturn,
};

mod components;
mod pages;

use crate::pages::geofence::Geofence;
use crate::pages::home::Home;
use crate::pages::settings::Settings;

use leptos_leaflet::prelude::Position;
use std::sync::Arc;
use ui_common::{Status, Topic, WsMessage};

#[derive(Clone)]
pub struct WebsocketContext {
    pub message: Signal<Option<String>>,
    send: Arc<dyn Fn(&String) + Send + Sync>, // use Rc to make it easily cloneable
    ready_state: Signal<ConnectionReadyState>,
    logging_status: ReadSignal<Status>,
    is_logging: ReadSignal<bool>,
    location: ReadSignal<Position>,
}

impl WebsocketContext {
    pub fn new(
        message: Signal<Option<String>>,
        send: Arc<dyn Fn(&String) + Send + Sync>,
        ready_state: Signal<ConnectionReadyState>,
        logging_status: ReadSignal<Status>,
        is_logging: ReadSignal<bool>,
        location: ReadSignal<Position>,
    ) -> Self {
        Self {
            message,
            send,
            ready_state,
            logging_status,
            is_logging,
            location,
        }
    }

    // create a method to avoid having to use parentheses around the field
    #[inline(always)]
    pub fn send(&self, message: &str) {
        (self.send)(&message.to_string())
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
    let (show_menu, set_show_menu) = signal(false);
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

    let (is_logging, set_is_logging) = signal(false);
    let (logging_status, set_logging_status) = signal(Status::default());

    // let (is_dark_mode, set_is_dark_mode) = signal(true);

    let tesla_factory_coords = Position::new(37.49, -121.94);
    let (location, set_location) = signal(tesla_factory_coords);

    let on_message_callback = move |msg: &str| match WsMessage::from_string(&*msg) {
        Ok(m) => {
            if let Topic::LoggingStatus = m.topic {
                let status = ui_common::Status::from_value(m.data.unwrap()).unwrap();
                set_is_logging(status.logging.enabled);
                set_logging_status(status.clone());
                if let Some(l) = status.vehicle.location.coords {
                    set_location(Position::new(l.0, l.1))
                }
            }
        }
        Err(e) => logging::log!(
            "Cannot convert websocket message to a known message type: {}",
            e
        ),
    };

    let UseWebSocketReturn {
        ready_state,
        message,
        send,
        ..
    } = use_websocket_with_options::<String, String, FromToStringCodec, _, _>(
        &format!("{}/websocket", get_host().unwrap()),
        UseWebSocketOptions::default()
            .immediate(true)
            // .on_open(on_open_callback.clone())
            // .on_close(on_close_callback.clone())
            // .on_error(on_error_callback.clone())
            // .on_message_bytes(on_message_bytes_callback.clone())
            .on_message_raw(on_message_callback),
    );

    provide_context(WebsocketContext::new(
        message,
        Arc::new(send.clone()),
        ready_state,
        logging_status,
        is_logging,
        location,
    ));

    view! {
        <Html/>
        <Title text="Chipmunk for Tesla"/>

        <Meta charset="UTF-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>

        <Link rel="apple-touch-icon" href="/public/apple-touch-icon.png" />
        <Meta name="apple-mobile-web-app-title" content="Chipmunk" />
        <Meta name="apple-mobile-web-app-capable" content="yes" />
        <Meta name="apple-mobile-web-app-status-bar-style" content="black-translucent" />

        // <div class:light=move || !is_dark_mode.get() class:dark=move || is_dark_mode.get()>
        <div class="bg-bkg-2">
            <Navbar/>
            <Router>
                <main class="pt-[4rem]">
                    <Routes fallback=|| "Not found." >
                        // <ParentRoute path=path!("") view=Home >
                            <Route path=path!("") view=Home/>
                            <Route path=path!("settings") view=Settings/>
                            <Route path=path!("geofence") view=Geofence/>
                        // </ParentRoute>
                    </Routes>
                </main>
            </Router>
        </div>
    }
}
