use std::path::PathBuf;

fn demo_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/demo/demo.hexen.yml")
}

#[tokio::test]
async fn opens_the_demo_project_end_to_end() {
    let path = demo_path();
    let opened = hexend_rs::project_io::open_project(&path).await.expect("open_project");

    assert_eq!(opened.project.title, "Demo Realm");
    assert_eq!(opened.project.default_location, "town");
    assert_eq!(opened.project.locations.len(), 3);
    assert!(opened.warnings.is_empty(), "unexpected warnings: {:?}", opened.warnings);
    assert!(
        opened.resolve_errors.is_empty(),
        "unexpected resolve errors: {:?}",
        opened.resolve_errors
    );

    let town = opened.resolved_content.get("town").expect("town resolves");
    assert!(!town.title.is_empty());

    let notice_board = opened
        .resolved_content
        .get("notice-board")
        .expect("notice-board resolves");
    assert_eq!(notice_board.title, "The Notice Board");
}

#[tokio::test]
async fn round_trips_through_yaml() {
    let path = demo_path();
    let yaml_text = tokio::fs::read_to_string(&path).await.unwrap();
    let parsed = hexend_rs::project_io::parse_hexen_project(&yaml_text).expect("parse");

    let reserialized = hexend_rs::project_io::serialize_hexen_project(&parsed.project).expect("serialize");
    let reparsed = hexend_rs::project_io::parse_hexen_project(&reserialized).expect("reparse");

    assert_eq!(parsed.project, reparsed.project);
}
