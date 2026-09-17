//! Same trust model as the rest of the server — no auth, no jail. This
//! just gives the editor something nicer than "paste an absolute path"
//! to open a project with. Port of apps/hexend/src/browse.ts.

use std::path::Path;

use serde::Serialize;

use crate::pathutil::resolve_cwd;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    pub name: String,
    pub is_directory: bool,
    pub is_project: bool,
}

#[derive(Serialize)]
pub struct DirectoryListing {
    pub path: String,
    pub parent: Option<String>,
    pub entries: Vec<DirEntry>,
}

pub async fn list_directory(path: Option<&str>) -> std::io::Result<DirectoryListing> {
    let target = match path.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => resolve_cwd(Path::new(p)),
        None => dirs_home(),
    };

    let mut read_dir = tokio::fs::read_dir(&target).await?;
    let mut entries = Vec::new();
    while let Some(entry) = read_dir.next_entry().await? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let is_directory = entry.file_type().await?.is_dir();
        let is_project = name.ends_with(".hexen.yml");
        if !is_directory && !is_project {
            continue;
        }
        entries.push(DirEntry {
            name,
            is_directory,
            is_project,
        });
    }
    entries.sort_by(|a, b| match (a.is_directory, b.is_directory) {
        (true, true) | (false, false) => a.name.cmp(&b.name),
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
    });

    let parent = target.parent().map(|p| p.to_string_lossy().into_owned());
    let target_str = target.to_string_lossy().into_owned();
    let parent = parent.filter(|p| *p != target_str);

    Ok(DirectoryListing {
        path: target_str,
        parent,
        entries,
    })
}

fn dirs_home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/"))
}
