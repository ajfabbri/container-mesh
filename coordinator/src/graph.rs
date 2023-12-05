use common::types::*;
use std::collections::{HashMap, VecDeque, HashSet};

// Connection graph generation. We represent the graphs as directed to indicate which side of a
// connection is the actie side: An edge from u to v,  u -> v, indicates that u calls connect(),
// whereas v does listen().
// From a graph theory and connectivity perspective, though, the edges are undirected: This means
// that the graph V = {a, b} with only the edge a -> b is considered a complete graph, for example.

#[derive(Debug, Clone)]
pub struct PeerEntry {
    id: PeerId,
    neighbors: HashSet<PeerId>,
}

impl PeerEntry {
    fn new(id: PeerId) -> Self {
        PeerEntry {
            id,
            neighbors: HashSet::new(),
        }
    }

    fn degree(&self) -> usize {
        self.neighbors.len()
    }
}

pub fn complete_graph(peers: Vec<PeerId>) -> PeerGraph {
    let mut graph: PeerGraph = HashMap::new();
    // Add vertices to graph one at a time, adding edges to each vertex already in the graph.
    // Base case: one vertex: trivially complete.
    // Inductive step: assume graph G is complete. Construct G' by adding v to G, and edges to
    // each vertex u in G. There are no two vertices (m, n) in G' which do not share an edge;
    // there werent any in G, and v is connected to all vertices in G, thus G' is complete.
    for v in peers {
        let mut edges_from_v = HashSet::new();
        for (u, _) in graph.iter_mut() {
            // add edges from to all vertices in G
            edges_from_v.insert(u.clone());
        }
        graph.insert(v, edges_from_v);
    }
    graph
}

// Create a directed graph of peers with maximum vertex degree specified
// Note: this makes a singly-connected graph, which may not be indicative of real-world mesh
// networks.
pub fn spanning_tree(mut peers: Vec<PeerId>, max_degree: usize) -> PeerGraph {
    let mut perimeter = VecDeque::new();
    let mut graph: PeerGraph = HashMap::new();
    if peers.len() == 0 {
        return graph;
    }
    peers.sort_by(|a, b| b.cmp(a));
    let root = peers.pop().unwrap();
    println!("_graph: root: {}", root);
    perimeter.push_back(PeerEntry::new(root));
    while let Some(mut p) = perimeter.pop_front() {
        while p.degree() < max_degree {
            let v = peers.pop();
            match v {
                Some(v) => {
                    p.neighbors.insert(v.clone());
                    perimeter.push_back(PeerEntry::new(v));
                }
                None => {
                    break;
                }
            }
        }
        graph.insert(p.id, p.neighbors);
    }
    graph
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_complete_graph() {
        let peers = to_peer_ids_vec(0..10);
        let graph = complete_graph(peers.clone());
        assert_eq!(graph.len(), 10);
        for (u, _) in &graph {
            for (v, _) in &graph {
                assert!(u == v || graph.get(u).unwrap().contains(v) || graph.get(v).unwrap().contains(u));
            }
        }
    }

    fn to_peer_ids(ids: std::ops::Range<usize>) -> HashSet<PeerId> {
        HashSet::from_iter(to_peer_ids_vec(ids))
    }

    fn to_peer_ids_vec(ids: std::ops::Range<usize>) -> Vec<PeerId> {
        ids.into_iter().map(|x| x.to_string()).collect()
    }

    #[test]
    pub fn test_spanning_tree() {
        let peers = to_peer_ids_vec(0..10);
        let graph = spanning_tree(peers.clone(), 3);
        println!("graph: {:?}", graph);
        let mut unconnected: HashSet<PeerId> = peers.into_iter().collect();
        for (k, v) in graph.iter() {
            assert!(v.len() <= 3);
            let uv = unconnected.take(k);
            assert!(uv.is_some());
        }
        assert_eq!(unconnected.len(), 0);
        assert_eq!(graph.get("0").unwrap(), &to_peer_ids(1..4));
        assert_eq!(graph.get("1").unwrap(), &to_peer_ids(4..7));
        assert_eq!(graph.get("2").unwrap(), &to_peer_ids(7..10));
        for i in 3..10 {
            assert_eq!(graph.get(&i.to_string()).unwrap(), &HashSet::<PeerId>::new());
        }
    }
}
