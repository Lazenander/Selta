//! Bounded, handle-pinned recursive file inventories.

use anyhow::{bail, Context, Result};

use crate::digest::{bytes_sha256, Digest};
use crate::path::{NodeKind, PinnedDirectory, PinnedFile, RepoPath};

/// Operational ceilings for one recursive inventory.
///
/// `max_depth` counts path components beneath the supplied root: entries in
/// the root have depth one. Every ceiling is inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InventoryLimits {
    pub(crate) max_depth: usize,
    pub(crate) max_entries_per_directory: usize,
    pub(crate) max_total_entries: usize,
    pub(crate) max_selected_files: usize,
    pub(crate) max_file_bytes: usize,
    pub(crate) max_selected_bytes: usize,
}

/// One selected regular file, observed and consumed through a retained handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InventoriedFile {
    pub(crate) path: RepoPath,
    pub(crate) bytes: Vec<u8>,
    pub(crate) bytes_sha256: Digest,
}

struct InventoryState {
    total_entries: usize,
    selected_files: usize,
    selected_bytes: usize,
    files: Vec<InventoriedFile>,
}

/// Recursively inventory selected regular files beneath a pinned directory.
///
/// `root_path` is the canonical repository path naming `root`; returned paths
/// include that prefix. The selector is evaluated only for regular files.
/// Every directory is enumerated exactly once, and every encountered entry is
/// required to be a regular file or directory even when the selector would
/// ignore its path. Traversal is depth-first in raw ASCII child-name order, so
/// selector effects and first failure are deterministic. An ignored regular
/// entry is still opened and bound to its enumerated stamp, but its bytes are
/// neither read nor charged to selected-byte limits.
pub(crate) fn inventory_recursive(
    root: &PinnedDirectory,
    root_path: &RepoPath,
    limits: InventoryLimits,
    mut select: impl FnMut(&RepoPath) -> bool,
) -> Result<Vec<InventoriedFile>> {
    let mut state = InventoryState {
        total_entries: 0,
        selected_files: 0,
        selected_bytes: 0,
        files: Vec::new(),
    };
    visit_directory(root, root_path, 0, limits, &mut select, &mut state)?;

    state
        .files
        .sort_by(|left, right| left.path.cmp(&right.path));
    Ok(state.files)
}

fn visit_directory(
    directory: &PinnedDirectory,
    directory_path: &RepoPath,
    directory_depth: usize,
    limits: InventoryLimits,
    select: &mut impl FnMut(&RepoPath) -> bool,
    state: &mut InventoryState,
) -> Result<()> {
    let entries = directory
        .entries(limits.max_entries_per_directory)
        .with_context(|| format!("cannot enumerate inventory directory {directory_path}"))?;
    state.total_entries = state
        .total_entries
        .checked_add(entries.len())
        .context("aggregate inventory entry count overflow")?;
    if state.total_entries > limits.max_total_entries {
        bail!(
            "inventory contains {} total entries; limit is {}",
            state.total_entries,
            limits.max_total_entries
        );
    }

    for entry in entries {
        let depth = directory_depth
            .checked_add(1)
            .context("inventory depth overflow")?;
        let path = directory_path
            .join_child(entry.name())
            .with_context(|| format!("cannot form inventory path below {directory_path}"))?;
        if depth > limits.max_depth {
            bail!(
                "inventory path {path} has depth {depth}; limit is {}",
                limits.max_depth
            );
        }

        match entry.kind() {
            NodeKind::Directory => {
                let child = entry
                    .open_directory()
                    .with_context(|| format!("cannot retain inventory directory {path}"))?;
                visit_directory(&child, &path, depth, limits, select, state)?;
            }
            NodeKind::Regular => {
                let file = entry
                    .open_regular()
                    .with_context(|| format!("cannot retain inventory file {path}"))?;
                if select(&path) {
                    select_file(file, path, limits, state)?;
                }
            }
            NodeKind::Symlink => bail!("inventory contains a symbolic link: {path}"),
            NodeKind::Other => bail!("inventory contains a special filesystem node: {path}"),
        }
    }
    Ok(())
}

