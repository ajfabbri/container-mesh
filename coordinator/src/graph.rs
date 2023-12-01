use common::types::*;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct PeerEntry {
    id: PeerId,
    neighbors: Vec<PeerId>,
}

impl PeerEntry {
    fn new(id: PeerId) -> Self {
        PeerEntry {
            id,
            neighbors: Vec::new(),
        }
    }

    fn degree(&self) -> usize {
        self.neighbors.len()
    }
}

// Create a directed graph of peers with maximum vertex degree specified
// TODO this makes a singly-connected graph, we probably want a different algo.
pub fn make_connection_graph(mut peers: Vec<PeerId>, max_degree: usize) -> PeerGraph {
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
                    p.neighbors.push(v.clone());
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

    fn to_peer_ids(ids: std::ops::Range<usize>) -> Vec<PeerId> {
        ids.into_iter().map(|x| x.to_string()).collect()
    }

    #[test]
    pub fn test_make_connection_graph() {
        let peers: Vec<PeerId> = to_peer_ids(0..10);
        let graph = make_connection_graph(peers.clone(), 3);
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
            assert_eq!(graph.get(&i.to_string()).unwrap(), &Vec::<PeerId>::new());
        }
    }
}
