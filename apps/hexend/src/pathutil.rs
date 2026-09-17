//! path.resolve-equivalent helpers: lexical join + normalization, no
//! filesystem access and no existence requirement — used everywhere a
//! request path gets checked against a directory it must not escape.

use std::path::{Path, PathBuf};

use path_clean::PathClean;

pub fn resolve(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf().clean()
    } else {
        base.join(path).clean()
    }
}

pub fn resolve_cwd(path: &Path) -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
    resolve(&cwd, path)
}
