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
            <Routes>
                <Route path="/" view=Home/>
                <Route path="/settings" view=Settings/>
                <Route path="/*" view=NotFound/>
            </Routes>
        </Router>
    }
}
