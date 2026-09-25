//! Old "type:"-discriminated project files (what packages/hexen-schema,
//! and so the TypeScript editor's folder storage and Obsidian importer,
//! wrote) to the current format — port of
//! scripts/migrate-project-to-wire-format.ts. Only the oneof-shaped
//! blocks change shape: project `content`, and each location's `grid`
//! and `content`. Everything else (ids, links — whose own `type` is a
//! plain field — images, fog) is identical in both.

use std::fs;
use std::path::Path;

use hexen_proto::hexen::v1::HexenProject;
use project_ops::document::{parse_hexen_project, serialize_hexen_project};
use serde_yml::{Mapping, Value};

/// `{ type: X, ...rest }` → `{ X: { ...rest } }`; anything else as is.
fn oneof_from_type(value: &mut Value) {
    let Some(map) = value.as_mapping_mut() else {
        return;
    };
    let Some(Value::String(kind)) = map.remove("type") else {
        return;
    };
    let rest = std::mem::take(map);
    let mut wrapped = Mapping::new();
    wrapped.insert(Value::String(kind), Value::Mapping(rest));
    *value = Value::Mapping(wrapped);
}

/// The old format's text converted to a project in the current one.
pub fn migrate(old: &str) -> Result<HexenProject, String> {
    let mut doc: Value = serde_yml::from_str(old).map_err(|e| e.to_string())?;
    if let Some(content) = doc.get_mut("content") {
        oneof_from_type(content);
    }
    if let Some(locations) = doc.get_mut("locations").and_then(Value::as_sequence_mut) {
        for location in locations {
            for key in ["grid", "content"] {
                if let Some(value) = location.get_mut(key) {
                    oneof_from_type(value);
                }
            }
        }
    }
    let text = serde_yml::to_string(&doc).map_err(|e| e.to_string())?;
    let parsed = parse_hexen_project(&text)
        .map_err(|e| format!("not a project in the old format either: {e}"))?;
    for warning in &parsed.warnings {
        eprintln!("Warning: {warning}");
    }
    Ok(parsed.project)
}

pub fn run(path: &Path, dry_run: bool) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if parse_hexen_project(&text).is_ok() {
        println!("{} is already in the current format.", path.display());
        return Ok(());
    }
    let project = migrate(&text)?;
    let yaml = serialize_hexen_project(&project).map_err(|e| e.to_string())?;
    if dry_run {
        print!("{yaml}");
        return Ok(());
    }
    fs::write(path, yaml).map_err(|e| format!("{}: {e}", path.display()))?;
    println!(
        "Migrated {} ({} location(s)) to the current format.",
        path.display(),
        project.locations.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use hexen_proto::hexen::v1::{grid, location_content, project_content};

    use super::*;

    const OLD: &str = r##"
schemaVersion: 1
title: Test Realm
defaultLocation: town
content: { type: obsidian, vaultRoot: _vault }
locations:
  - id: town
    grid:
      type: hex
      origin: { x: 200, y: 200 }
      b1: { x: 60, y: 0 }
      b2: { x: 30, y: 52 }
      distancePerCell: 1
      style: { color: "#c19a5f", weight: 1, opacity: 0.45 }
    image: { file: _assets/town.png, width: 400, height: 400 }
    content: { type: obsidian, ref: town.md }
    links:
      - id: front-door
        target: inn
        x: 1
        y: 1
        type: settlement
        color: null
        hidden: false
    fog: { revealedCells: ["0,0"] }
  - id: inn
    grid: { type: square, origin: { x: 0, y: 0 }, cellSize: { x: 32, y: 32 }, style: { color: "#fff", weight: 1, opacity: 0.5 } }
    image: null
    content: { type: inline, title: The Inn, body: Warm. }
    links: []
"##;

    #[test]
    fn converts_every_oneof_and_keeps_everything_else() {
        let p = migrate(OLD).unwrap();
        assert!(matches!(
            p.content.and_then(|c| c.kind),
            Some(project_content::Kind::Obsidian(o)) if o.vault_root == "_vault"
        ));
        let town = &p.locations[0];
        assert!(
            matches!(town.grid.as_ref().and_then(|g| g.kind.as_ref()), Some(grid::Kind::Hex(h)) if h.b2.unwrap().y == 52.0)
        );
        assert!(matches!(
            town.content.as_ref().and_then(|c| c.kind.as_ref()),
            Some(location_content::Kind::Obsidian(o)) if o.r#ref == "town.md"
        ));
        assert_eq!(town.links[0].r#type, "settlement");
        assert_eq!(town.fog.as_ref().unwrap().revealed_cells, ["0,0"]);
        let inn = &p.locations[1];
        assert!(matches!(
            inn.grid.as_ref().and_then(|g| g.kind.as_ref()),
            Some(grid::Kind::Square(_))
        ));
        assert!(matches!(
            inn.content.as_ref().and_then(|c| c.kind.as_ref()),
            Some(location_content::Kind::Inline(i)) if i.title == "The Inn" && i.body.as_deref() == Some("Warm.")
        ));
    }

    #[test]
    fn the_result_round_trips_in_the_current_format() {
        let p = migrate(OLD).unwrap();
        let again = parse_hexen_project(&serialize_hexen_project(&p).unwrap()).unwrap();
        assert_eq!(again.project, p);
    }

    #[test]
    fn inline_project_content_becomes_an_empty_inline_block() {
        let p = migrate("schemaVersion: 1\ntitle: T\ndefaultLocation: a\ncontent: { type: inline }\nlocations: [{ id: a }]\n")
            .unwrap();
        assert!(matches!(
            p.content.and_then(|c| c.kind),
            Some(project_content::Kind::Inline(_))
        ));
    }
}
