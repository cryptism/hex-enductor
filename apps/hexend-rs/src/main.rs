use hexend_rs::{server, session};

const BANNER: &str = r#"
  ██║  ██║███████╗██╗  ██╗███████╗███╗   ██╗██████╗
  ██╠══██║██╠════╝╚██╗██╔╝██╠════╝████╗  ██║██╠══██╗
  ███████║█████╗   ╚███╔╝ █████╗  ██╔██╗ ██║██║  ██║
  ██╠══██║██╠══╝   ██╔██╗ ██╠══╝  ██║╚██╗██║██║  ██║
  ██║  ██║███████╗██╔╝ ██╗███████╗██║ ╚████║██████╔╝
  ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═══╝╚═════╝
"#;

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(4001);

    println!("{BANNER}");
    println!("  it has opened an eye at http://localhost:{port}, and it does not blink.");
    println!("  feed it a .hexen.yml, or feed it nothing — it will wait either way.\n");

    let app = server::app(session::new_sessions());
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.expect("bind port");
    axum::serve(listener, app).await.expect("server error");
}
