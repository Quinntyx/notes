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

fn normalize(s: &str) -> String {
    let mut out = String::new();
    let mut in_space = false;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if in_space && !out.is_empty() {
                out.push(' ');
            }
            out.extend(c.to_lowercase());
            in_space = false;
        } else {
            in_space = true;
        }
    }
    out
}

fn canonicalize(s: &str) -> String {
    normalize(s).replace(' ', "")
}

fn is_boundary(text: &str, idx: usize) -> bool {
    idx == 0 || text.as_bytes()[idx - 1].is_ascii_whitespace()
}

fn is_end_boundary(text: &str, idx: usize) -> bool {
    idx == text.len() || text.as_bytes()[idx].is_ascii_whitespace()
}

struct Match {
    start: usize,
    end: usize,
    idx: usize,
}

fn find_unique_links(text: &str, _canon: &[String], names: &[String]) -> Vec<usize> {
    let mut matches = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let mut search_start = 0;
        while let Some(pos) = text[search_start..].find(name) {
            let start = search_start + pos;
            let end = start + name.len();
            if is_boundary(text, start) && is_end_boundary(text, end) {
                matches.push(Match { start, end, idx: i });
            }
            search_start = start + 1;
        }
    }

    let mut keep = vec![true; matches.len()];
    for i in 0..matches.len() {
        for j in 0..matches.len() {
            if i == j {
                continue;
            }
            let a = &matches[i];
            let b = &matches[j];
            if b.start <= a.start && b.end >= a.end && (b.end - b.start) > (a.end - a.start) {
                keep[i] = false;
                break;
            }
        }
    }

    let mut result = Vec::new();
    for (m, &k) in matches.iter().zip(&keep) {
        if k {
            result.push(m.idx);
        }
    }
    result.sort_unstable();
    result.dedup();
    result
}

pub fn build_graph() -> Graph {
    let mut nodes = Vec::new();
    let mut canonical = Vec::new();
    let mut normalized = Vec::new();
    let mut index_map: HashMap<String, usize> = HashMap::new();

    if let Ok(entries) = fs::read_dir(NOTES_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                let canon = canonicalize(stem);
                if !index_map.contains_key(&canon) {
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
                    index_map.insert(canon.clone(), idx);
                    canonical.push(canon);
                    normalized.push(normalize(stem));
                }
            }
        }
    }

    // store directed edges using indices
    let mut edges: HashSet<(usize, usize)> = HashSet::new();

    for i in 0..nodes.len() {
        let path = &nodes[i].path;
        if let Ok(content) = fs::read_to_string(path) {
            let text = normalize(&content);
            for j in find_unique_links(&text, &canonical, &normalized) {
                if i == j {
                    continue;
                }
                edges.insert((i, j));
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
    normalized: Vec<String>,
    contents: Vec<String>,
}

fn recompute_edges(data: &mut GraphData) {
    let n = data.graph.nodes.len();
    let mut edges: HashSet<(usize, usize)> = HashSet::new();
    for i in 0..n {
        let text = &data.contents[i];
        for j in find_unique_links(text, &data.canonical, &data.normalized) {
            if i == j {
                continue;
            }
            edges.insert((i, j));
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
    let mut normalized = Vec::new();
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
                    normalized.push(normalize(stem));
                    let text = fs::read_to_string(&path).unwrap_or_default();
                    contents.push(normalize(&text));
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
        normalized,
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
                    data.contents[idx] = normalize(&content);
                }
            }
        }
    }
    recompute_edges(data);
}

#[cfg(test)]
mod tests {
    use super::{canonicalize, find_unique_links, normalize};

    #[test]
    fn longest_match() {
        let names = vec![
            "nuclear power".to_string(),
            "nuclear power in iran".to_string(),
        ];
        let canonical: Vec<String> = names.iter().map(|s| canonicalize(s)).collect();
        let normalized: Vec<String> = names.iter().map(|s| normalize(s)).collect();
        let text = normalize("nuclear power in Iran");
        let links = find_unique_links(&text, &canonical, &normalized);
        assert_eq!(links, vec![1]);
    }

    #[test]
    fn partial_overlap() {
        let names = vec![
            "Power Generation Techniques".to_string(),
            "Nuclear Power Generation".to_string(),
        ];
        let canonical: Vec<String> = names.iter().map(|s| canonicalize(s)).collect();
        let normalized: Vec<String> = names.iter().map(|s| normalize(s)).collect();
        let text = normalize("nuclear power generation techniques");
        let mut links = find_unique_links(&text, &canonical, &normalized);
        links.sort();
        assert_eq!(links, vec![0, 1]);
    }

    #[test]
    fn no_substring_match() {
        let names = vec!["note".to_string(), "another note".to_string()];
        let canonical: Vec<String> = names.iter().map(|s| canonicalize(s)).collect();
        let normalized: Vec<String> = names.iter().map(|s| normalize(s)).collect();
        let text = normalize("newnote another note with spaces");
        let mut links = find_unique_links(&text, &canonical, &normalized);
        links.sort();
        assert_eq!(links, vec![1]);
    }
}
