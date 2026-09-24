//! Iterative cycle detection over resolved parent maps.
//!
//! No recursion: a chain of 100k nodes cannot overflow the stack (work order
//! section 41). Cycles are reported, never silently broken: deleting an edge
//! would fabricate certainty the file does not contain.

use std::collections::{BTreeMap, BTreeSet};

use crate::graph::NodeId;

/// Detect cycles in a parent map (`child -> parent`).
///
/// Returns one group per cycle, deterministic: groups are ordered by their
/// first member as discovered by sorted iteration, and each group starts at
/// the entry node that closed the cycle. Nodes not in a cycle are not
/// reported.
pub fn detect_cycles(parents: &BTreeMap<&str, &str>) -> Vec<Vec<NodeId>> {
    let mut state: BTreeMap<&str, u8> = BTreeMap::new(); // 0 unknown, 1 done, 2 on stack
    let mut groups: Vec<Vec<NodeId>> = Vec::new();

    for start in parents.keys() {
        if state.get(start).copied().unwrap_or(0) != 0 {
            continue;
        }
        let mut path: Vec<&str> = Vec::new();
        let mut current: Option<&str> = Some(start);
        while let Some(node) = current {
            match state.get(node).copied().unwrap_or(0) {
                0 => {
                    state.insert(node, 2);
                    path.push(node);
                    current = parents.get(node).copied();
                }
                2 => {
                    if let Some(position) = path.iter().position(|value| *value == node) {
                        let group: Vec<NodeId> = path
                            .iter()
                            .skip(position)
                            .map(|value| NodeId::new(*value))
                            .collect();
                        groups.push(group);
                    }
                    break;
                }
                _ => break,
            }
        }
        for node in path {
            state.insert(node, 1);
        }
    }

    groups.sort_by(|left, right| left.first().cmp(&right.first()));
    let mut seen: BTreeSet<Vec<String>> = BTreeSet::new();
    let mut out: Vec<Vec<NodeId>> = Vec::new();
    for group in groups {
        let mut key: Vec<String> = group.iter().map(|node| node.0.clone()).collect();
        key.sort();
        if seen.insert(key) {
            out.push(group);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: &[(&'static str, &'static str)]) -> BTreeMap<&'static str, &'static str> {
        entries.iter().copied().collect()
    }

    #[test]
    fn detects_plain_cycle() {
        let parents = map(&[("a", "b"), ("b", "c"), ("c", "b")]);
        let groups = detect_cycles(&parents);
        assert_eq!(groups.len(), 1);
        let ids: Vec<&str> = groups[0].iter().map(|node| node.as_str()).collect();
        assert!(ids == ["b", "c"] || ids == ["c", "b"], "{ids:?}");
    }

    #[test]
    fn no_false_positives_on_trees() {
        let parents = map(&[("a", "b"), ("b", "c")]);
        assert!(detect_cycles(&parents).is_empty());
    }

    #[test]
    fn detects_self_cycle_and_long_chain_safely() {
        // Self edges normally get filtered before this stage; the detector
        // still reports them instead of hanging.
        let parents = map(&[("a", "a")]);
        assert_eq!(detect_cycles(&parents).len(), 1);

        // 200k-node chain: iterative traversal must finish without stack
        // overflow.
        let nodes: Vec<(String, String)> = (0..200_000)
            .map(|index| (format!("n{index}"), format!("n{}", index + 1)))
            .collect();
        let parents: BTreeMap<&str, &str> = nodes
            .iter()
            .map(|(child, parent)| (child.as_str(), parent.as_str()))
            .collect();
        assert!(detect_cycles(&parents).is_empty());
    }
}
