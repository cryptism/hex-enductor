//! Content resolution: a Location's `content` block resolved into a
//! title/summary/body. Port of packages/content-resolver and
//! packages/content-obsidian.

use std::path::PathBuf;

use crate::pb::hexen::v1::{InlineLocationContent, ObsidianLocationContent, ResolvedContent};

pub fn resolve_inline_content(content: &InlineLocationContent) -> ResolvedContent {
    ResolvedContent {
        title: content.title.clone(),
        summary: None,
        body: content.body.clone().unwrap_or_default(),
    }
}

fn empty_mapping() -> serde_yml::Value {
    serde_yml::Value::Mapping(serde_yml::Mapping::new())
}

/// Splits a leading `---`-delimited YAML frontmatter block off a
/// note's body. Obsidian notes only ever use this one frontmatter
/// style, so a full multi-format parser is more than this needs.
fn split_frontmatter(raw: &str) -> (serde_yml::Value, String) {
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

    let data = serde_yml::from_str::<serde_yml::Value>(&frontmatter_text).unwrap_or_else(|_| empty_mapping());
    let data = if data.is_mapping() { data } else { empty_mapping() };
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
    let summary = data.get("summary").and_then(|v| v.as_str()).map(str::to_string);

    ResolvedContent {
        title,
        summary,
        body: body.trim().to_string(),
    }
}

pub struct ObsidianResolverConfig {
    /// Directory the .hexen.yml file lives in — vault_root resolves against this.
    pub project_dir: PathBuf,
    /// content.vault_root from the project's top-level content block.
    pub vault_root: String,
}

pub struct ObsidianResolver {
    vault_dir: PathBuf,
}

impl ObsidianResolver {
    pub fn new(config: ObsidianResolverConfig) -> Self {
        Self {
            vault_dir: config.project_dir.join(config.vault_root),
        }
    }

    pub async fn resolve(&self, content: &ObsidianLocationContent) -> std::io::Result<ResolvedContent> {
        let file_path = self.vault_dir.join(&content.r#ref);
        let raw = tokio::fs::read_to_string(&file_path).await?;
        Ok(parse_obsidian_note(&raw, &content.r#ref))
    }
}
