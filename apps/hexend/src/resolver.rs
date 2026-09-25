//! Obsidian content resolution from disk. The note parsing itself is
//! crates/project-ops's `content`, shared with the editor's local-folder
//! storage.

use std::path::PathBuf;

use crate::pb::hexen::v1::{ObsidianLocationContent, ResolvedContent};
pub use project_ops::content::{parse_obsidian_note, resolve_inline_content};

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
