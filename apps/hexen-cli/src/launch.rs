//! `hexen launch` — one command for the local dev loop (port of master's
//! scripts/launch.ts): starts hexend and trunk for the editor and the
//! presentation view, waits for each to come up, then opens a browser
//! tab for each already pointed at the project. When any one of them
//! exits, the rest are stopped too. Ctrl+C reaches them all directly
//! (they share the terminal's process group).

use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::{Duration, Instant};

pub struct Ports {
    pub hexend: u16,
    pub editor: u16,
    pub presentation: u16,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Everything launch started, stopped when this is dropped.
struct Services(Vec<(&'static str, Child)>);

impl Drop for Services {
    fn drop(&mut self) {
        for (_, child) in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Services {
    fn spawn(&mut self, name: &'static str, command: &mut Command) -> Result<(), String> {
        let child = command
            .spawn()
            .map_err(|e| format!("couldn't start {name}: {e}"))?;
        self.0.push((name, child));
        Ok(())
    }

    /// Waits until something accepts connections on `port`, or fails if
    /// the service exits first or `timeout` passes. (A first trunk build
    /// compiles the whole app to wasm, so its timeout is long.)
    fn wait_for(&mut self, name: &str, port: u16, timeout: Duration) -> Result<(), String> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
                return Ok(());
            }
            if let Some(exited) = self.first_exited() {
                return Err(format!("{exited} exited before {name} came up"));
            }
            sleep(Duration::from_millis(300));
        }
        Err(format!("timed out waiting for {name} on port {port}"))
    }

    fn first_exited(&mut self) -> Option<&'static str> {
        self.0
            .iter_mut()
            .find_map(|(name, child)| matches!(child.try_wait(), Ok(Some(_))).then_some(*name))
    }
}

fn open_browser(url: &str) {
    let mut command = if cfg!(target_os = "macos") {
        Command::new("open")
    } else if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/c", "start", ""]);
        c
    } else {
        Command::new("xdg-open")
    };
    if command.arg(url).spawn().is_err() {
        eprintln!("Couldn't open a browser automatically — open this yourself:\n  {url}");
    }
}

fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The editor and presentation URLs for a project on a hexend.
pub fn app_urls(project: &Path, ports: &Ports) -> (String, String) {
    let server = format!("http://localhost:{}", ports.hexend);
    let query = format!(
        "?server={}&path={}",
        encode(&server),
        encode(&project.to_string_lossy())
    );
    (
        format!("http://localhost:{}/{query}", ports.editor),
        format!("http://localhost:{}/{query}", ports.presentation),
    )
}

pub fn run(project: &Path, ports: Ports, open: bool) -> Result<(), String> {
    let project = std::path::absolute(project).map_err(|e| e.to_string())?;
    if !project.is_file() {
        return Err(format!("No such file: {}", project.display()));
    }
    let root = repo_root();
    let mut services = Services(Vec::new());

    println!("Starting hexend on :{}…", ports.hexend);
    services.spawn(
        "hexend",
        Command::new("cargo")
            .args(["run", "-p", "hexend"])
            .current_dir(&root)
            .env("PORT", ports.hexend.to_string()),
    )?;
    services.wait_for("hexend", ports.hexend, Duration::from_secs(300))?;

    for (name, dir, port) in [
        ("the editor", "apps/editor", ports.editor),
        ("presentation", "apps/presentation", ports.presentation),
    ] {
        println!("Starting {name} on :{port}…");
        services.spawn(
            if dir == "apps/editor" {
                "editor"
            } else {
                "presentation"
            },
            Command::new("trunk")
                .args(["serve", "--port", &port.to_string()])
                .current_dir(root.join(dir)),
        )?;
        services.wait_for(name, port, Duration::from_secs(600))?;
    }

    let (editor, presentation) = app_urls(&project, &ports);
    println!("\nEverything's up:\n  editor:       {editor}\n  presentation: {presentation}");
    if open {
        open_browser(&editor);
        open_browser(&presentation);
    }

    println!("\nPress Ctrl+C to stop everything.");
    loop {
        if let Some(name) = services.first_exited() {
            // A crash or a port fight — bring the rest down rather than leave orphans.
            return Err(format!("{name} exited; stopping the rest"));
        }
        sleep(Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_carry_the_server_and_the_absolute_project_path() {
        let ports = Ports {
            hexend: 4001,
            editor: 5173,
            presentation: 5174,
        };
        let (editor, presentation) = app_urls(Path::new("/maps/my realm.hexen.yml"), &ports);
        assert_eq!(
            editor,
            "http://localhost:5173/?server=http%3A%2F%2Flocalhost%3A4001&path=%2Fmaps%2Fmy%20realm.hexen.yml"
        );
        assert!(presentation.starts_with("http://localhost:5174/?server="));
    }
}
