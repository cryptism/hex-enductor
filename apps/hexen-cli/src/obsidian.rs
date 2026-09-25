//! The Obsidian vault importer and its frontmatter stripper — port of
//! scripts/import-obsidian-vault.ts, strip-obsidian-frontmatter.ts and
//! scripts/lib/*. The TS importer wrote the old "type:" project format
//! (unreadable by hexend); this one writes the current one.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use hexen_proto::hexen::v1::{
    grid, location_content, project_content, Grid, GridStyle, HexGrid, HexenProject, ImageRef,
    Link, Location, LocationContent, ObsidianLocationContent, ObsidianProjectContent, Point,
    ProjectContent,
};
use project_ops::content::split_frontmatter;
use project_ops::document::serialize_hexen_project;
use project_ops::image_size::read_image_size;
use serde_yml::Value;
use unicode_normalization::UnicodeNormalization;

pub struct VaultEntry {
    /// Path to the note, relative to the vault root, `/`-separated.
    pub relative_path: String,
    pub frontmatter: Value,
}

/// Every frontmatter key the importer reads off a map-root note — also
/// exactly what the stripper removes from one.
pub const MAP_ROOT_FIELDS: &[&str] = &[
    "map-root",
    "map-id",
    "map-hex-origin-x",
    "map-hex-origin-y",
    "map-hex-b1-x",
    "map-hex-b1-y",
    "map-hex-b2-x",
    "map-hex-b2-y",
    "map-hex-km-per-hex",
    "map-hex-color",
    "map-hex-weight",
    "map-hex-opacity",
    "map-img",
    "map-base",
];

/// Every frontmatter key the importer reads off a pinned location note.
pub const PIN_FIELDS: &[&str] = &["map", "map-x", "map-y", "map-type", "map-icon", "map-color"];

