use ui_common::Topic;
use yew::{function_component, html, use_context, Html};

use crate::WsContext;

#[function_component]
pub fn Status() -> Html {
    let ctx = use_context::<WsContext>();
    if let Some(c) = ctx {
        match c.msg.topic {
            Topic::LoggingStatus => {
                let status = ui_common::Status::from_value(c.msg.data.unwrap()).unwrap();
                return html! {
                    <>
                        <h1>{"Logger Status"}</h1>
                        <div>{"charge_state: "}{status.charging.map(|c| c.charge_added)}</div>
                        <div>{"current_points: "}{status.logging.current_num_points}</div>
                        <div>{"is_logging: "}{status.logging.enabled}</div>
                        <div>{"is_user_present: "}{status.vehicle.is_user_nearby}</div>
                        <div>{"odometer: "}{status.vehicle.odometer}</div>
                        <div>{"total_points: "}{status.logging.total_num_points}</div>
                    </>
                };
            }
            _ => (),
        }
    }

    html! { <h1>{"Error"}</h1> }
}
