use agent_desktop_core::{AdapterError, Deadline, ErrorCode};

use super::light::{Light, Nodes};
use crate::tree::AtspiElement;

const MAX_NODES: usize = 8_000;

struct Entry {
    light: Light,
    parent: Option<usize>,
}

fn chain(arena: &[Entry], mut index: usize) -> Vec<Light> {
    let mut path = Vec::new();
    loop {
        path.push(arena[index].light.clone());
        match arena[index].parent {
            Some(parent) => index = parent,
            None => break,
        }
    }
    path.reverse();
    path
}

/// What a search looks for: nodes `prune` accepts are neither matched nor
/// entered, `candidate` is the cheap test, and `accept` sees the ancestors
/// root first for the rules that need them.
pub(crate) struct Query<'a> {
    pub(crate) prune: &'a dyn Fn(&Light) -> bool,
    pub(crate) candidate: &'a dyn Fn(&Light) -> bool,
    pub(crate) accept: &'a dyn Fn(&[Light], &Light) -> bool,
}

/// Breadth-first search from `root`. The result is the chain from the root
/// to the first accepted node, root first and the match last. A node that
/// vanishes while the search runs is skipped; the deadline ends the search.
pub(crate) fn find<N: Nodes>(
    nodes: &N,
    root: &AtspiElement,
    deadline: Deadline,
    query: &Query<'_>,
) -> Result<Option<Vec<Light>>, AdapterError> {
    let mut arena: Vec<Entry> = Vec::new();
    let mut frontier: Vec<(AtspiElement, Option<usize>)> = vec![(root.clone(), None)];
    while !frontier.is_empty() {
        if deadline.is_expired() {
            return Err(deadline.timeout_error());
        }
        let elements: Vec<AtspiElement> = frontier
            .iter()
            .map(|(element, _)| element.clone())
            .collect();
        let reads = nodes.read_level(&elements);
        let mut next = Vec::new();
        for ((_, parent), read) in frontier.drain(..).zip(reads) {
            let light = match read {
                Ok(light) => light,
                Err(error) if error.code == ErrorCode::ElementNotFound => continue,
                Err(error) => return Err(error),
            };
            if (query.prune)(&light) {
                continue;
            }
            let index = arena.len();
            let children = light.children.clone();
            let candidate = (query.candidate)(&light);
            arena.push(Entry { light, parent });
            if candidate {
                let path = chain(&arena, index);
                let matched = &arena[index].light;
                let above = &path[..path.len() - 1];
                if (query.accept)(above, matched) {
                    return Ok(Some(path));
                }
            }
            next.extend(children.into_iter().map(|child| (child, Some(index))));
        }
        if arena.len() > MAX_NODES {
            return Ok(None);
        }
        frontier = next;
    }
    Ok(None)
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