/// A note's filename as a Location id. Not a general transliterator —
/// handles what the illuminated-world vault actually contains
/// (apostrophes, a few accented letters).
pub fn slugify(text: &str) -> String {
    let folded: String = text
        .nfkd()
        .filter(|c| !('\u{0300}'..='\u{036f}').contains(c)) // combining marks left by NFKD
        .map(|c| match c {
            'ø' => 'o',
            'Ø' => 'O',
            c => c,
        })
        .collect::<String>()
        .to_lowercase()
        .replace('\'', "");
    let mut slug = String::new();
    for c in folded.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

fn num(fm: &Value, key: &str) -> Option<f64> {
    fm.get(key).and_then(Value::as_f64)
}

fn text(fm: &Value, key: &str) -> Option<String> {
    fm.get(key).and_then(Value::as_str).map(str::to_string)
}

fn is_map_root(fm: &Value) -> bool {
    fm.get("map-root").and_then(Value::as_bool) == Some(true) && text(fm, "map-id").is_some()
}

fn is_pin(fm: &Value) -> bool {
    text(fm, "map").is_some() && num(fm, "map-x").is_some() && num(fm, "map-y").is_some()
}

fn obsidian_content(r#ref: &str) -> Option<LocationContent> {
    Some(LocationContent {
        kind: Some(location_content::Kind::Obsidian(ObsidianLocationContent {
            r#ref: r#ref.to_string(),
        })),
    })
}

/// Pure: frontmatter (already read) plus each map's already-resolved
/// image in, Location/Link graph out. A map-root note becomes a Location
/// with a grid; each pinned note becomes its own Location (for its
/// content) and a Link on its map's Location (for its position).
pub fn build_hexen_project(
    entries: &[VaultEntry],
    title: &str,
    vault_root: &str,
    map_images: &HashMap<String, ImageRef>,
) -> (HexenProject, Vec<String>) {
    let mut warnings = Vec::new();
    let mut locations: Vec<Location> = Vec::new();
    let mut links_by_map: HashMap<String, Vec<Link>> = HashMap::new();

    let roots: Vec<&VaultEntry> = entries
        .iter()
        .filter(|e| is_map_root(&e.frontmatter))
        .collect();
    for root in &roots {
        let fm = &root.frontmatter;
        let map_id = text(fm, "map-id").unwrap_or_default();
        let image = map_images.get(&map_id).cloned();
        if image.is_none() {
            warnings.push(format!(
                "No resolved image for map \"{map_id}\" — image will be null."
            ));
        }
        let calibrated = ["map-hex-origin-x", "map-hex-b1-x", "map-hex-b2-x"]
            .iter()
            .all(|k| num(fm, k).is_some());
        let point = |x: &str, y: &str| {
            Some(Point {
                x: num(fm, x).unwrap_or(0.0),
                y: num(fm, y).unwrap_or(0.0),
            })
        };
        let grid = calibrated.then(|| Grid {
            kind: Some(grid::Kind::Hex(HexGrid {
                origin: point("map-hex-origin-x", "map-hex-origin-y"),
                b1: point("map-hex-b1-x", "map-hex-b1-y"),
                b2: point("map-hex-b2-x", "map-hex-b2-y"),
                distance_per_cell: num(fm, "map-hex-km-per-hex"),
                style: Some(GridStyle {
                    color: text(fm, "map-hex-color").unwrap_or_else(|| "#c19a5f".into()),
                    weight: Some(num(fm, "map-hex-weight").unwrap_or(1.0)),
                    opacity: Some(num(fm, "map-hex-opacity").unwrap_or(0.45)),
                }),
            })),
        });
        locations.push(Location {
            id: map_id.clone(),
            grid,
            image,
            content: obsidian_content(&root.relative_path),
            links: vec![],
            fog: None,
        });
        links_by_map.insert(map_id, Vec::new());
    }

    for pin in entries.iter().filter(|e| is_pin(&e.frontmatter)) {
        let fm = &pin.frontmatter;
        let map_id = text(fm, "map").unwrap_or_default();
        let stem = pin
            .relative_path
            .rsplit('/')
            .next()
            .unwrap_or(&pin.relative_path);
        let stem = stem
            .strip_suffix(".md")
            .or_else(|| stem.strip_suffix(".MD"))
            .unwrap_or(stem);
        let id = slugify(stem);

        locations.push(Location {
            id: id.clone(),
            content: obsidian_content(&pin.relative_path),
            ..Default::default()
        });

        let Some(links) = links_by_map.get_mut(&map_id) else {
            warnings.push(format!(
                "\"{}\" has map: \"{map_id}\", but no map-root note declares that map-id — dropping its pin.",
                pin.relative_path
            ));
            continue;
        };
        links.push(Link {
            id: uuid::Uuid::new_v4().to_string(),
            target: id,
            x: num(fm, "map-x").unwrap_or(0.0),
            y: num(fm, "map-y").unwrap_or(0.0),
            r#type: text(fm, "map-type").unwrap_or_else(|| "waypoint".into()),
            icon: text(fm, "map-icon"),
            color: text(fm, "map-color"),
            hidden: Some(false),
        });
    }

    for location in &mut locations {
        if let Some(links) = links_by_map.remove(&location.id) {
            location.links = links;
        }
    }

    let default_location = match roots.as_slice() {
        [] => {
            warnings.push("No map-root note found — defaultLocation will be a guess.".into());
            locations.first().map(|l| l.id.clone()).unwrap_or_default()
        }
        [first, rest @ ..] => {
            let id = text(&first.frontmatter, "map-id").unwrap_or_default();
            if !rest.is_empty() {
                warnings.push(format!(
                    "{} map-root notes found; using \"{id}\" as defaultLocation.",
                    roots.len()
                ));
            }
            id
        }
    };

    let project = HexenProject {
        schema_version: 1,
        title: title.to_string(),
        default_location,
        content: Some(ProjectContent {
            kind: Some(project_content::Kind::Obsidian(ObsidianProjectContent {
                vault_root: vault_root.to_string(),
            })),
        }),
        locations,
    };
    (project, warnings)
}

/// Underscore- and dot-prefixed folders hold assets, not notes (Quartz's
/// own convention) — skipped entirely.
fn is_hidden_dir(name: &str) -> bool {
    name.starts_with('_') || name.starts_with('.') || name == "node_modules"
}

pub fn read_vault_entries(vault: &Path) -> Result<Vec<VaultEntry>, String> {
    fn walk(vault: &Path, dir: &Path, out: &mut Vec<VaultEntry>) -> Result<(), String> {
        let mut items: Vec<_> = fs::read_dir(dir)
            .map_err(|e| format!("{}: {e}", dir.display()))?
            .filter_map(Result::ok)
            .collect();
        items.sort_by_key(|e| e.file_name());
        for item in items {
            let path = item.path();
            let name = item.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !is_hidden_dir(&name) {
                    walk(vault, &path, out)?;
                }
                continue;
            }
            if !name.to_lowercase().ends_with(".md") {
                continue;
            }
            let raw = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            if raw.trim().is_empty() {
                continue; // stub notes, e.g. an unwritten wikilink target
            }
            let relative = path.strip_prefix(vault).unwrap_or(&path);
            out.push(VaultEntry {
                relative_path: relative
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/"),
                frontmatter: split_frontmatter(&raw).0,
            });
        }
        Ok(())
    }
    let mut entries = Vec::new();
    walk(vault, vault, &mut entries)?;
    Ok(entries)
}

