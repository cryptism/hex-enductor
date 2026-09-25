mod about;
mod app;
mod desktop;
mod editor;
mod fog_controls;
mod forms;
mod http;
mod location_browser;
mod logo;
mod picker;
mod recents;
mod server_panel;
mod storage;
mod ui_state;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
