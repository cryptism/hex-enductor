//! The Command union's reducer. Port of packages/project-ops/src/commands.ts.

use crate::mutations::{
    add_location, add_location_link, save_grid, save_image, save_link, save_location_content,
    set_fog, set_fog_cells, MutationError,
};
use crate::pb::hexen::v1::{command, Command, HexenProject};

pub fn apply_command(project: &mut HexenProject, command: Command) -> Result<(), MutationError> {
    match command.kind {
        Some(command::Kind::SaveLink(c)) => {
            save_link(project, &c.location_id, &c.link_id, c.patch.unwrap_or_default())
        }
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
        Some(command::Kind::SetFogCells(c)) => set_fog_cells(project, &c.location_id, &c.cells, c.revealed),
        Some(command::Kind::AddLocation(c)) => add_location(project, &c.location_id),
        None => Err(MutationError::EmptyCommand),
    }
}
