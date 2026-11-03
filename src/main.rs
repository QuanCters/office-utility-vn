pub mod app;
pub mod favorite_tools;
pub mod home_view;
pub mod micro_apps;
pub mod recent_tools;
pub mod shared_ui;
pub mod sidebar;
pub mod tools;

use app::*;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <App/>
        }
    })
}
