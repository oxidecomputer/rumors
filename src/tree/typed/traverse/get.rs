use super::*;

/// Perform a batch lookup of paths in the tree, returning the leaf at each
/// path (or `None` if no leaf exists there).
///
/// Results are returned in the same order as the input paths.
pub fn get<P, H: Get>(node: Option<&Node<P, H>>, paths: Vec<Path<H>>) -> Vec<Bytes>
where
    P: Clone + Hash + Eq,
{
    Get::get(node, paths)
}

pub trait Get: Height {
    fn get<P>(node: Option<&Node<P, Self>>, paths: Vec<Path<Self>>) -> Vec<Bytes>
    where
        P: Clone + Hash + Eq;
}

impl<H: Get> Get for S<H>
where
    S<H>: Height,
{
    fn get<P>(node: Option<&Node<P, Self>>, paths: Vec<Path<Self>>) -> Vec<Bytes>
    where
        P: Clone + Hash + Eq,
    {
        let Some(node) = node else {
            return Vec::new();
        };

        // Group the paths by their first element
        let by_radix = paths
            .into_iter()
            .map(|path| {
                let (child, path) = path.pop();
                (child, path)
            })
            .sorted_by_key(|(child, _)| *child)
            .chunk_by(|(child, _)| *child);

        // Decompose the node into its children
        let children = node.clone().into_children();

        // Recursively look up each radix group in the corresponding child
        by_radix
            .into_iter()
            .flat_map(|(radix, group)| {
                let child_paths: Vec<_> = group.map(|(_, path)| path).collect();
                Get::get(children.get(&radix), child_paths)
            })
            .collect()
    }
}

impl Get for Z {
    fn get<P>(node: Option<&Node<P, Self>>, paths: Vec<Path<Self>>) -> Vec<Bytes>
    where
        P: Clone + Hash + Eq,
    {
        let Some(node) = node else {
            return Vec::new();
        };

        let leaf = node.as_leaf().clone();
        if paths.is_empty() {
            vec![]
        } else {
            vec![leaf.value.clone()]
        }
    }
}
