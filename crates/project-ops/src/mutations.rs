//! Pure, transport-agnostic operations on an in-memory HexenProject.
//! Port of packages/project-ops/src/mutations.ts (tests included).

use thiserror::Error;
use uuid::Uuid;

use hexen_proto::hexen::v1::{
    location_content, FogOfWar, Grid, HexenProject, ImageRef, InlineContentPatch,
    InlineLocationContent, Link, LinkPatch, Location, LocationContent, ProjectContent,
};

#[derive(Debug, Error)]
pub enum MutationError {
    #[error("No location \"{0}\" in this project")]
    NoLocation(String),
    #[error("Location \"{0}\" has no link \"{1}\"")]
    NoLink(String, String),
    #[error("Location \"{0}\" has {1} content, not inline")]
    NotInlineContent(String, &'static str),
    #[error("Location \"{0}\" has no fog of war — start it before setting cells")]
    NoFog(String),
    #[error("Location \"{0}\" already exists")]
    LocationExists(String),
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

pub fn add_location(project: &mut HexenProject, location_id: &str) -> Result<(), MutationError> {
    if project.locations.iter().any(|l| l.id == location_id) {
        return Err(MutationError::LocationExists(location_id.to_string()));
    }
    project.locations.push(Location {
        id: location_id.to_string(),
        grid: None,
        image: None,
        content: None,
        links: vec![],
        fog: None,
    });
    Ok(())
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
        }) => (
            inline.title.clone(),
            inline.body.clone().unwrap_or_default(),
        ),
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

pub fn set_fog_cells(
    project: &mut HexenProject,
    location_id: &str,
    cells: &[String],
    revealed: bool,
) -> Result<(), MutationError> {
    let location = find_location(project, location_id)?;
    let fog = location
        .fog
        .as_mut()
        .ok_or_else(|| MutationError::NoFog(location_id.to_string()))?;

    for cell in cells {
        let pos = fog.revealed_cells.iter().position(|c| c == cell);
        match (revealed, pos) {
            (true, None) => fog.revealed_cells.push(cell.clone()),
            (false, Some(pos)) => {
                fog.revealed_cells.remove(pos);
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use hexen_proto::hexen::v1::{
        grid, project_content, GridStyle, HexGrid, InlineProjectContent, ObsidianLocationContent,
        Point,
    };

    use super::*;

    fn inline(title: &str, body: &str) -> Option<LocationContent> {
        Some(LocationContent {
            kind: Some(location_content::Kind::Inline(InlineLocationContent {
                title: title.into(),
                body: Some(body.into()),
            })),
        })
    }

    fn bare(id: &str) -> Location {
        Location {
            id: id.into(),
            ..Default::default()
        }
    }

    fn fixture() -> HexenProject {
        HexenProject {
            schema_version: 1,
            title: "Test Realm".into(),
            default_location: "town".into(),
            content: Some(ProjectContent {
                kind: Some(project_content::Kind::Inline(InlineProjectContent {})),
            }),
            locations: vec![
                Location {
                    id: "town".into(),
                    content: inline("The Town", ""),
                    links: vec![Link {
                        id: "front-door".into(),
                        target: "inn".into(),
                        x: 1.0,
                        y: 1.0,
                        r#type: "settlement".into(),
                        icon: None,
                        color: None,
                        hidden: Some(false),
                    }],
                    ..Default::default()
                },
                bare("inn"),
                bare("well"),
            ],
        }
    }

    fn loc<'a>(p: &'a HexenProject, id: &str) -> &'a Location {
        p.locations.iter().find(|l| l.id == id).unwrap()
    }

    #[test]
    fn create_minimal_project_is_one_bare_location() {
        let content = ProjectContent {
            kind: Some(project_content::Kind::Inline(InlineProjectContent {})),
        };
        let p = create_minimal_project("New Realm".into(), "town".into(), content);
        assert_eq!(p.default_location, "town");
        assert_eq!(p.locations, vec![bare("town")]);
    }

    #[test]
    fn save_link_merges_a_patch() {
        let mut p = fixture();
        save_link(
            &mut p,
            "town",
            "front-door",
            LinkPatch {
                x: Some(9.0),
                hidden: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
        let link = &loc(&p, "town").links[0];
        assert_eq!((link.x, link.y, link.hidden), (9.0, 1.0, Some(true)));
        assert_eq!(link.r#type, "settlement");
    }

    #[test]
    fn save_link_errors_on_unknown_location_or_link() {
        let err = save_link(
            &mut fixture(),
            "nowhere",
            "front-door",
            LinkPatch::default(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("No location \"nowhere\""));
        let err = save_link(&mut fixture(), "town", "ghost", LinkPatch::default()).unwrap_err();
        assert!(err.to_string().contains("has no link \"ghost\""));
    }

    #[test]
    fn save_location_content_defaults_missing_fields_to_empty() {
        let mut p = fixture();
        let patch = InlineContentPatch {
            title: Some("The Inn".into()),
            body: None,
        };
        save_location_content(&mut p, "inn", patch).unwrap();
        assert_eq!(loc(&p, "inn").content, inline("The Inn", ""));
    }

    #[test]
    fn save_location_content_preserves_unpatched_fields() {
        let mut p = fixture();
        let patch = InlineContentPatch {
            title: None,
            body: Some("Updated.".into()),
        };
        save_location_content(&mut p, "town", patch).unwrap();
        assert_eq!(loc(&p, "town").content, inline("The Town", "Updated."));
    }

    #[test]
    fn save_location_content_refuses_non_inline_content() {
        let mut p = fixture();
        p.locations[0].content = Some(LocationContent {
            kind: Some(location_content::Kind::Obsidian(ObsidianLocationContent {
                r#ref: "town.md".into(),
            })),
        });
        let err = save_location_content(&mut p, "town", InlineContentPatch::default()).unwrap_err();
        assert!(err.to_string().contains("not inline"));
    }

    #[test]
    fn add_location_link_creates_the_target_when_new() {
        let mut p = fixture();
        add_location_link(&mut p, "town", "old-mill", 5.0, 6.0, "landmark".into()).unwrap();
        let link = loc(&p, "town")
            .links
            .iter()
            .find(|l| l.target == "old-mill")
            .unwrap();
        assert_eq!(
            (link.x, link.y, link.r#type.as_str(), link.hidden),
            (5.0, 6.0, "landmark", Some(false))
        );
        assert!(!link.id.is_empty());
        assert_eq!(loc(&p, "old-mill"), &bare("old-mill"));
    }

    #[test]
    fn add_location_link_reuses_an_existing_target() {
        let mut p = fixture();
        add_location_link(&mut p, "town", "well", 2.0, 2.0, "landmark".into()).unwrap();
        assert_eq!(p.locations.iter().filter(|l| l.id == "well").count(), 1);
    }

    #[test]
    fn add_location_link_allows_two_entrances_to_one_target() {
        let mut p = fixture();
        add_location_link(&mut p, "town", "inn", 9.0, 9.0, "settlement".into()).unwrap();
        let to_inn: Vec<_> = loc(&p, "town")
            .links
            .iter()
            .filter(|l| l.target == "inn")
            .collect();
        assert_eq!(to_inn.len(), 2);
        assert_ne!(to_inn[0].id, to_inn[1].id);
    }

    #[test]
    fn add_location_adds_a_bare_unlinked_location() {
        let mut p = fixture();
        add_location(&mut p, "staged-map").unwrap();
        assert_eq!(loc(&p, "staged-map"), &bare("staged-map"));
        assert_eq!(loc(&p, "town").links.len(), 1);
    }

    #[test]
    fn add_location_refuses_a_duplicate_id() {
        assert!(add_location(&mut fixture(), "inn")
            .unwrap_err()
            .to_string()
            .contains("already exists"));
    }

    #[test]
    fn save_grid_sets_and_clears() {
        let mut p = fixture();
        let hex = Grid {
            kind: Some(grid::Kind::Hex(HexGrid {
                origin: Some(Point { x: 0.0, y: 0.0 }),
                b1: Some(Point { x: 10.0, y: 0.0 }),
                b2: Some(Point { x: 5.0, y: 8.0 }),
                distance_per_cell: None,
                style: Some(GridStyle {
                    color: "#fff".into(),
                    weight: Some(1.0),
                    opacity: Some(0.5),
                }),
            })),
        };
        save_grid(&mut p, "town", Some(hex.clone())).unwrap();
        assert_eq!(loc(&p, "town").grid, Some(hex));
        save_grid(&mut p, "town", None).unwrap();
        assert_eq!(loc(&p, "town").grid, None);
    }

    #[test]
    fn save_image_sets_clears_and_checks_the_location() {
        let mut p = fixture();
        let image = ImageRef {
            file: "_assets/town.png".into(),
            width: 400,
            height: 300,
        };
        save_image(&mut p, "town", Some(image.clone())).unwrap();
        assert_eq!(loc(&p, "town").image, Some(image));
        save_image(&mut p, "town", None).unwrap();
        assert_eq!(loc(&p, "town").image, None);
        assert!(save_image(&mut p, "nowhere", None).is_err());
    }

    #[test]
    fn set_fog_starts_and_clears() {
        let mut p = fixture();
        set_fog(
            &mut p,
            "town",
            Some(FogOfWar {
                revealed_cells: vec![],
            }),
        )
        .unwrap();
        assert_eq!(
            loc(&p, "town").fog,
            Some(FogOfWar {
                revealed_cells: vec![]
            })
        );
        set_fog(&mut p, "town", None).unwrap();
        assert_eq!(loc(&p, "town").fog, None);
    }

    fn cells(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn set_fog_cells_reveals_then_rehides() {
        let mut p = fixture();
        set_fog(&mut p, "town", Some(FogOfWar::default())).unwrap();
        set_fog_cells(&mut p, "town", &cells(&["2,3", "2,4"]), true).unwrap();
        assert_eq!(
            loc(&p, "town").fog.as_ref().unwrap().revealed_cells,
            cells(&["2,3", "2,4"])
        );
        set_fog_cells(&mut p, "town", &cells(&["2,3", "2,4"]), false).unwrap();
        assert!(loc(&p, "town")
            .fog
            .as_ref()
            .unwrap()
            .revealed_cells
            .is_empty());
    }

    #[test]
    fn set_fog_cells_is_idempotent() {
        let mut p = fixture();
        set_fog(&mut p, "town", Some(FogOfWar::default())).unwrap();
        set_fog_cells(&mut p, "town", &cells(&["2,3"]), true).unwrap();
        set_fog_cells(&mut p, "town", &cells(&["2,3"]), true).unwrap();
        assert_eq!(
            loc(&p, "town").fog.as_ref().unwrap().revealed_cells,
            cells(&["2,3"])
        );
        set_fog_cells(&mut p, "town", &cells(&["2,3"]), false).unwrap();
        set_fog_cells(&mut p, "town", &cells(&["2,3"]), false).unwrap();
        assert!(loc(&p, "town")
            .fog
            .as_ref()
            .unwrap()
            .revealed_cells
            .is_empty());
    }

    #[test]
    fn set_fog_cells_needs_fog_and_a_real_location() {
        let err = set_fog_cells(&mut fixture(), "town", &cells(&["2,3"]), true).unwrap_err();
        assert!(err.to_string().contains("no fog of war"));
        let err = set_fog_cells(&mut fixture(), "nowhere", &cells(&["2,3"]), true).unwrap_err();
        assert!(err.to_string().contains("No location \"nowhere\""));
    }
}
