use notes_core::graph::build_graph;
use notes_core::note::{NOTES_DIR, Note};
use std::env;
use std::fs;
use std::path::PathBuf;

fn setup() -> PathBuf {
    let mut dir = env::temp_dir();
    dir.push(format!("graph_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    env::set_current_dir(&dir).unwrap();
    fs::create_dir_all(NOTES_DIR).unwrap();
    dir
}

#[test]
fn build_simple_graph() {
    let _dir = setup();
    let a = Note::new("A".to_string(), "links to B".to_string(), None);
    a.save().unwrap();
    let b = Note::new("B".to_string(), "".to_string(), None);
    b.save().unwrap();

    let graph = build_graph();
    assert_eq!(graph.nodes.len(), 2);
    // edges stored as pairs of indices: 0->1 or 1->0 etc
    assert_eq!(graph.edges.len(), 1);
}
