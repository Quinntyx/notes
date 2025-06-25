use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn temp_dir() -> PathBuf {
    let mut dir = env::temp_dir();
    dir.push(format!("cli_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn cli_new_and_show() {
    let dir = temp_dir();
    env::set_current_dir(&dir).unwrap();
    let exe = env!("CARGO_BIN_EXE_notes");

    let output = Command::new(exe)
        .args(["new", "MyCliNote"])
        .output()
        .unwrap();
    assert!(output.status.success());

    let output = Command::new(exe)
        .args(["show", "MyCliNote"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--- MyCliNote ---"));
}
