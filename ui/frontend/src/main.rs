#![feature(async_closure)]

mod components;
mod websocket;

use async_channel::Sender;
use stylist::Style;
use ui_common::{Json, LoggingStatus, Topic, WsMessage};
use wasm_bindgen_futures::spawn_local;
use web_sys::MouseEvent;
use yew::{function_component, html, use_effect, use_state, Callback, ContextProvider, Html};

use crate::{
    components::{logging_status::Status, settings::Settings},
    websocket::Ws,
};

const STYLE_SHEET: &str = include_str!("main.css");

#[derive(Debug, Clone)]
struct WsContext {
    msg: WsMessage,
    tx: Sender<WsMessage>,
}

impl PartialEq for WsContext {
    fn eq(&self, other: &Self) -> bool {
        self.msg == other.msg
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
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

#[function_component(App)]
fn ws() -> Html {
    let first_load = use_state(|| true);
    let is_logging = use_state(|| true);

    let ws = use_state(|| Ws::start());

    let ws_context = use_state(|| WsContext {
        msg: WsMessage::default(),
        tx: ws.tx.clone(),
    });
    let ws_context_clone = ws_context.clone();

    let ws_clone = ws.clone();
    use_effect(move || {
        if *first_load {
            spawn_local(async move {
                loop {
                    match ws_clone.rx.recv().await {
                        Ok(m) => {
                            ws_context_clone.set(WsContext {
                                msg: m,
                                tx: ws_clone.tx.clone(),
                            });
                        }
                        Err(e) => {
                            log::error!("{e}");
                            break;
                        }
                    }
                }
            });

            first_load.set(false);
        }
    });

    let logging_onclick = |enable: bool| {
        let ws = ws.clone();
        Callback::from(move |_: MouseEvent| {
            let topic = match enable {
                true => Topic::StartLogging,
                false => Topic::StopLogging,
            };

            let msg = WsMessage::command(topic, None);

            if let Err(e) = futures::executor::block_on(ws.tx.send(msg)) {
                log::error!("{e}");
            }
        })
    };

    let stylesheet = match Style::new(STYLE_SHEET) {
        Ok(v) => v,
        Err(e) => {
            log::error!("{e}");
            Style::new("").expect("Cannot load style")
        }
    };

    let msg = (*ws_context).clone().msg;
    match msg.topic {
        Topic::LoggingStatus => {
            let status = LoggingStatus::from_value(msg.data.unwrap()).unwrap();
            if *is_logging != status.is_logging {
                is_logging.set(status.is_logging);
            }
        }
        _ => (),
    }

    html! {
        <ContextProvider<WsContext> context={(*ws_context).clone()}>
            <div class={stylesheet}>
                if *is_logging {
                    <a href="#" onclick={logging_onclick(!*is_logging)} class="stopButton">
                      <i class="stopButtonLabel">{"||"}</i>
                    </a>
                } else {
                    <a href="#" onclick={logging_onclick(!*is_logging)} class="startButton">
                      <i class="startButtonLabel">{">"}</i>
                    </a>
                }
            </div>
            <Status />
            <Settings />
        </ContextProvider<WsContext>>
    }
}
