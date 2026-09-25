pub mod browse;
pub mod pathutil;
pub mod pb;
pub mod project_io;
pub mod resolver;
pub mod router;
pub mod server;
pub mod session;

// The pure, transport-agnostic half — shared with the editor's
// local-folder storage — lives in crates/project-ops.
pub use project_ops::{commands, image_size, mutations};
