//! Undo/redo as a command log with a cursor into it, each entry holding
//! the project as it stood after that command. The server session and
//! the editor's local-folder storage both keep one — the server's
//! shared by everyone connected to a project, the local one per tab.

use hexen_proto::hexen::v1::{Command, HexenProject};

use crate::commands::apply_command;
use crate::mutations::MutationError;

struct Entry {
    // Kept for inspection/debugging; undo/redo only ever need snapshots.
    #[allow(dead_code)]
    command: Command,
    snapshot: HexenProject,
}

pub struct History {
    /// The project before the first command — where undo bottoms out.
    base: HexenProject,
    log: Vec<Entry>,
    /// Index of the entry the current state came from; `None` = `base`.
    cursor: Option<usize>,
}

impl History {
    pub fn new(base: HexenProject) -> Self {
        History {
            base,
            log: Vec::new(),
            cursor: None,
        }
    }

    pub fn current(&self) -> &HexenProject {
        match self.cursor {
            Some(i) => &self.log[i].snapshot,
            None => &self.base,
        }
    }

    /// Applies `command` to the current state and returns the result. A
    /// command issued after an undo discards the redo branch. On error
    /// nothing changes.
    pub fn execute(&mut self, command: Command) -> Result<&HexenProject, MutationError> {
        let mut next = self.current().clone();
        apply_command(&mut next, command.clone())?;
        let keep = self.cursor.map_or(0, |i| i + 1);
        self.log.truncate(keep);
        self.log.push(Entry {
            command,
            snapshot: next,
        });
        self.cursor = Some(self.log.len() - 1);
        Ok(self.current())
    }

    /// Steps back one command; `None` if there's nothing to undo.
    pub fn undo(&mut self) -> Option<&HexenProject> {
        self.cursor = match self.cursor? {
            0 => None,
            i => Some(i - 1),
        };
        Some(self.current())
    }

    /// Steps forward one command; `None` if there's nothing to redo.
    pub fn redo(&mut self) -> Option<&HexenProject> {
        let next = self.cursor.map_or(0, |i| i + 1);
        if next >= self.log.len() {
            return None;
        }
        self.cursor = Some(next);
        Some(self.current())
    }
}

#[cfg(test)]
mod tests {
    use hexen_proto::hexen::v1::{command, AddLocationCommand, Location};

    use super::*;

    fn project() -> HexenProject {
        HexenProject {
            schema_version: 1,
            default_location: "town".into(),
            locations: vec![Location {
                id: "town".into(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn add(id: &str) -> Command {
        Command {
            kind: Some(command::Kind::AddLocation(AddLocationCommand {
                location_id: id.into(),
            })),
        }
    }

    fn ids(p: &HexenProject) -> Vec<&str> {
        p.locations.iter().map(|l| l.id.as_str()).collect()
    }

    #[test]
    fn undo_and_redo_walk_the_log() {
        let mut h = History::new(project());
        h.execute(add("a")).unwrap();
        h.execute(add("b")).unwrap();
        assert_eq!(ids(h.undo().unwrap()), ["town", "a"]);
        assert_eq!(ids(h.undo().unwrap()), ["town"]);
        assert!(h.undo().is_none());
        assert_eq!(ids(h.redo().unwrap()), ["town", "a"]);
        assert_eq!(ids(h.redo().unwrap()), ["town", "a", "b"]);
        assert!(h.redo().is_none());
    }

    #[test]
    fn a_new_command_after_undo_drops_the_redo_branch() {
        let mut h = History::new(project());
        h.execute(add("a")).unwrap();
        h.undo();
        h.execute(add("c")).unwrap();
        assert!(h.redo().is_none());
        assert_eq!(ids(h.current()), ["town", "c"]);
    }

    #[test]
    fn a_failed_command_changes_nothing() {
        let mut h = History::new(project());
        h.execute(add("a")).unwrap();
        assert!(h.execute(add("a")).is_err());
        assert_eq!(ids(h.current()), ["town", "a"]);
        assert_eq!(ids(h.undo().unwrap()), ["town"]);
    }
}
