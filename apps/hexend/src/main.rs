use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use hexend::{server, session};

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
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(4000);
    // Loopback unless asked otherwise: this API has no auth and can open,
    // write and list files anywhere the process can. HOST=0.0.0.0 exposes
    // it to the network — only do that on one you trust.
    let host: IpAddr = std::env::var("HOST").ok().and_then(|h| h.parse().ok()).unwrap_or(Ipv4Addr::LOCALHOST.into());

    println!("{BANNER}");
    println!("  it has opened an eye at http://{}, and it does not blink.", SocketAddr::new(host, port));
    println!("  feed it a .hexen.yml, or feed it nothing — it will wait either way.\n");
    if !host.is_loopback() {
        println!("  (HOST={host}: reachable from other machines, with no authentication.)\n");
    }

    let app = server::app(session::new_sessions());
    let listener = tokio::net::TcpListener::bind((host, port)).await.expect("bind port");
    axum::serve(listener, app).await.expect("server error");
}
