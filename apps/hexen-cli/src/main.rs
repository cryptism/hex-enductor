//! `hexen` — the project maintenance commands that used to be bun
//! scripts under scripts/. Run from the repo root:
//! `cargo run -p hexen-cli -- <command> --help`.

mod icons;
mod launch;
mod migrate;
mod obsidian;
mod schema;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hexen", about = "Hex Enductor project maintenance")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Turn an Obsidian vault using the map-root/map-x/map-y frontmatter
    /// convention into a .hexen.yml project (step 1 of the Obsidian
    /// migration). Copies each map's _maps/<id>/map.png into the assets dir.
    ImportObsidian {
        #[arg(long)]
        vault: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value = "Imported project")]
        title: String,
        /// Where map images are copied, relative to --out's directory.
        #[arg(long, default_value = "_assets")]
        assets_dir: String,
    },
    /// Remove the map-*/map-hex-* frontmatter that import-obsidian moved into
    /// the .hexen.yml from the vault notes it came from (step 2). Leaves
    /// every other line of every note untouched. Run only once the generated
    /// project is confirmed correct.
    StripObsidianFrontmatter {
        #[arg(long)]
        vault: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Convert a .hexen.yml written in the old "type:"-discriminated format
    /// (written by the former TypeScript tools) to the current one, in place. Files already
    /// in the current format are left as they are.
    Migrate {
        project: PathBuf,
        /// Print the result instead of writing it.
        #[arg(long)]
        dry_run: bool,
    },
    /// Regenerate crates/map-core/src/link_icons.rs from game-icons.net's
    /// source SVGs (CC BY 3.0). To add an icon, add it to ICON_SOURCES in
    /// apps/hexen-cli/src/icons.rs and run this.
    SyncLinkIcons {
        #[arg(long, default_value = "crates/map-core/src/link_icons.rs")]
        out: PathBuf,
    },
    /// Start hexend, the editor and the presentation view, and open a browser
    /// tab for each already pointed at PROJECT. Ctrl+C stops everything.
    Launch {
        project: PathBuf,
        #[arg(long, default_value_t = 4000)]
        hexend_port: u16,
        #[arg(long, default_value_t = 5173)]
        editor_port: u16,
        #[arg(long, default_value_t = 5174)]
        presentation_port: u16,
        /// Just print the URLs.
        #[arg(long)]
        no_browser: bool,
    },
    /// Regenerate docs/hexen.schema.json — a JSON Schema for .hexen.yml,
    /// derived from schema/hexen/v1/*.proto — for editor completion via
    /// `# yaml-language-server: $schema=...`.
    Schema {
        #[arg(long, default_value = "docs/hexen.schema.json")]
        out: PathBuf,
    },
}

fn run(command: Command) -> Result<(), String> {
    match command {
        Command::ImportObsidian {
            vault,
            out,
            title,
            assets_dir,
        } => obsidian::import(&vault, &out, &title, &assets_dir),
        Command::StripObsidianFrontmatter { vault, dry_run } => obsidian::strip(&vault, dry_run),
        Command::Migrate { project, dry_run } => migrate::run(&project, dry_run),
        Command::SyncLinkIcons { out } => icons::sync(&out),
        Command::Schema { out } => schema::write(&out),
        Command::Launch {
            project,
            hexend_port,
            editor_port,
            presentation_port,
            no_browser,
        } => launch::run(
            &project,
            launch::Ports {
                hexend: hexend_port,
                editor: editor_port,
                presentation: presentation_port,
            },
            !no_browser,
        ),
    }
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
