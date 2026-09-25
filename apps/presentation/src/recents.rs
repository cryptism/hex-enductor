//! Recently watched projects — like the editor's recent server projects,
//! but a presentation URL carries both pieces (`?server=&path=`), since
//! this app has no default server, so an entry is the pair. A
//! convenience only: any storage failure just means an empty list.

use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "hex-enductor:recent-targets";
const MAX_RECENT: usize = 8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Target {
    pub server: String,
    pub path: String,
}

/// `target` moved (or added) to the front of `recent`, capped.
pub fn with_recent(recent: &[Target], target: &Target) -> Vec<Target> {
    std::iter::once(target.clone())
        .chain(recent.iter().filter(|t| *t != target).cloned())
        .take(MAX_RECENT)
        .collect()
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

pub fn recent_targets() -> Vec<Target> {
    storage()
        .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten())
        .and_then(|raw| serde_json::from_str::<Vec<serde_json::Value>>(&raw).ok())
        .map(|items| {
            items
                .into_iter()
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect()
        })
        .unwrap_or_default()
}

pub fn add_recent_target(target: &Target) {
    let updated = with_recent(&recent_targets(), target);
    if let (Some(storage), Ok(json)) = (storage(), serde_json::to_string(&updated)) {
        let _ = storage.set_item(STORAGE_KEY, &json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(path: &str) -> Target {
        Target {
            server: "http://localhost:4000".into(),
            path: path.into(),
        }
    }

    #[test]
    fn adds_to_the_front_without_duplicating() {
        let recent = with_recent(&with_recent(&[], &t("/a")), &t("/b"));
        assert_eq!(recent, [t("/b"), t("/a")]);
        assert_eq!(with_recent(&recent, &t("/a")), [t("/a"), t("/b")]);
    }

    #[test]
    fn the_same_path_on_another_server_is_a_different_entry() {
        let other = Target {
            server: "http://localhost:4001".into(),
            path: "/a".into(),
        };
        assert_eq!(with_recent(&[t("/a")], &other).len(), 2);
    }

    #[test]
    fn caps_at_eight() {
        let recent = (0..10).fold(Vec::new(), |r, i| with_recent(&r, &t(&format!("/p{i}"))));
        assert_eq!(recent.len(), 8);
        assert_eq!(recent[0], t("/p9"));
    }
}
