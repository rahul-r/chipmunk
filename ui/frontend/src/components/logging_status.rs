use ui_common::{Json, LoggingStatus, Topic};
use yew::{function_component, html, use_context, Html};

use crate::WsContext;

#[function_component]
pub fn Status() -> Html {
    let ctx = use_context::<WsContext>();
    if let Some(c) = ctx {
        match c.msg.topic {
            Topic::LoggingStatus => {
                let status = LoggingStatus::from_value(c.msg.data.unwrap()).unwrap();
                return html! {
                    <>
                        <h1>{"Logger Status"}</h1>
                        <div>{"charge_state: "}{status.charge_state}</div>
                        <div>{"charging_status: "}{status.charging_status}</div>
                        <div>{"current_miles: "}{status.current_miles}</div>
                        <div>{"current_points: "}{status.current_points}</div>
                        <div>{"is_logging: "}{status.is_logging}</div>
                        <div>{"is_user_present: "}{status.is_user_present}</div>
                        <div>{"odometer: "}{status.odometer}</div>
                        <div>{"total_miles: "}{status.total_miles}</div>
                        <div>{"total_points: "}{status.total_points}</div>
                    </>
                };
            }
            _ => (),
        }
    }

    html! { <h1>{"Error"}</h1> }
}