/// `path` relative to `base`, `/`-separated, for paths written into a
/// .hexen.yml. Both are made absolute first.
fn relative_to(path: &Path, base: &Path) -> String {
    let abs = |p: &Path| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let (path, base) = (abs(path), abs(base));
    let common = path
        .components()
        .zip(base.components())
        .take_while(|(a, b)| a == b)
        .count();
    let ups = base.components().count() - common;
    let rel: PathBuf = std::iter::repeat_n(std::path::Component::ParentDir.as_os_str(), ups)
        .chain(path.components().skip(common).map(|c| c.as_os_str()))
        .collect();
    let rel = rel.to_string_lossy().replace('\\', "/");
    if rel.is_empty() {
        ".".into()
    } else {
        rel
    }
}

pub fn import(vault: &Path, out: &Path, title: &str, assets_dir: &str) -> Result<(), String> {
    let out_dir = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let entries = read_vault_entries(vault)?;

    let mut map_images = HashMap::new();
    fs::create_dir_all(out_dir.join(assets_dir)).map_err(|e| e.to_string())?;
    for map_id in entries
        .iter()
        .filter(|e| is_map_root(&e.frontmatter))
        .filter_map(|e| text(&e.frontmatter, "map-id"))
    {
        // The base image always lives at _maps/<id>/map.png, whatever a
        // map-root note's own map-img says (nothing ever read that).
        let source = vault.join("_maps").join(&map_id).join("map.png");
        let Ok(bytes) = fs::read(&source) else {
            eprintln!(
                "Warning: no image at {} for map \"{map_id}\" — skipping its image.",
                source.display()
            );
            continue;
        };
        let Some(size) = read_image_size(&bytes) else {
            eprintln!(
                "Warning: {} doesn't look like a PNG — skipping its image.",
                source.display()
            );
            continue;
        };
        let dest = out_dir.join(assets_dir).join(format!("{map_id}.png"));
        fs::copy(&source, &dest).map_err(|e| format!("{}: {e}", dest.display()))?;
        map_images.insert(
            map_id,
            ImageRef {
                file: relative_to(&dest, out_dir),
                width: size.width as i32,
                height: size.height as i32,
            },
        );
    }

    let (project, warnings) =
        build_hexen_project(&entries, title, &relative_to(vault, out_dir), &map_images);
    for warning in warnings {
        eprintln!("Warning: {warning}");
    }
    let yaml = serialize_hexen_project(&project).map_err(|e| e.to_string())?;
    fs::write(out, yaml).map_err(|e| format!("{}: {e}", out.display()))?;
    let maps = project
        .locations
        .iter()
        .filter(|l| l.grid.is_some())
        .count();
    println!(
        "Wrote {}: {} locations ({maps} with their own map).",
        out.display(),
        project.locations.len()
    );
    Ok(())
}

/// Removes specific top-level keys from a note's frontmatter by
/// filtering lines, not re-serializing the YAML — field order, quoting
/// and comments survive for everything that wasn't removed. Only simple
/// `key: value` lines (everything the importer reads is one).
pub fn strip_frontmatter_fields(raw: &str, fields: &[&str]) -> (String, Vec<String>) {
    let Some(rest) = raw
        .strip_prefix("---\n")
        .or_else(|| raw.strip_prefix("---\r\n"))
    else {
        return (raw.to_string(), vec![]);
    };
    // The closing delimiter: a "---" line.
    let mut offset = 0;
    let mut close = None;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            close = Some((offset, offset + line.len()));
            break;
        }
        offset += line.len();
    }
    let Some((block_end, after)) = close else {
        return (raw.to_string(), vec![]);
    };

    let mut removed = Vec::new();
    let kept: Vec<&str> = rest[..block_end]
        .lines()
        .filter(|line| {
            let key: String = line
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            let is_key = !key.is_empty() && line[key.len()..].starts_with(':');
            if is_key && fields.contains(&key.as_str()) {
                removed.push(key);
                return false;
            }
            true
        })
        .collect();
    let block = if kept.is_empty() {
        String::new()
    } else {
        format!("{}\n", kept.join("\n"))
    };
    (format!("---\n{block}---\n{}", &rest[after..]), removed)
}

