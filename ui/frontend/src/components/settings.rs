use serde_json::json;
use ui_common::{Topic, WsMessage};
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::{Event, HtmlInputElement, InputEvent};
use yew::{function_component, html, use_context, use_state, Callback, Html};

use crate::WsContext;

fn get_value_from_input_event(e: InputEvent) -> String {
    let event: Event = e.dyn_into().unwrap_throw();
    let event_target = event.target().unwrap_throw();
    let target: HtmlInputElement = event_target.dyn_into().unwrap_throw();
    target.value()
}

#[function_component]
pub fn Settings() -> Html {
    let input = use_state(|| "".to_string());

    let ctx = use_context::<WsContext>();
    let Some(ctx) = ctx else {
        log::error!("Cannot get context");
        panic!();
    };

    let input_clone = input.clone();
    let oninput = Callback::from(move |input_event: InputEvent| {
        let i = get_value_from_input_event(input_event);
        input_clone.set(i);
    });

    let input_clone = input.clone();
    let send_refresh_token = Callback::from(move |_| {
        let data = json!({"token": *input_clone});
        let msg = WsMessage::command(Topic::RefreshToken, Some(data));
        if let Err(e) = futures::executor::block_on(ctx.tx.send(msg)) {
            log::error!("{e}");
        }
    });

    html! {
        <>
            <h1>{"Settings"}</h1>
            <p>{"Refresh token:"}</p>
            <input type="text" input={(*input).clone()} {oninput} />
            <button onclick={send_refresh_token}>{"Save"}</button>
        </>
    }
}
