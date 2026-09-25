//! Client/UI state only — port of apps/editor's zustand store. The
//! project's own data arrives from the storage backend and is held by
//! the editor component; the backend itself lives beside this in a
//! local signal (a browser folder handle can't live in plain state).
//! Every rule about which switch resets which lives here, as plain
//! methods, so it's testable without a browser.

#[derive(Debug, Clone, PartialEq)]
pub struct UiState {
    pub current_location_id: Option<String>,
    pub selected_link_id: Option<String>,
    /// Gates every mutating surface behind an explicit switch, off by
    /// default — reading a map at the table shouldn't risk changing it.
    pub edit_mode: bool,
    /// A second, independent switch for fog of war: alone it gives a
    /// read-only fog "layers panel" and a readout of what players see;
    /// with `edit_mode` too, the fog paint tool and blanket actions.
    /// Independent on purpose — during play a GM often wants map
    /// authoring locked while still checking/controlling fog.
    pub gm_mode: bool,
    /// The fog paint tool's armed state; only meaningful with both
    /// `edit_mode` and `gm_mode` on.
    pub painting_fog: bool,
    /// A view toggle, not a safety gate — carries across projects and
    /// locations rather than resetting.
    pub grid_visible: bool,
    /// The Add Location tool's armed state — a click on the map places
    /// a pin. Exited whenever the surrounding context changes.
    pub placing_location: bool,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            current_location_id: None,
            selected_link_id: None,
            edit_mode: false,
            gm_mode: false,
            painting_fog: false,
            grid_visible: true,
            placing_location: false,
        }
    }
}

impl UiState {
    /// A different project was opened (any backend): everything but the
    /// view toggles goes back to its defaults.
    pub fn open_project(&mut self) {
        *self = UiState {
            grid_visible: self.grid_visible,
            ..UiState::default()
        };
    }

    pub fn set_current_location(&mut self, id: Option<String>) {
        self.current_location_id = id;
        self.selected_link_id = None;
        self.placing_location = false;
        self.painting_fog = false;
    }

    pub fn select_link(&mut self, id: Option<String>) {
        self.selected_link_id = id;
    }

    pub fn set_edit_mode(&mut self, edit_mode: bool) {
        self.edit_mode = edit_mode;
        self.selected_link_id = None;
        self.placing_location = false;
        self.painting_fog = false;
    }

    pub fn set_gm_mode(&mut self, gm_mode: bool) {
        self.gm_mode = gm_mode;
        self.painting_fog = false;
    }

    pub fn set_painting_fog(&mut self, painting_fog: bool) {
        self.painting_fog = painting_fog;
        self.selected_link_id = None;
    }

    pub fn set_grid_visible(&mut self, grid_visible: bool) {
        self.grid_visible = grid_visible;
    }

    pub fn set_placing_location(&mut self, placing_location: bool) {
        self.placing_location = placing_location;
        self.selected_link_id = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    #[test]
    fn opening_a_project_resets_location_and_link_selection() {
        let mut s = UiState::default();
        s.set_current_location(some("somewhere"));
        s.select_link(some("something"));
        s.open_project();
        assert_eq!(s.current_location_id, None);
        assert_eq!(s.selected_link_id, None);
    }

    #[test]
    fn switching_location_clears_link_selection_but_not_the_location() {
        let mut s = UiState::default();
        s.select_link(some("inn"));
        s.set_current_location(some("town"));
        assert_eq!(s.current_location_id, some("town"));
        assert_eq!(s.selected_link_id, None);
    }

    #[test]
    fn starts_in_view_mode_with_gm_mode_off() {
        let s = UiState::default();
        assert!(!s.edit_mode);
        assert!(!s.gm_mode);
    }

    #[test]
    fn opening_a_project_drops_back_to_view_mode_and_gm_off() {
        let mut s = UiState::default();
        s.set_edit_mode(true);
        s.set_gm_mode(true);
        s.open_project();
        assert!(!s.edit_mode);
        assert!(!s.gm_mode);
    }

    #[test]
    fn switching_edit_mode_clears_link_selection() {
        let mut s = UiState::default();
        s.select_link(some("inn"));
        s.set_edit_mode(true);
        assert_eq!(s.selected_link_id, None);
        assert!(s.edit_mode);
    }

    #[test]
    fn gm_mode_and_edit_mode_are_independent() {
        let mut s = UiState::default();
        s.set_edit_mode(false);
        s.set_gm_mode(true);
        assert!(!s.edit_mode && s.gm_mode);
        s.set_edit_mode(true);
        assert!(s.gm_mode);
    }

    #[test]
    fn turning_off_gm_mode_disarms_the_fog_paint_tool() {
        let mut s = UiState::default();
        s.set_gm_mode(true);
        s.set_painting_fog(true);
        s.set_gm_mode(false);
        assert!(!s.painting_fog);
    }

    #[test]
    fn turning_off_edit_mode_disarms_the_fog_paint_tool() {
        let mut s = UiState::default();
        s.set_gm_mode(true);
        s.set_edit_mode(true);
        s.set_painting_fog(true);
        s.set_edit_mode(false);
        assert!(!s.painting_fog);
    }

    #[test]
    fn switching_location_disarms_both_tools() {
        let mut s = UiState::default();
        s.set_gm_mode(true);
        s.set_edit_mode(true);
        s.set_painting_fog(true);
        s.set_placing_location(true);
        s.set_current_location(some("inn"));
        assert!(!s.painting_fog);
        assert!(!s.placing_location);
    }

    #[test]
    fn arming_either_tool_clears_link_selection() {
        let mut s = UiState::default();
        s.select_link(some("inn"));
        s.set_painting_fog(true);
        assert_eq!(s.selected_link_id, None);
        assert!(s.painting_fog);

        s.select_link(some("inn"));
        s.set_placing_location(true);
        assert_eq!(s.selected_link_id, None);
        assert!(s.placing_location);
    }

    #[test]
    fn grid_is_visible_by_default_and_survives_opening_another_project() {
        let mut s = UiState::default();
        assert!(s.grid_visible);
        s.set_grid_visible(false);
        s.open_project();
        assert!(!s.grid_visible);
    }

    #[test]
    fn leaving_edit_mode_exits_the_add_location_tool() {
        let mut s = UiState::default();
        s.set_edit_mode(true);
        s.set_placing_location(true);
        s.set_edit_mode(false);
        assert!(!s.placing_location);
    }
}
