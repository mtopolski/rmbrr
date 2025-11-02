// Directory tree discovery and dependency graph construction

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct DirectoryTree {
    /// All directories in the tree
    pub dirs: Vec<PathBuf>,
    /// Map of directory -> list of child directories
    pub children: HashMap<PathBuf, Vec<PathBuf>>,
    /// Directories with no subdirectories (initial leaves)
    pub leaves: Vec<PathBuf>,
}

impl DirectoryTree {
    pub fn new() -> Self {
        Self {
            dirs: Vec::new(),
            children: HashMap::new(),
            leaves: Vec::new(),
        }
    }
}

pub fn discover_tree(root: &Path) -> io::Result<DirectoryTree> {
    let mut tree = DirectoryTree::new();
    let mut all_dirs = HashSet::new();
    let mut has_children = HashSet::new();

    // Recursive scan
    scan_recursive(root, &mut all_dirs, &mut tree.children, &mut has_children)?;

    // Convert to sorted vec
    tree.dirs = all_dirs.iter().cloned().collect();
    tree.dirs.sort();

    // Identify leaves (dirs not in has_children set)
    for dir in &tree.dirs {
        if !has_children.contains(dir) {
            tree.leaves.push(dir.clone());
        }
    }

    Ok(tree)
}

fn scan_recursive(
    dir: &Path,
    all_dirs: &mut HashSet<PathBuf>,
    children_map: &mut HashMap<PathBuf, Vec<PathBuf>>,
    has_children: &mut HashSet<PathBuf>,
) -> io::Result<()> {
    all_dirs.insert(dir.to_path_buf());

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            // Skip directories we can't read (permissions, etc.)
            eprintln!("Warning: Cannot read {}: {}", dir.display(), e);
            return Ok(());
        }
    };

    let mut child_dirs = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        let is_dir = match entry.file_type() {
            Ok(ft) => ft.is_dir(),
            Err(_) => continue,
        };

        if is_dir {
            child_dirs.push(path.clone());
            scan_recursive(&path, all_dirs, children_map, has_children)?;
        }
    }

    if !child_dirs.is_empty() {
        has_children.insert(dir.to_path_buf());
        children_map.insert(dir.to_path_buf(), child_dirs);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_tree(base: &Path) -> io::Result<()> {
        // Structure:
        // base/
        //   a/
        //     a1/
        //     a2/
        //   b/
        //   c/
        //     c1/

        fs::create_dir_all(base.join("a/a1"))?;
        fs::create_dir_all(base.join("a/a2"))?;
        fs::create_dir(base.join("b"))?;
        fs::create_dir_all(base.join("c/c1"))?;

        Ok(())
    }

    #[test]
    fn test_discover_tree() {
        let temp = std::env::temp_dir().join("win_rmdir_tree_test");
        let _ = fs::remove_dir_all(&temp);

        create_test_tree(&temp).unwrap();

        let tree = discover_tree(&temp).unwrap();

        // Should find 7 directories: base, a, a1, a2, b, c, c1
        assert_eq!(tree.dirs.len(), 7);

        // Leaves should be: a1, a2, b, c1 (4 total)
        assert_eq!(tree.leaves.len(), 4);

        // Verify leaves don't have children
        for leaf in &tree.leaves {
            assert!(!tree.children.contains_key(leaf));
        }

        // Cleanup
        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_single_dir() {
        let temp = std::env::temp_dir().join("win_rmdir_single_test");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir(&temp).unwrap();

        let tree = discover_tree(&temp).unwrap();

        // Just the root directory
        assert_eq!(tree.dirs.len(), 1);
        assert_eq!(tree.leaves.len(), 1);

        fs::remove_dir(&temp).ok();
    }

    #[test]
    fn test_deep_nesting() {
        let temp = std::env::temp_dir().join("win_rmdir_deep_test");
        let _ = fs::remove_dir_all(&temp);

        // Create 10 levels deep
        let mut path = temp.clone();
        for i in 0..10 {
            path = path.join(format!("level{}", i));
        }
        fs::create_dir_all(&path).unwrap();

        let tree = discover_tree(&temp).unwrap();

        // Should have 11 directories (root + 10 levels)
        assert_eq!(tree.dirs.len(), 11);

        // Only the deepest is a leaf
        assert_eq!(tree.leaves.len(), 1);

        fs::remove_dir_all(&temp).ok();
    }
}
