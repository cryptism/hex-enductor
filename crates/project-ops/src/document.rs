//! YAML <-> HexenProject, plus the cross-reference warnings check.
//! Port of packages/hexen-schema/src/index.ts, in hexen-proto's pbjson
//! dialect.

use std::collections::HashSet;

use hexen_proto::hexen::v1::HexenProject;
use thiserror::Error;

pub struct ParsedHexenProject {
    pub project: HexenProject,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(transparent)]
    Yaml(#[from] serde_yml::Error),
    #[error("Unsupported schemaVersion {0} (expected 1)")]
    UnsupportedSchemaVersion(i32),
}

pub fn parse_hexen_project(yaml_text: &str) -> Result<ParsedHexenProject, ParseError> {
    let project: HexenProject = serde_yml::from_str(yaml_text)?;
    if project.schema_version != 1 {
        return Err(ParseError::UnsupportedSchemaVersion(project.schema_version));
    }
    let warnings = validate_references(&project);
    Ok(ParsedHexenProject { project, warnings })
}

pub fn serialize_hexen_project(project: &HexenProject) -> Result<String, serde_yml::Error> {
    serde_yml::to_string(project)
}

pub fn validate_references(project: &HexenProject) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut seen_ids: HashSet<&str> = HashSet::new();

    for location in &project.locations {
        if !seen_ids.insert(location.id.as_str()) {
            warnings.push(format!("Duplicate location id \"{}\"", location.id));
        }
    }

    if !seen_ids.contains(project.default_location.as_str()) {
        warnings.push(format!(
            "defaultLocation \"{}\" isn't a known location id",
            project.default_location
        ));
    }

    for location in &project.locations {
        for link in &location.links {
            if !seen_ids.contains(link.target.as_str()) {
                warnings.push(format!(
                    "Location \"{}\" links to unknown location id \"{}\"",
                    location.id, link.target
                ));
            }
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = r#"
schemaVersion: 1
title: Test Realm
defaultLocation: town
content:
  inline: {}
locations:
  - id: town
    content:
      inline:
        title: The Town
    links:
      - id: front-door
        target: inn
        x: 1
        y: 1
        type: settlement
      - id: dangling
        target: nowhere
        x: 0
        y: 0
        type: landmark
  - id: inn
"#;

    #[test]
    fn parses_and_warns_about_dangling_links() {
        let parsed = parse_hexen_project(YAML).unwrap();
        assert_eq!(parsed.project.title, "Test Realm");
        assert_eq!(parsed.project.locations.len(), 2);
        assert_eq!(
            parsed.warnings,
            vec!["Location \"town\" links to unknown location id \"nowhere\"".to_string()]
        );
    }

    #[test]
    fn round_trips_through_serialize() {
        let parsed = parse_hexen_project(YAML).unwrap();
        let again =
            parse_hexen_project(&serialize_hexen_project(&parsed.project).unwrap()).unwrap();
        assert_eq!(again.project, parsed.project);
    }

    #[test]
    fn rejects_other_schema_versions() {
        let yaml = YAML.replace("schemaVersion: 1", "schemaVersion: 2");
        assert!(matches!(
            parse_hexen_project(&yaml),
            Err(ParseError::UnsupportedSchemaVersion(2))
        ));
    }

    #[test]
    fn rejects_the_old_type_discriminated_dialect() {
        // What packages/hexen-schema (and the TS editor's local-folder
        // storage) wrote — not readable here; see
        // scripts/migrate-project-to-wire-format.ts.
        let old = "schemaVersion: 1\ntitle: T\ndefaultLocation: a\ncontent: { type: inline }\nlocations: []\n";
        assert!(parse_hexen_project(old).is_err());
    }

    #[test]
    fn warns_on_duplicate_ids_and_an_unknown_default() {
        let yaml = "schemaVersion: 1\ntitle: T\ndefaultLocation: z\ncontent: { inline: {} }\nlocations: [{ id: a }, { id: a }]\n";
        let warnings = parse_hexen_project(yaml).unwrap().warnings;
        assert!(warnings.contains(&"Duplicate location id \"a\"".to_string()));
        assert!(warnings.contains(&"defaultLocation \"z\" isn't a known location id".to_string()));
    }
}
