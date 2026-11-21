#![recursion_limit = "512"]

pub mod app;
pub mod launcher;
pub mod micro_apps;
pub mod platform;
pub mod shared_ui;
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
