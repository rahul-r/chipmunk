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

use std::rc::Rc;
use ui_common::{Status, Topic, WsMessage};

#[derive(Clone)]
pub struct WebsocketContext {
    pub message: Signal<Option<String>>,
    send: Rc<dyn Fn(&str)>, // use Rc to make it easily cloneable
    ready_state: Signal<ConnectionReadyState>,
    logging_status: ReadSignal<Status>,
    is_logging: ReadSignal<bool>,
}

impl WebsocketContext {
    pub fn new(
        message: Signal<Option<String>>,
        send: Rc<dyn Fn(&str)>,
        ready_state: Signal<ConnectionReadyState>,
        logging_status: ReadSignal<Status>,
        is_logging: ReadSignal<bool>,
    ) -> Self {
        Self {
            message,
            send,
            ready_state,
            logging_status,
            is_logging,
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
    view! {
        <nav class="bg-white dark:bg-gray-900 fixed w-full z-20 top-0 start-0 border-b border-gray-200 dark:border-gray-600 p-3">
          <div class="max-w-screen-xl flex flex-wrap items-center justify-between mx-auto">
              <a href="/" class="flex">
                  <img src="/public/logo.svg" class="h-8" alt="Chipmunk logo"/>
                  <span class="self-center text-2xl font-semibold whitespace-nowrap dark:text-white">Chipmunk</span>
              </a>
              <div class="items-center justify-between hidden w-full md:flex md:w-auto md:order-1" id="navbar-sticky">
                <ul class="flex flex-col md:p-0 font-medium border border-gray-100 rounded-lg bg-gray-50 md:space-x-8 rtl:space-x-reverse md:flex-row md:mt-0 md:border-0 md:bg-white dark:bg-gray-800 md:dark:bg-gray-900 dark:border-gray-700">
                  <li>
                    <a href="/" class="block py-2 px-3 text-white bg-blue-700 rounded md:bg-transparent md:text-blue-700 md:p-0 md:dark:text-blue-500" aria-current="page">Home</a>
                  </li>
                  <li>
                    <a href="#" class="block py-2 px-3 text-gray-900 rounded hover:bg-gray-100 md:hover:bg-transparent md:hover:text-blue-700 md:p-0 md:dark:hover:text-blue-500 dark:text-white dark:hover:bg-gray-700 dark:hover:text-white md:dark:hover:bg-transparent dark:border-gray-700">Dashboards</a>
                  </li>
                  <li>
                    <a href="/geofence" class="block py-2 px-3 text-gray-900 rounded hover:bg-gray-100 md:hover:bg-transparent md:hover:text-blue-700 md:p-0 md:dark:hover:text-blue-500 dark:text-white dark:hover:bg-gray-700 dark:hover:text-white md:dark:hover:bg-transparent dark:border-gray-700">Geo-Fence</a>
                  </li>
                  <li>
                    <a href="/settings" class="block py-2 px-3 text-gray-900 rounded hover:bg-gray-100 md:hover:bg-transparent md:hover:text-blue-700 md:p-0 md:dark:hover:text-blue-500 dark:text-white dark:hover:bg-gray-700 dark:hover:text-white md:dark:hover:bg-transparent dark:border-gray-700">Settings</a>
                  </li>
                </ul>
              </div>
          </div>
        </nav>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let (is_logging, set_is_logging) = create_signal(false);
    let (logging_status, set_logging_status) = create_signal(Status::default());

    let on_message_callback = move |msg: String| match WsMessage::from_string(&*msg) {
        Ok(m) => {
            if let Topic::LoggingStatus = m.topic {
                let status = ui_common::Status::from_value(m.data.unwrap()).unwrap();
                set_is_logging(status.logging.enabled);
                set_logging_status(status);
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
    ));

    view! {
        <Html lang="en" dir="ltr" attr:data-theme="light"/>

        <Title text="Chipmunk for Tesla"/>

        <Meta charset="UTF-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>

        <Navbar/>

        <Router>
            <main class="p-[3.6rem]">
                <Routes>
                    <Route path="/" view=Home/>
                    <Route path="/settings" view=Settings/>
                    <Route path="/geofence" view=Geofence/>
                    <Route path="/*" view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}