pub fn strip(vault: &Path, dry_run: bool) -> Result<(), String> {
    let mut changed = 0;
    for entry in read_vault_entries(vault)? {
        let fm = &entry.frontmatter;
        let fields = if fm.get("map-root").and_then(Value::as_bool) == Some(true) {
            MAP_ROOT_FIELDS
        } else if text(fm, "map").is_some() {
            PIN_FIELDS
        } else {
            continue;
        };
        let path = vault.join(&entry.relative_path);
        let raw = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let (content, removed) = strip_frontmatter_fields(&raw, fields);
        if removed.is_empty() {
            continue;
        }
        changed += 1;
        println!("{}: removed {}", entry.relative_path, removed.join(", "));
        if !dry_run {
            fs::write(&path, content).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    println!(
        "{} {changed} file(s).",
        if dry_run {
            "Dry run: would change"
        } else {
            "Changed"
        }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_cases() {
        assert_eq!(slugify("An Unnamed Chapel"), "an-unnamed-chapel");
        assert_eq!(slugify("Willard's Hideout"), "willards-hideout");
        assert_eq!(slugify("Frøsselanding"), "frosselanding");
        assert_eq!(
            slugify("The Temple of 1,000 Swords"),
            "the-temple-of-1-000-swords"
        );
        assert_eq!(slugify("Café Noël"), "cafe-noel");
    }

    fn yaml(s: &str) -> Value {
        serde_yml::from_str(s).unwrap()
    }

    fn map_root() -> VaultEntry {
        VaultEntry {
            relative_path: "Locations/Vilheim.md".into(),
            frontmatter: yaml(
                "map-root: true\nmap-id: vilheim-crace\ntitle: Vilheim & Crace\nmap-hex-origin-x: 0\nmap-hex-origin-y: 0\n\
                 map-hex-b1-x: 10\nmap-hex-b1-y: 0\nmap-hex-b2-x: 0\nmap-hex-b2-y: 10\nmap-hex-km-per-hex: 9\nmap-hex-color: '#c19a5f'\n",
            ),
        }
    }

    fn pin(path: &str, extra: &str) -> VaultEntry {
        VaultEntry {
            relative_path: path.into(),
            frontmatter: yaml(&format!(
                "map: vilheim-crace\nmap-x: 10\nmap-y: 20\n{extra}"
            )),
        }
    }

    fn images() -> HashMap<String, ImageRef> {
        let image = ImageRef {
            file: "_assets/vilheim-crace.png".into(),
            width: 100,
            height: 100,
        };
        HashMap::from([("vilheim-crace".to_string(), image)])
    }

    #[test]
    fn builds_a_map_location_plus_a_pin_location_and_link() {
        let entries = [
            map_root(),
            pin(
                "Locations/Pentegil Manor.md",
                "map-type: settlement\nmap-icon: village\n",
            ),
        ];
        let (project, warnings) = build_hexen_project(&entries, "Test", ".", &images());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(project.default_location, "vilheim-crace");

        let map = project
            .locations
            .iter()
            .find(|l| l.id == "vilheim-crace")
            .unwrap();
        assert!(
            matches!(map.grid.as_ref().and_then(|g| g.kind.as_ref()), Some(grid::Kind::Hex(h)) if h.distance_per_cell == Some(9.0))
        );
        assert_eq!(map.image, images().remove("vilheim-crace"));
        assert_eq!(map.content, obsidian_content("Locations/Vilheim.md"));
        let link = &map.links[0];
        assert_eq!(
            (
                link.target.as_str(),
                link.x,
                link.y,
                link.r#type.as_str(),
                link.icon.as_deref(),
                &link.color,
                link.hidden
            ),
            (
                "pentegil-manor",
                10.0,
                20.0,
                "settlement",
                Some("village"),
                &None,
                Some(false)
            )
        );
        assert!(!link.id.is_empty());

        let pin = project
            .locations
            .iter()
            .find(|l| l.id == "pentegil-manor")
            .unwrap();
        assert!(pin.grid.is_none());
        assert_eq!(pin.content, obsidian_content("Locations/Pentegil Manor.md"));
    }

    #[test]
    fn a_pin_on_an_unknown_map_keeps_its_location_but_drops_the_link() {
        let mut orphan = pin("Locations/Orphan.md", "");
        orphan.frontmatter = yaml("map: nowhere\nmap-x: 1\nmap-y: 2\n");
        let (project, warnings) =
            build_hexen_project(&[map_root(), orphan], "Test", ".", &images());
        assert!(warnings.iter().any(|w| w.contains("nowhere")));
        assert!(project
            .locations
            .iter()
            .find(|l| l.id == "vilheim-crace")
            .unwrap()
            .links
            .is_empty());
        assert!(project.locations.iter().any(|l| l.id == "orphan"));
    }

    #[test]
    fn a_map_without_an_image_warns() {
        let (project, warnings) = build_hexen_project(&[map_root()], "Test", ".", &HashMap::new());
        assert!(warnings.iter().any(|w| w.contains("vilheim-crace")));
        assert!(project.locations[0].image.is_none());
    }

    #[test]
    fn pin_type_icon_and_color_default() {
        let (project, _) = build_hexen_project(
            &[map_root(), pin("Locations/Bare.md", "")],
            "Test",
            ".",
            &images(),
        );
        let link = &project.locations[0].links[0];
        assert_eq!(
            (link.r#type.as_str(), &link.icon, &link.color),
            ("waypoint", &None, &None)
        );
    }

    #[test]
    fn the_result_is_readable_by_hexend() {
        let (project, _) = build_hexen_project(
            &[map_root(), pin("Locations/Bare.md", "")],
            "Test",
            ".",
            &images(),
        );
        let yaml = serialize_hexen_project(&project).unwrap();
        let parsed = project_ops::document::parse_hexen_project(&yaml).unwrap();
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
        assert_eq!(parsed.project, project);
    }

    #[test]
    fn strip_removes_only_named_fields() {
        let raw = "---\nmap: vilheim-crace\nmap-type: ruin\nsummary: A small chapel.\n---\n\nBody text stays exactly as it was.\n";
        let (content, mut removed) =
            strip_frontmatter_fields(raw, &["map", "map-type", "map-x", "map-y"]);
        removed.sort();
        assert_eq!(removed, ["map", "map-type"]);
        assert_eq!(
            content,
            "---\nsummary: A small chapel.\n---\n\nBody text stays exactly as it was.\n"
        );
    }

    #[test]
    fn strip_leaves_unmatched_or_frontmatterless_notes_alone() {
        let raw = "---\ntitle: Untouched\n---\nBody.\n";
        assert_eq!(
            strip_frontmatter_fields(raw, &["map"]),
            (raw.to_string(), vec![])
        );
        let raw = "Just a body, no frontmatter.\n";
        assert_eq!(
            strip_frontmatter_fields(raw, &["map"]),
            (raw.to_string(), vec![])
        );
    }

    #[test]
    fn strip_can_empty_the_block_and_keeps_quoting() {
        let (content, _) =
            strip_frontmatter_fields("---\nmap: x\nmap-x: 1\n---\nBody.\n", &["map", "map-x"]);
        assert_eq!(content, "---\n---\nBody.\n");
        let (content, _) = strip_frontmatter_fields(
            "---\nmap-color: \"#f66151\"\nsummary: \"\"\n---\nBody.\n",
            &["map-color"],
        );
        assert_eq!(content, "---\nsummary: \"\"\n---\nBody.\n");
    }

    #[test]
    fn relative_paths() {
        assert_eq!(
            relative_to(Path::new("/a/b/c.png"), Path::new("/a")),
            "b/c.png"
        );
        assert_eq!(
            relative_to(Path::new("/a/vault"), Path::new("/a/out")),
            "../vault"
        );
        assert_eq!(relative_to(Path::new("/a"), Path::new("/a")), ".");
    }
}
