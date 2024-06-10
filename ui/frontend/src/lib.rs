use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use leptos_use::{core::ConnectionReadyState, use_websocket, UseWebsocketReturn};

mod components;
mod pages;

use crate::pages::home::Home;
use crate::pages::not_found::NotFound;
use crate::pages::settings::Settings;

use std::rc::Rc;

#[derive(Clone)]
pub struct WebsocketContext {
    pub message: Signal<Option<String>>,
    send: Rc<dyn Fn(&str)>, // use Rc to make it easily cloneable
    ready_state: Signal<ConnectionReadyState>,
}

impl WebsocketContext {
    pub fn new(
        message: Signal<Option<String>>,
        send: Rc<dyn Fn(&str)>,
        ready_state: Signal<ConnectionReadyState>,
    ) -> Self {
        Self {
            message,
            send,
            ready_state,
        }
    }

    // create a method to avoid having to use parantheses around the field
    #[inline(always)]
    pub fn send(&self, message: &str) {
        (self.send)(message)
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let UseWebsocketReturn {
        ready_state,
        message,
        send,
        ..
    } = use_websocket("ws://localhost:3072/websocket");

    provide_context(WebsocketContext::new(
        message,
        Rc::new(send.clone()),
        ready_state,
    ));

    view! {
        <Html lang="en" dir="ltr" attr:data-theme="light"/>

        <Title text="Chipmunk for Tesla"/>

        <Meta charset="UTF-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>

        <Router>
            <nav class="bg-white dark:bg-gray-900 fixed w-full z-20 top-0 start-0 border-b border-gray-200 dark:border-gray-600">
              <div class="max-w-screen-xl flex flex-wrap items-center justify-between mx-auto p-4">
                  <a href="/" class="flex items-center space-x-3 rtl:space-x-reverse">
                      <img src="/public/logo.svg" class="h-8" alt="Chipmunk logo"/>
                      <span class="self-center text-2xl font-semibold whitespace-nowrap dark:text-white">Chipmunk</span>
                  </a>
                  <div class="items-center justify-between hidden w-full md:flex md:w-auto md:order-1" id="navbar-sticky">
                    <ul class="flex flex-col p-4 md:p-0 mt-4 font-medium border border-gray-100 rounded-lg bg-gray-50 md:space-x-8 rtl:space-x-reverse md:flex-row md:mt-0 md:border-0 md:bg-white dark:bg-gray-800 md:dark:bg-gray-900 dark:border-gray-700">
                      <li>
                        <a href="/" class="block py-2 px-3 text-white bg-blue-700 rounded md:bg-transparent md:text-blue-700 md:p-0 md:dark:text-blue-500" aria-current="page">Home</a>
                      </li>
                      <li>
                        <a href="#" class="block py-2 px-3 text-gray-900 rounded hover:bg-gray-100 md:hover:bg-transparent md:hover:text-blue-700 md:p-0 md:dark:hover:text-blue-500 dark:text-white dark:hover:bg-gray-700 dark:hover:text-white md:dark:hover:bg-transparent dark:border-gray-700">Dashboards</a>
                      </li>
                      <li>
                        <a href="#" class="block py-2 px-3 text-gray-900 rounded hover:bg-gray-100 md:hover:bg-transparent md:hover:text-blue-700 md:p-0 md:dark:hover:text-blue-500 dark:text-white dark:hover:bg-gray-700 dark:hover:text-white md:dark:hover:bg-transparent dark:border-gray-700">Geo-Fence</a>
                      </li>
                      <li>
                        <a href="/settings" class="block py-2 px-3 text-gray-900 rounded hover:bg-gray-100 md:hover:bg-transparent md:hover:text-blue-700 md:p-0 md:dark:hover:text-blue-500 dark:text-white dark:hover:bg-gray-700 dark:hover:text-white md:dark:hover:bg-transparent dark:border-gray-700">Settings</a>
                      </li>
                    </ul>
                  </div>
              </div>
            </nav>
            <main class="pt-20">
                <Routes>
                    <Route path="/" view=Home/>
                    <Route path="/settings" view=Settings/>
                    <Route path="/*" view=NotFound/>
                </Routes>
            </main>
        </Router>
    }
}
