use std::time::Duration;

use leptos::*;
use leptos_leaflet::*;

#[component]
pub fn Map(lat: f64, lon: f64) -> impl IntoView {
    let (marker_position, set_marker_position) = create_signal(Position::new(37.49, -121.94));

    create_effect(move |_| {
        set_interval_with_handle(
            move || {
                set_marker_position.update(|pos| {
                    pos.lat = lat;
                    pos.lng = lon;
                });
            },
            Duration::from_millis(200),
        )
        .ok()
    });

    view! {
          <MapContainer style="height: 300px" center=Position::new(37.49, -121.94) zoom=13.0 set_view=true class="z-0">
              <TileLayer url="https://tile.openstreetmap.org/{z}/{x}/{y}.png"/>
              <Marker position=marker_position >
                  <Popup>
                      <strong>{"Car"}</strong>
                  </Popup>
              </Marker>
         </MapContainer>
    }
}
