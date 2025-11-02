// Work broker: dependency tracking and work dispatch

use crate::tree::DirectoryTree;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Broker {
    /// Map: directory -> number of children still pending deletion
    child_counts: Mutex<HashMap<PathBuf, usize>>,
    /// Map: directory -> parent directory
    parent_map: Mutex<HashMap<PathBuf, PathBuf>>,
    /// Channel sender for pushing work to workers
    work_tx: Sender<PathBuf>,
}

impl Broker {
    /// Create broker from DirectoryTree, returns (Broker, Receiver for workers)
    pub fn new(tree: DirectoryTree) -> (Self, Receiver<PathBuf>) {
        let (tx, rx) = unbounded();

        let mut child_counts = HashMap::new();
        let mut parent_map = HashMap::new();

        // Build parent map and initialize child counts
        for (parent, children) in &tree.children {
            child_counts.insert(parent.clone(), children.len());
            for child in children {
                parent_map.insert(child.clone(), parent.clone());
            }
        }

        let broker = Self {
            child_counts: Mutex::new(child_counts),
            parent_map: Mutex::new(parent_map),
            work_tx: tx,
        };

        // Push all initial leaves to work queue
        for leaf in tree.leaves {
            broker.work_tx.send(leaf).ok();
        }

        (broker, rx)
    }

    /// Mark directory as deleted, update dependency graph, push newly-available parents
    pub fn mark_complete(&self, dir: PathBuf) {
        let parent = {
            let parent_map = self.parent_map.lock().unwrap();
            parent_map.get(&dir).cloned()
        };

        if let Some(parent_path) = parent {
            let mut counts = self.child_counts.lock().unwrap();

            if let Some(count) = counts.get_mut(&parent_path) {
                *count -= 1;

                // If parent now has no pending children, it becomes a leaf
                if *count == 0 {
                    counts.remove(&parent_path);
                    drop(counts); // Release lock before sending
                    self.work_tx.send(parent_path).ok();
                }
            }
        }
    }

    /// Get total pending directories (for monitoring)
    pub fn pending_count(&self) -> usize {
        self.child_counts.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::DirectoryTree;
    use std::collections::HashMap;

    #[test]
    fn test_broker_simple_tree() {
        // Create tree:
        //   root
        //     a (leaf)
        //     b (leaf)

        let root = PathBuf::from("/root");
        let a = PathBuf::from("/root/a");
        let b = PathBuf::from("/root/b");

        let mut tree = DirectoryTree::new();
        tree.dirs = vec![root.clone(), a.clone(), b.clone()];
        tree.leaves = vec![a.clone(), b.clone()];

        let mut children = HashMap::new();
        children.insert(root.clone(), vec![a.clone(), b.clone()]);
        tree.children = children;

        let (broker, rx) = Broker::new(tree);

        // Should immediately dispatch both leaves
        assert_eq!(rx.recv().unwrap(), a);
        assert_eq!(rx.recv().unwrap(), b);

        // Mark a complete
        broker.mark_complete(a);

        // Root still has 1 child pending
        assert!(rx.try_recv().is_err());

        // Mark b complete
        broker.mark_complete(b);

        // Now root should be dispatched
        assert_eq!(rx.recv().unwrap(), root);
    }

    #[test]
    fn test_broker_deep_tree() {
        // Create tree: root -> a -> b -> c
        let root = PathBuf::from("/root");
        let a = PathBuf::from("/root/a");
        let b = PathBuf::from("/root/a/b");
        let c = PathBuf::from("/root/a/b/c");

        let mut tree = DirectoryTree::new();
        tree.dirs = vec![root.clone(), a.clone(), b.clone(), c.clone()];
        tree.leaves = vec![c.clone()];

        let mut children = HashMap::new();
        children.insert(root.clone(), vec![a.clone()]);
        children.insert(a.clone(), vec![b.clone()]);
        children.insert(b.clone(), vec![c.clone()]);
        tree.children = children;

        let (broker, rx) = Broker::new(tree);

        // Only leaf c dispatched initially
        assert_eq!(rx.recv().unwrap(), c);

        // Mark c complete -> b becomes available
        broker.mark_complete(c.clone());
        assert_eq!(rx.recv().unwrap(), b);

        // Mark b complete -> a becomes available
        broker.mark_complete(b.clone());
        assert_eq!(rx.recv().unwrap(), a);

        // Mark a complete -> root becomes available
        broker.mark_complete(a.clone());
        assert_eq!(rx.recv().unwrap(), root);
    }

    #[test]
    fn test_broker_pending_count() {
        let root = PathBuf::from("/root");
        let a = PathBuf::from("/root/a");
        let b = PathBuf::from("/root/b");

        let mut tree = DirectoryTree::new();
        tree.dirs = vec![root.clone(), a.clone(), b.clone()];
        tree.leaves = vec![a.clone(), b.clone()];

        let mut children = HashMap::new();
        children.insert(root.clone(), vec![a.clone(), b.clone()]);
        tree.children = children;

        let (broker, _rx) = Broker::new(tree);

        // Root has 2 children pending
        assert_eq!(broker.pending_count(), 1);

        broker.mark_complete(a);
        assert_eq!(broker.pending_count(), 1);

        broker.mark_complete(b);
        // Root now has 0 children, should be removed from counts
        assert_eq!(broker.pending_count(), 0);
    }
}
