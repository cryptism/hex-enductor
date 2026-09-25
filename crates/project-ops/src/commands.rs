//! The Command union's reducer. Port of packages/project-ops/src/commands.ts.

use crate::mutations::{
    add_location, add_location_link, save_grid, save_image, save_link, save_location_content,
    set_fog, set_fog_cells, MutationError,
};
use hexen_proto::hexen::v1::{command, Command, HexenProject};

pub fn apply_command(project: &mut HexenProject, command: Command) -> Result<(), MutationError> {
    match command.kind {
        Some(command::Kind::SaveLink(c)) => save_link(
            project,
            &c.location_id,
            &c.link_id,
            c.patch.unwrap_or_default(),
        ),
        Some(command::Kind::SaveLocationContent(c)) => {
            save_location_content(project, &c.location_id, c.patch.unwrap_or_default())
        }
        Some(command::Kind::AddLocationLink(c)) => add_location_link(
            project,
            &c.parent_location_id,
            &c.target_location_id,
            c.x,
            c.y,
            c.link_type,
        ),
        Some(command::Kind::SaveGrid(c)) => save_grid(project, &c.location_id, c.grid),
        Some(command::Kind::SaveImage(c)) => save_image(project, &c.location_id, c.image),
        Some(command::Kind::SetFog(c)) => set_fog(project, &c.location_id, c.fog),
        Some(command::Kind::SetFogCells(c)) => {
            set_fog_cells(project, &c.location_id, &c.cells, c.revealed)
        }
        Some(command::Kind::AddLocation(c)) => add_location(project, &c.location_id),
        None => Err(MutationError::EmptyCommand),
    }
}

#[cfg(test)]
mod tests {
    use hexen_proto::hexen::v1::{
        AddLocationCommand, AddLocationLinkCommand, FogOfWar, ImageRef, LinkPatch, Location,
        SaveImageCommand, SaveLinkCommand, SetFogCellsCommand, SetFogCommand,
    };

    use super::*;

    fn fixture() -> HexenProject {
        let mut p = HexenProject {
            schema_version: 1,
            default_location: "town".into(),
            ..Default::default()
        };
        let mut town = Location {
            id: "town".into(),
            ..Default::default()
        };
        crate::mutations::add_location(&mut p, "inn").unwrap();
        town.links.push(hexen_proto::hexen::v1::Link {
            id: "front-door".into(),
            target: "inn".into(),
            ..Default::default()
        });
        p.locations.insert(0, town);
        p
    }

    fn cmd(kind: command::Kind) -> Command {
        Command { kind: Some(kind) }
    }

    #[test]
    fn dispatches_save_link() {
        let mut p = fixture();
        let c = SaveLinkCommand {
            location_id: "town".into(),
            link_id: "front-door".into(),
            patch: Some(LinkPatch {
                hidden: Some(true),
                ..Default::default()
            }),
        };
        apply_command(&mut p, cmd(command::Kind::SaveLink(c))).unwrap();
        assert_eq!(p.locations[0].links[0].hidden, Some(true));
    }

    #[test]
    fn dispatches_add_location_link_mapping_link_type_to_the_pin_type() {
        let mut p = fixture();
        let c = AddLocationLinkCommand {
            parent_location_id: "town".into(),
            target_location_id: "old-mill".into(),
            x: 5.0,
            y: 6.0,
            link_type: "landmark".into(),
        };
        apply_command(&mut p, cmd(command::Kind::AddLocationLink(c))).unwrap();
        let link = p.locations[0]
            .links
            .iter()
            .find(|l| l.target == "old-mill")
            .unwrap();
        assert_eq!(
            (link.r#type.as_str(), link.x, link.y),
            ("landmark", 5.0, 6.0)
        );
    }

    #[test]
    fn dispatches_save_image_and_fog() {
        let mut p = fixture();
        let image = ImageRef {
            file: "_assets/town.png".into(),
            width: 400,
            height: 300,
        };
        let c = SaveImageCommand {
            location_id: "town".into(),
            image: Some(image.clone()),
        };
        apply_command(&mut p, cmd(command::Kind::SaveImage(c))).unwrap();
        assert_eq!(p.locations[0].image, Some(image));

        let c = SetFogCommand {
            location_id: "town".into(),
            fog: Some(FogOfWar::default()),
        };
        apply_command(&mut p, cmd(command::Kind::SetFog(c))).unwrap();
        let c = SetFogCellsCommand {
            location_id: "town".into(),
            cells: vec!["0,0".into()],
            revealed: true,
        };
        apply_command(&mut p, cmd(command::Kind::SetFogCells(c))).unwrap();
        assert_eq!(
            p.locations[0].fog.as_ref().unwrap().revealed_cells,
            vec!["0,0".to_string()]
        );
    }

    #[test]
    fn dispatches_add_location() {
        let mut p = fixture();
        let c = AddLocationCommand {
            location_id: "staged-map".into(),
        };
        apply_command(&mut p, cmd(command::Kind::AddLocation(c))).unwrap();
        assert!(p.locations.iter().any(|l| l.id == "staged-map"));
    }

    #[test]
    fn an_empty_command_is_an_error() {
        assert!(matches!(
            apply_command(&mut fixture(), Command { kind: None }),
            Err(MutationError::EmptyCommand)
        ));
    }
}