fn select_file(
    file: PinnedFile,
    path: RepoPath,
    limits: InventoryLimits,
    state: &mut InventoryState,
) -> Result<()> {
    if state.selected_files == limits.max_selected_files {
        bail!(
            "inventory exceeds its selected-file ceiling of {}",
            limits.max_selected_files
        );
    }

    let file_bytes = usize::try_from(file.len())
        .with_context(|| format!("inventory file length exceeds usize: {path}"))?;
    if file_bytes > limits.max_file_bytes {
        bail!(
            "inventory file {path} is {file_bytes} bytes; limit is {}",
            limits.max_file_bytes
        );
    }
    let selected_bytes = state
        .selected_bytes
        .checked_add(file_bytes)
        .context("aggregate selected-byte count overflow")?;
    if selected_bytes > limits.max_selected_bytes {
        bail!(
            "inventory selected bytes would total {selected_bytes}; limit is {}",
            limits.max_selected_bytes
        );
    }

    let bytes = file
        .read_all_verified(path.as_str(), limits.max_file_bytes)
        .with_context(|| format!("cannot consume inventory file {path}"))?;
    state.selected_files += 1;
    state.selected_bytes = selected_bytes;
    state.files.push(InventoriedFile {
        bytes_sha256: bytes_sha256(&bytes),
        path,
        bytes,
    });
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;

    use super::*;
    use crate::path::Repository;

    const GENEROUS: InventoryLimits = InventoryLimits {
        max_depth: 8,
        max_entries_per_directory: 16,
        max_total_entries: 64,
        max_selected_files: 16,
        max_file_bytes: 1024,
        max_selected_bytes: 4096,
    };

    fn open_tree(root: &tempfile::TempDir) -> (PinnedDirectory, RepoPath) {
        let physical_root = fs::canonicalize(root.path()).unwrap();
        let repository = Repository::open(&physical_root).unwrap();
        let path: RepoPath = "tree".parse().unwrap();
        let directory = repository.open_directory(&path).unwrap();
        (directory, path)
    }

    fn fixture_with_two_selected_files() -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("tree")).unwrap();
        fs::create_dir(root.path().join("tree/d")).unwrap();
        fs::write(root.path().join("tree/one.json"), b"12").unwrap();
        fs::write(root.path().join("tree/d/two.json"), b"345").unwrap();
        root
    }

    #[test]
    fn selects_nested_json_and_returns_canonical_full_path_order() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("tree")).unwrap();
        fs::create_dir(root.path().join("tree/a")).unwrap();
        fs::write(root.path().join("tree/a/z.json"), b"nested").unwrap();
        fs::write(root.path().join("tree/a/ignored.txt"), b"ignored").unwrap();
        fs::write(root.path().join("tree/a0.json"), b"root").unwrap();
        fs::write(root.path().join("tree/readme"), b"ignored").unwrap();

        let (directory, path) = open_tree(&root);
        let mut selector_calls = Vec::new();
        let files = inventory_recursive(&directory, &path, GENEROUS, |candidate| {
            selector_calls.push(candidate.to_string());
            candidate.as_str().ends_with(".json")
        })
        .unwrap();

        assert_eq!(
            selector_calls,
            [
                "tree/a/ignored.txt",
                "tree/a/z.json",
                "tree/a0.json",
                "tree/readme",
            ]
        );
        assert_eq!(
            files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            ["tree/a/z.json", "tree/a0.json"]
        );
        assert_eq!(files[0].bytes, b"nested");
        assert_eq!(files[0].bytes_sha256, bytes_sha256(b"nested"));
        assert_eq!(files[1].bytes, b"root");
        assert_eq!(files[1].bytes_sha256, bytes_sha256(b"root"));
    }

    #[test]
    fn all_regular_selection_includes_extensionless_and_non_json_files() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("tree")).unwrap();
        fs::write(root.path().join("tree/Cargo.toml"), b"manifest").unwrap();
        fs::write(root.path().join("tree/LICENSE"), b"license").unwrap();

        let (directory, path) = open_tree(&root);
        let files = inventory_recursive(&directory, &path, GENEROUS, |_| true).unwrap();

        assert_eq!(
            files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            ["tree/Cargo.toml", "tree/LICENSE"]
        );
    }

    #[test]
    fn ignored_symlink_still_rejects_the_inventory() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("tree")).unwrap();
        fs::write(root.path().join("target"), b"target").unwrap();
        symlink(
            root.path().join("target"),
            root.path().join("tree/ignored.txt"),
        )
        .unwrap();

        let (directory, path) = open_tree(&root);
        let error = inventory_recursive(&directory, &path, GENEROUS, |candidate| {
            candidate.as_str().ends_with(".json")
        })
        .unwrap_err();
        assert!(error.to_string().contains("symbolic link"));
    }

    #[test]
    fn ignored_special_node_still_rejects_the_inventory() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("tree")).unwrap();
        let _socket = UnixListener::bind(root.path().join("tree/ignored.socket")).unwrap();

        let (directory, path) = open_tree(&root);
        let error = inventory_recursive(&directory, &path, GENEROUS, |candidate| {
            candidate.as_str().ends_with(".json")
        })
        .unwrap_err();
        assert!(error.to_string().contains("special filesystem node"));
    }

    #[test]
    fn ignored_unreadable_regular_file_still_rejects_the_inventory() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("tree")).unwrap();
        let unreadable = root.path().join("tree/ignored.txt");
        fs::write(&unreadable, b"ignored").unwrap();
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::File::open(&unreadable).is_ok() {
            // A privileged Unix runner may bypass mode bits; this environment
            // cannot provide real unreadability evidence.
            return;
        }

        let (directory, path) = open_tree(&root);
        let error = inventory_recursive(&directory, &path, GENEROUS, |candidate| {
            candidate.as_str().ends_with(".json")
        })
        .unwrap_err();
        assert!(format!("{error:#}").contains("cannot retain inventory file"));
    }

    #[test]
    fn every_inclusive_limit_accepts_its_exact_boundary() {
        let root = fixture_with_two_selected_files();
        let (directory, path) = open_tree(&root);
        let limits = InventoryLimits {
            max_depth: 2,
            max_entries_per_directory: 2,
            max_total_entries: 3,
            max_selected_files: 2,
            max_file_bytes: 3,
            max_selected_bytes: 5,
        };

        let files = inventory_recursive(&directory, &path, limits, |_| true).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files.iter().map(|file| file.bytes.len()).sum::<usize>(), 5);
    }

    #[test]
    fn total_entry_limit_counts_ignored_files_across_directories() {
        let root = tempfile::tempdir().unwrap();
        for directory in ["tree/a", "tree/b"] {
            fs::create_dir_all(root.path().join(directory)).unwrap();
            fs::write(root.path().join(directory).join("ignored.txt"), b"ignored").unwrap();
        }

        let (directory, path) = open_tree(&root);
        let exact = InventoryLimits {
            max_total_entries: 4,
            ..GENEROUS
        };
        assert!(inventory_recursive(&directory, &path, exact, |_| false)
            .unwrap()
            .is_empty());

        let (directory, path) = open_tree(&root);
        let crossing = InventoryLimits {
            max_total_entries: 3,
            ..GENEROUS
        };
        let error = inventory_recursive(&directory, &path, crossing, |_| false).unwrap_err();
        assert!(format!("{error:#}").contains("4 total entries; limit is 3"));
    }

    #[test]
    fn each_limit_rejects_one_step_beyond_its_boundary() {
        let checks = [
            (
                InventoryLimits {
                    max_depth: 1,
                    ..GENEROUS
                },
                "has depth 2; limit is 1",
            ),
            (
                InventoryLimits {
                    max_entries_per_directory: 1,
                    ..GENEROUS
                },
                "entry ceiling of 1",
            ),
            (
                InventoryLimits {
                    max_total_entries: 2,
                    ..GENEROUS
                },
                "3 total entries; limit is 2",
            ),
            (
                InventoryLimits {
                    max_selected_files: 1,
                    ..GENEROUS
                },
                "selected-file ceiling of 1",
            ),
            (
                InventoryLimits {
                    max_file_bytes: 2,
                    ..GENEROUS
                },
                "is 3 bytes; limit is 2",
            ),
            (
                InventoryLimits {
                    max_selected_bytes: 4,
                    ..GENEROUS
                },
                "would total 5; limit is 4",
            ),
        ];

        for (limits, expected) in checks {
            let root = fixture_with_two_selected_files();
            let (directory, path) = open_tree(&root);
            let error = inventory_recursive(&directory, &path, limits, |_| true).unwrap_err();
            assert!(
                format!("{error:#}").contains(expected),
                "unexpected error for {limits:?}: {error:#}"
            );
        }
    }
}
