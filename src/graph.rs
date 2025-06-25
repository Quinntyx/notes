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
    /// Number of links connected to this node (in or out)
    pub links: usize,
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
                        links: 0,
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

    // count links for each node
    let mut link_counts = vec![0usize; nodes.len()];
    for &(a, b) in &edges {
        if a < link_counts.len() {
            link_counts[a] += 1;
        }
        if b < link_counts.len() {
            link_counts[b] += 1;
        }
    }
    for (node, count) in nodes.iter_mut().zip(link_counts) {
        node.links = count;
    }

    Graph {
        nodes,
        edges: edges.into_iter().collect(),
    }
}

#[derive(Debug)]
pub struct GraphData {
    pub graph: Graph,
    canonical: Vec<String>,
    contents: Vec<String>,
}

fn recompute_edges(data: &mut GraphData) {
    let n = data.graph.nodes.len();
    let mut edges: HashSet<(usize, usize)> = HashSet::new();
    for i in 0..n {
        let text = &data.contents[i];
        for j in 0..n {
            if i == j {
                continue;
            }
            if text.contains(&data.canonical[j]) {
                edges.insert((i, j));
            }
        }
    }

    let mut link_counts = vec![0usize; n];
    for &(a, b) in &edges {
        if a < n {
            link_counts[a] += 1;
        }
        if b < n {
            link_counts[b] += 1;
        }
    }
    for (node, count) in data.graph.nodes.iter_mut().zip(link_counts) {
        node.links = count;
    }
    data.graph.edges = edges.into_iter().collect();
}

pub fn load_graph_data() -> GraphData {
    let mut nodes = Vec::new();
    let mut canonical = Vec::new();
    let mut contents = Vec::new();

    if let Ok(entries) = fs::read_dir(NOTES_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let (Some(stem), Some(filename)) = (
                path.file_stem().and_then(|s| s.to_str()),
                path.file_name().and_then(|s| s.to_str()),
            ) {
                let canon = canonicalize(stem);
                if !canonical.contains(&canon) {
                    nodes.push(Node {
                        name: filename.to_string(),
                        path: path.clone(),
                        links: 0,
                    });
                    canonical.push(canon);
                    let text = fs::read_to_string(&path).unwrap_or_default();
                    contents.push(canonicalize(&text));
                }
            }
        }
    }

    let mut data = GraphData {
        graph: Graph {
            nodes,
            edges: Vec::new(),
        },
        canonical,
        contents,
    };
    recompute_edges(&mut data);
    data
}

pub fn update_open_notes(data: &mut GraphData, open_notes: &[String]) {
    for name in open_notes {
        if let Some(stem) = PathBuf::from(name).file_stem().and_then(|s| s.to_str()) {
            let canon = canonicalize(stem);
            if let Some(idx) = data.canonical.iter().position(|c| c == &canon) {
                let path = &data.graph.nodes[idx].path;
                if let Ok(content) = fs::read_to_string(path) {
                    data.contents[idx] = canonicalize(&content);
                }
            }
        }
    }
    recompute_edges(data);
}
