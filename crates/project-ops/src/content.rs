//! Content resolution that doesn't need the filesystem: inline content,
//! and parsing an Obsidian note's text once someone has read it. Port
//! of packages/content-resolver and packages/content-obsidian's
//! parseNote.

use hexen_proto::hexen::v1::{InlineLocationContent, ResolvedContent};

pub fn resolve_inline_content(content: &InlineLocationContent) -> ResolvedContent {
    ResolvedContent {
        title: content.title.clone(),
        summary: None,
        body: content.body.clone().unwrap_or_default(),
    }
}

/// The error a Location with obsidian content gets when its project has
/// no vault configured.
pub fn no_vault_error(location_id: &str) -> String {
    format!(
        "Location \"{location_id}\" has obsidian content, but this project has no vault configured"
    )
}

/// `vault_root` + a note's `ref`, as a `/`-separated path relative to
/// the project's directory — collapsing `.` and empty segments the way
/// node:path.join did, for backends with no real path type (the
/// browser's File System Access API).
pub fn vault_relative_path(vault_root: &str, r#ref: &str) -> String {
    format!("{vault_root}/{}", r#ref)
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/")
}

fn empty_mapping() -> serde_yml::Value {
    serde_yml::Value::Mapping(serde_yml::Mapping::new())
}

/// Splits a leading `---`-delimited YAML frontmatter block off a
/// note's body, returning the frontmatter as a mapping (empty if there
/// is none or it isn't one). Obsidian notes only ever use this one
/// frontmatter style, so a full multi-format parser is more than this
/// needs.
pub fn split_frontmatter(raw: &str) -> (serde_yml::Value, String) {
    let is_delim = |l: &str| l.trim_end_matches('\r') == "---";

    let mut lines = raw.split('\n');
    match lines.next() {
        Some(first) if is_delim(first) => {}
        _ => return (empty_mapping(), raw.to_string()),
    }

    let rest: Vec<&str> = lines.collect();
    let Some(close_idx) = rest.iter().position(|l| is_delim(l)) else {
        return (empty_mapping(), raw.to_string());
    };

    let frontmatter_text = rest[..close_idx].join("\n");
    let body = rest[close_idx + 1..].join("\n");

    let data = serde_yml::from_str::<serde_yml::Value>(&frontmatter_text)
        .unwrap_or_else(|_| empty_mapping());
    let data = if data.is_mapping() {
        data
    } else {
        empty_mapping()
    };
    (data, body)
}

/// Parses a vault note's raw text into title/summary/body — pure, no
/// file I/O.
pub fn parse_obsidian_note(raw: &str, r#ref: &str) -> ResolvedContent {
    let (data, body) = split_frontmatter(raw);

    let last_segment = r#ref.rsplit('/').next().unwrap_or(r#ref);
    let fallback_title = match last_segment.rsplit_once('.') {
        Some((stem, _ext)) => stem,
        None => last_segment,
    };

    let title = match data.get("title").and_then(|v| v.as_str()) {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => fallback_title.to_string(),
    };
    let summary = data
        .get("summary")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    ResolvedContent {
        title,
        summary,
        body: body.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_title_summary_and_body_from_frontmatter() {
        let note =
            "---\ntitle: The Rusty Tankard\nsummary: A cozy inn.\n---\nWarm firelight inside.\n";
        let result = parse_obsidian_note(note, "with-title.md");
        assert_eq!(result.title, "The Rusty Tankard");
        assert_eq!(result.summary.as_deref(), Some("A cozy inn."));
        assert_eq!(result.body, "Warm firelight inside.");
    }

    #[test]
    fn falls_back_to_the_filename_without_a_frontmatter_title() {
        let result = parse_obsidian_note("Just a body, no frontmatter.\n", "sub/No Frontmatter.md");
        assert_eq!(result.title, "No Frontmatter");
        assert_eq!(result.summary, None);
        assert_eq!(result.body, "Just a body, no frontmatter.");
    }

    #[test]
    fn an_unclosed_frontmatter_block_is_all_body() {
        let result = parse_obsidian_note("---\ntitle: x\nno close", "n.md");
        assert_eq!(result.title, "n");
        assert_eq!(result.body, "---\ntitle: x\nno close");
    }

    #[test]
    fn inline_content_defaults_a_missing_body() {
        let resolved = resolve_inline_content(&InlineLocationContent {
            title: "T".into(),
            body: None,
        });
        assert_eq!((resolved.title.as_str(), resolved.body.as_str()), ("T", ""));
    }

    #[test]
    fn vault_paths_collapse_dot_and_empty_segments() {
        assert_eq!(vault_relative_path(".", "town.md"), "town.md");
        assert_eq!(
            vault_relative_path("_vault/", "./places/town.md"),
            "_vault/places/town.md"
        );
    }
}
