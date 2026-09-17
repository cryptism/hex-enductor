//! Pure, transport-agnostic operations on an in-memory HexenProject.
//! Port of packages/project-ops/src/mutations.ts.

use thiserror::Error;
use uuid::Uuid;

use crate::pb::hexen::v1::{
    location_content, FogOfWar, ImageRef, InlineContentPatch, InlineLocationContent, Link, LinkPatch,
    Location, LocationContent, Grid, HexenProject, ProjectContent,
};

#[derive(Debug, Error)]
pub enum MutationError {
    #[error("No location \"{0}\" in this project")]
    NoLocation(String),
    #[error("Location \"{0}\" has no link \"{1}\"")]
    NoLink(String, String),
    #[error("Location \"{0}\" has {1} content, not inline")]
    NotInlineContent(String, &'static str),
    #[error("Location \"{0}\" has no fog of war — start it before toggling a cell")]
    NoFog(String),
    #[error("Command message carried no command")]
    EmptyCommand,
}

pub fn create_minimal_project(
    title: String,
    default_location_id: String,
    content: ProjectContent,
) -> HexenProject {
    HexenProject {
        schema_version: 1,
        title,
        default_location: default_location_id.clone(),
        content: Some(content),
        locations: vec![Location {
            id: default_location_id,
            grid: None,
            image: None,
            content: None,
            links: vec![],
            fog: None,
        }],
    }
}

fn find_location<'a>(
    project: &'a mut HexenProject,
    location_id: &str,
) -> Result<&'a mut Location, MutationError> {
    project
        .locations
        .iter_mut()
        .find(|l| l.id == location_id)
        .ok_or_else(|| MutationError::NoLocation(location_id.to_string()))
}

pub fn save_link(
    project: &mut HexenProject,
    location_id: &str,
    link_id: &str,
    patch: LinkPatch,
) -> Result<(), MutationError> {
    let location = find_location(project, location_id)?;
    let link = location
        .links
        .iter_mut()
        .find(|l| l.id == link_id)
        .ok_or_else(|| MutationError::NoLink(location_id.to_string(), link_id.to_string()))?;

    if let Some(id) = patch.id {
        link.id = id;
    }
    if let Some(target) = patch.target {
        link.target = target;
    }
    if let Some(x) = patch.x {
        link.x = x;
    }
    if let Some(y) = patch.y {
        link.y = y;
    }
    if let Some(r#type) = patch.r#type {
        link.r#type = r#type;
    }
    if patch.icon.is_some() {
        link.icon = patch.icon;
    }
    if patch.color.is_some() {
        link.color = patch.color;
    }
    if let Some(hidden) = patch.hidden {
        link.hidden = Some(hidden);
    }

    Ok(())
}

pub fn save_location_content(
    project: &mut HexenProject,
    location_id: &str,
    patch: InlineContentPatch,
) -> Result<(), MutationError> {
    let location = find_location(project, location_id)?;

    let (base_title, base_body) = match &location.content {
        None => (String::new(), String::new()),
        Some(LocationContent {
            kind: Some(location_content::Kind::Inline(inline)),
        }) => (inline.title.clone(), inline.body.clone().unwrap_or_default()),
        Some(_) => {
            return Err(MutationError::NotInlineContent(
                location_id.to_string(),
                "obsidian",
            ))
        }
    };

    location.content = Some(LocationContent {
        kind: Some(location_content::Kind::Inline(InlineLocationContent {
            title: patch.title.unwrap_or(base_title),
            body: Some(patch.body.unwrap_or(base_body)),
        })),
    });

    Ok(())
}

pub fn add_location_link(
    project: &mut HexenProject,
    parent_location_id: &str,
    target_location_id: &str,
    x: f64,
    y: f64,
    link_type: String,
) -> Result<(), MutationError> {
    find_location(project, parent_location_id)?;

    if !project.locations.iter().any(|l| l.id == target_location_id) {
        project.locations.push(Location {
            id: target_location_id.to_string(),
            grid: None,
            image: None,
            content: None,
            links: vec![],
            fog: None,
        });
    }

    let parent = find_location(project, parent_location_id)?;
    parent.links.push(Link {
        id: Uuid::new_v4().to_string(),
        target: target_location_id.to_string(),
        x,
        y,
        r#type: link_type,
        icon: None,
        color: None,
        hidden: Some(false),
    });

    Ok(())
}

pub fn save_grid(
    project: &mut HexenProject,
    location_id: &str,
    grid: Option<Grid>,
) -> Result<(), MutationError> {
    find_location(project, location_id)?.grid = grid;
    Ok(())
}

pub fn save_image(
    project: &mut HexenProject,
    location_id: &str,
    image: Option<ImageRef>,
) -> Result<(), MutationError> {
    find_location(project, location_id)?.image = image;
    Ok(())
}

pub fn set_fog(
    project: &mut HexenProject,
    location_id: &str,
    fog: Option<FogOfWar>,
) -> Result<(), MutationError> {
    find_location(project, location_id)?.fog = fog;
    Ok(())
}

pub fn toggle_fog_cell(
    project: &mut HexenProject,
    location_id: &str,
    cell: &str,
) -> Result<(), MutationError> {
    let location = find_location(project, location_id)?;
    let fog = location
        .fog
        .as_mut()
        .ok_or_else(|| MutationError::NoFog(location_id.to_string()))?;

    if let Some(pos) = fog.revealed_cells.iter().position(|c| c == cell) {
        fog.revealed_cells.remove(pos);
    } else {
        fog.revealed_cells.push(cell.to_string());
    }

    Ok(())
}
