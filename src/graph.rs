use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use crate::note::NOTES_DIR;

#[derive(Debug)]
pub struct Graph {
    pub nodes: Vec<Node>,
    /// Directed edges using node indices (from -> to)
    pub edges: Vec<(usize, usize)>,
}

#[derive(Debug)]
pub struct Node {
    pub name: String,
    pub path: PathBuf,
}

fn canonicalize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect::<String>()
}

pub fn build_graph() -> Graph {
    let mut nodes = Vec::new();
    let mut index_map: HashMap<String, usize> = HashMap::new();

    if let Ok(entries) = fs::read_dir(NOTES_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let canonical = canonicalize(stem);
                if !index_map.contains_key(&canonical) {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default()
                        .to_string();
                    let idx = nodes.len();
                    nodes.push(Node {
                        name,
                        path: path.clone(),
                    });
                    index_map.insert(canonical, idx);
                }
            }
        }
    }

    // store directed edges using indices
    let mut edges: HashSet<(usize, usize)> = HashSet::new();

    for (canon, &i) in &index_map {
        let path = &nodes[i].path;
        if let Ok(content) = fs::read_to_string(path) {
            let text = canonicalize(&content);
            for (other_canon, &j) in &index_map {
                if canon == other_canon {
                    continue;
                }
                if text.contains(other_canon) {
                    edges.insert((i, j));
                }
            }
        }
    }

    Graph {
        nodes,
        edges: edges.into_iter().collect(),
    }
}
