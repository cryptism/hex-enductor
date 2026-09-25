mod app;
mod live_session;
mod map_canvas;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
