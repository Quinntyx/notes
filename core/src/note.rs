use once_cell::sync::OnceCell;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
// Intentionally removed: use std::path::Path;

pub const NOTES_DIR: &str = "notes";

static VAULT_DIR: OnceCell<PathBuf> = OnceCell::new();

pub fn set_vault_dir<P: Into<PathBuf>>(dir: P) {
    let _ = VAULT_DIR.set(dir.into());
}

pub fn vault_dir() -> PathBuf {
    VAULT_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| PathBuf::from(NOTES_DIR))
}

#[derive(Debug, Clone)]
pub struct Note {
    pub title: String,
    pub content: String,
    pub aliases: Vec<String>,
    pub path: PathBuf,
}

impl Note {
    pub fn new(title: String, content: String, aliases: Option<Vec<String>>) -> Self {
        let mut path = vault_dir();
        path.push(format!("{}.md", title));
        Note {
            title,
            content,
            aliases: aliases.unwrap_or_else(Vec::new),
            path,
        }
    }

    pub fn save(&self) -> io::Result<()> {
        fs::create_dir_all(vault_dir())?;
        let mut file = fs::File::create(&self.path)?;
        file.write_all(self.content.as_bytes())?;
        Ok(())
    }

    pub fn load(path: &PathBuf) -> io::Result<Self> {
        if !path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Note file not found",
            ));
        }
        let content = fs::read_to_string(path)?;
        let title = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(String::from)
            .unwrap_or_else(|| "Untitled".to_string());
        Ok(Note {
            title,
            content,
            aliases: Vec::new(),
            path: path.clone(),
        })
    }

    pub fn path_from_title(title: &str) -> PathBuf {
        let mut path = vault_dir();
        path.push(format!("{}.md", title));
        path
    }
}
