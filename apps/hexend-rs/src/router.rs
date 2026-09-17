//! One-shot filesystem operations that don't have "current state" to
//! be authoritative over — everything that touches a project's live
//! state goes through the /ws session instead (see session.rs).
//! Port of apps/hexend/src/router.ts.

use std::path::Path;

use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;

use crate::mutations::create_minimal_project;
use crate::pb::hexen::v1::{project_content, InlineProjectContent, OpenedProjectData, ProjectContent};
use crate::project_io::{open_project, save_project};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    path: String,
    title: String,
    default_location_id: String,
    content: Option<ProjectContent>,
}

pub async fn create_project(Json(body): Json<CreateProjectRequest>) -> Response {
    let path = Path::new(&body.path);
    if path.exists() {
        return (StatusCode::BAD_REQUEST, format!("\"{}\" already exists", body.path)).into_response();
    }
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    if !dir.exists() {
        return (
            StatusCode::BAD_REQUEST,
            format!("Directory \"{}\" doesn't exist", dir.display()),
        )
            .into_response();
    }

    let content = body.content.unwrap_or(ProjectContent {
        kind: Some(project_content::Kind::Inline(InlineProjectContent {})),
    });
    let project = create_minimal_project(body.title, body.default_location_id, content);

    if let Err(err) = save_project(path, &project).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
    }

    match open_project(path).await {
        Ok(opened) => Json(OpenedProjectData {
            project: Some(opened.project),
            warnings: opened.warnings,
            resolved_content: opened.resolved_content,
            resolve_errors: opened.resolve_errors,
        })
        .into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct ListDirectoryQuery {
    path: Option<String>,
}

pub async fn list_directory(Query(q): Query<ListDirectoryQuery>) -> Response {
    match crate::browse::list_directory(q.path.as_deref()).await {
        Ok(listing) => Json(listing).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}
