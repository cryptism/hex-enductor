//! Transport-agnostic operations on a hexen project — port of
//! packages/project-ops, packages/content-obsidian's note parser, and
//! packages/hexen-schema's parse/serialize/validate. No file or network
//! I/O lives here: hexend wraps it with tokio::fs and a WS session, the
//! editor's local-folder storage with the File System Access API.
//!
//! Everything reads and writes the one YAML dialect: hexen-proto's
//! pbjson mapping (oneofs as `{ <field>: value }`).

pub mod commands;
pub mod content;
pub mod document;
pub mod history;
pub mod image_size;
pub mod mutations;

pub use hexen_proto::hexen::v1 as pb;
