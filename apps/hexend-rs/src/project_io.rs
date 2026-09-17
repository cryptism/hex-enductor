//! YAML <-> HexenProject, plus the cross-reference warnings check.
//! Port of packages/hexen-schema/src/index.ts.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use thiserror::Error;

use crate::pb::hexen::v1::{location_content, project_content, HexenProject, ResolvedContent};
use crate::resolver::{resolve_inline_content, ObsidianResolver, ObsidianResolverConfig};

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

pub struct OpenedProject {
    pub project: HexenProject,
    pub warnings: Vec<String>,
    /// location.id -> resolved content, for every location whose content resolved cleanly.
    pub resolved_content: HashMap<String, ResolvedContent>,
    /// location.id -> the error message, for every location whose content failed to resolve.
    pub resolve_errors: HashMap<String, String>,
}

#[derive(Debug, Error)]
pub enum OpenError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] ParseError),
}

/// Port of apps/hexend/src/projectIO.ts's openProject.
pub async fn open_project(path: &Path) -> Result<OpenedProject, OpenError> {
    let yaml_text = tokio::fs::read_to_string(path).await?;
    let ParsedHexenProject { project, warnings } = parse_hexen_project(&yaml_text)?;

    let mut resolved_content = HashMap::new();
    let mut resolve_errors = HashMap::new();

    // A Location's own content.kind picks how it resolves, independent
    // of the project's — an obsidian-backed project can still hold
    // inline locations that don't warrant a vault file of their own.
    let obsidian_resolver = match &project.content {
        Some(pc) => match &pc.kind {
            Some(project_content::Kind::Obsidian(o)) => {
                let project_dir = path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
                Some(ObsidianResolver::new(ObsidianResolverConfig {
                    project_dir,
                    vault_root: o.vault_root.clone(),
                }))
            }
            _ => None,
        },
        None => None,
    };

    for location in &project.locations {
        let Some(content) = &location.content else {
            continue;
        };

        let result: Result<ResolvedContent, String> = match &content.kind {
            Some(location_content::Kind::Inline(inline)) => Ok(resolve_inline_content(inline)),
            Some(location_content::Kind::Obsidian(obsidian)) => match &obsidian_resolver {
                Some(resolver) => resolver.resolve(obsidian).await.map_err(|e| e.to_string()),
                None => Err(format!(
                    "Location \"{}\" has obsidian content, but this project has no vault configured",
                    location.id
                )),
            },
            None => continue,
        };

        match result {
            Ok(resolved) => {
                resolved_content.insert(location.id.clone(), resolved);
            }
            Err(err) => {
                resolve_errors.insert(location.id.clone(), err);
            }
        }
    }

    Ok(OpenedProject {
        project,
        warnings,
        resolved_content,
        resolve_errors,
    })
}

pub async fn save_project(path: &Path, project: &HexenProject) -> Result<(), OpenError> {
    let yaml_text = serialize_hexen_project(project).map_err(ParseError::Yaml)?;
    tokio::fs::write(path, yaml_text).await?;
    Ok(())
}

fn validate_references(project: &HexenProject) -> Vec<String> {
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
