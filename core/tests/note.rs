use notes_core::note::Note;
use std::env;
use std::fs;
use std::path::PathBuf;

fn temp_dir() -> PathBuf {
    let mut dir = env::temp_dir();
    dir.push(format!("notes_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn save_and_load_note() {
    let dir = temp_dir();
    env::set_current_dir(&dir).unwrap();

    let note = Note::new("Test".to_string(), "content".to_string(), None);
    note.save().unwrap();

    let loaded = Note::load(&note.path).unwrap();
    assert_eq!(loaded.title, "Test");
    assert_eq!(loaded.content, "content");
}

#[test]
fn path_from_title() {
    let path = Note::path_from_title("My Note");
    assert!(path.ends_with("notes/My Note.md"));
}
