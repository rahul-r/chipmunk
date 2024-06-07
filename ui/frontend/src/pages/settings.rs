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
        <div>
            <span>{"Refresh token:"}</span>
            <input type="text" on:input=move |ev| set_refresh_token(event_target_value(&ev)) />
            <button on:click=send_refresh_token>"Save"</button>
        </div>
    }
}
