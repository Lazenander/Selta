//! Repository-relative paths and handle-pinned, no-follow artifact access.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use anyhow::{bail, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A non-empty, normalized, ASCII repository-relative path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RepoPath(String);

/// One repository directory pinned for an entire arbiter operation.
///
/// File and directory methods descend from the retained root handle. No path
/// is validated and later reopened by name.
pub(crate) struct Repository(platform::Repository);

/// One repository directory retained across inventory and child reads.
pub(crate) struct PinnedDirectory(platform::PinnedDirectory);

impl Repository {
    pub(crate) fn open(root: &Path) -> Result<Self> {
        platform::Repository::open(root).map(Self)
    }

    pub(crate) fn read_regular_file(&self, path: &RepoPath, max_bytes: usize) -> Result<Vec<u8>> {
        self.0.read_regular_file(path, max_bytes)
    }

    pub(crate) fn open_directory(&self, path: &RepoPath) -> Result<PinnedDirectory> {
        self.0.open_directory(path).map(PinnedDirectory)
    }
}

impl PinnedDirectory {
    pub(crate) fn list(&self, max_entries: usize) -> Result<Vec<Vec<u8>>> {
        self.0.list(max_entries)
    }

    pub(crate) fn read_regular_file(&self, name: &str, max_bytes: usize) -> Result<Vec<u8>> {
        let path = name.parse::<RepoPath>()?;
        if path.0.contains('/') {
            bail!("pinned-directory child name must contain one component");
        }
        self.0.read_regular_file(&path.0, max_bytes)
    }
}

impl fmt::Display for RepoPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for RepoPath {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        if value.is_empty() {
            bail!("repository path must not be empty");
        }
        if value.len() > 1024 {
            bail!("repository path exceeds 1,024 bytes");
        }
        if !value.is_ascii() {
            bail!("repository path must be ASCII");
        }

        for component in value.split('/') {
            if component.is_empty() {
                bail!("repository path has an empty component");
            }
            let mut bytes = component.bytes();
            let first = bytes.next().expect("non-empty component");
            if !is_component_initial(first) || !bytes.all(is_component_remainder) {
                bail!("repository path contains a noncanonical component: {component:?}");
            }
        }
        Ok(Self(value.to_owned()))
    }
}

impl TryFrom<String> for RepoPath {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}

impl Serialize for RepoPath {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RepoPath {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

fn is_component_initial(value: u8) -> bool {
    value.is_ascii_alphanumeric() || matches!(value, b'_' | b'-')
}

fn is_component_remainder(value: u8) -> bool {
    is_component_initial(value) || value == b'.'
}

#[cfg(unix)]
mod platform {
    use std::ffi::{CStr, CString, OsStr};
    use std::fs::File;
    use std::io::Read;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Component;

    use anyhow::{Context, Result};
    use errno::{errno, set_errno, Errno};

    use super::*;

    pub(super) struct Repository {
        root: OwnedFd,
    }

    impl Repository {
        pub(super) fn open(root: &Path) -> Result<Self> {
            if root.as_os_str().is_empty() {
                bail!("repository root must not be empty");
            }
            let start = if root.is_absolute() {
                OsStr::new("/")
            } else {
                OsStr::new(".")
            };
            let mut current = open_directory(libc::AT_FDCWD, start, "repository root anchor")?;
            for component in root.components() {
                match component {
                    Component::RootDir | Component::CurDir => {}
                    Component::Normal(name) => {
                        current = open_directory(current.as_raw_fd(), name, "repository root")?;
                    }
                    Component::ParentDir => {
                        bail!("repository root must not contain a parent component")
                    }
                    Component::Prefix(_) => {
                        bail!("repository root uses an unsupported platform prefix")
                    }
                }
            }
            Ok(Self { root: current })
        }

        pub(super) fn read_regular_file(
            &self,
            relative: &RepoPath,
            max_bytes: usize,
        ) -> Result<Vec<u8>> {
            let descriptor = self.open_relative(relative, false)?;
            read_open_file(descriptor, &relative.to_string(), max_bytes)
        }

        pub(super) fn open_directory(&self, relative: &RepoPath) -> Result<PinnedDirectory> {
            self.open_relative(relative, true)
                .map(|handle| PinnedDirectory { handle })
        }

        fn open_relative(&self, relative: &RepoPath, directory: bool) -> Result<OwnedFd> {
            let mut parent = self.root.as_raw_fd();
            let mut opened = None;
            let component_count = relative.0.split('/').count();
            for (index, component) in relative.0.split('/').enumerate() {
                let component = CString::new(component).expect("RepoPath excludes NUL");
                let is_final = index + 1 == component_count;
                let mut flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW;
                if !is_final || directory {
                    flags |= libc::O_DIRECTORY;
                } else {
                    flags |= libc::O_NONBLOCK;
                }
                let descriptor = unsafe { libc::openat(parent, component.as_ptr(), flags) };
                if descriptor < 0 {
                    return Err(std::io::Error::last_os_error())
                        .with_context(|| format!("cannot securely open artifact {relative}"));
                }
                let descriptor = unsafe { OwnedFd::from_raw_fd(descriptor) };
                parent = descriptor.as_raw_fd();
                opened = Some(descriptor);
            }
            opened.context("repository-relative path has no component")
        }
    }

    pub(super) struct PinnedDirectory {
        handle: OwnedFd,
    }

    impl PinnedDirectory {
        pub(super) fn read_regular_file(&self, name: &str, max_bytes: usize) -> Result<Vec<u8>> {
            let name = CString::new(name).expect("RepoPath child excludes NUL");
            let descriptor = unsafe {
                libc::openat(
                    self.handle.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
                )
            };
            if descriptor < 0 {
                return Err(std::io::Error::last_os_error())
                    .with_context(|| format!("cannot securely open directory child {name:?}"));
            }
            let descriptor = unsafe { OwnedFd::from_raw_fd(descriptor) };
            read_open_file(descriptor, name.to_str().unwrap_or("<non-UTF8>"), max_bytes)
        }

        pub(super) fn list(&self, max_entries: usize) -> Result<Vec<Vec<u8>>> {
            let current = c".";
            let descriptor = unsafe {
                libc::openat(
                    self.handle.as_raw_fd(),
                    current.as_ptr(),
                    libc::O_RDONLY | libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW,
                )
            };
            if descriptor < 0 {
                return Err(std::io::Error::last_os_error())
                    .context("cannot reopen pinned directory handle");
            }
            // fdopendir owns the descriptor after success.
            let directory = unsafe { libc::fdopendir(descriptor) };
            if directory.is_null() {
                let error = std::io::Error::last_os_error();
                unsafe { libc::close(descriptor) };
                return Err(error).context("cannot enumerate open directory handle");
            }
            let directory = DirectoryStream(directory);
            let mut names = Vec::new();
            loop {
                set_errno(Errno(0));
                let entry = unsafe { libc::readdir(directory.0) };
                if entry.is_null() {
                    let error = errno();
                    if error.0 != 0 {
                        bail!("cannot read open directory handle: {error}");
                    }
                    break;
                }
                let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
                if name == b"." || name == b".." {
                    continue;
                }
                if names.len() == max_entries {
                    bail!("directory exceeds its entry ceiling of {max_entries}");
                }
                names.push(name.to_vec());
            }
            names.sort();
            Ok(names)
        }
    }

    struct DirectoryStream(*mut libc::DIR);

    impl Drop for DirectoryStream {
        fn drop(&mut self) {
            unsafe { libc::closedir(self.0) };
        }
    }

    fn read_open_file(descriptor: OwnedFd, label: &str, max_bytes: usize) -> Result<Vec<u8>> {
        let mut file = File::from(descriptor);
        let metadata = file
            .metadata()
            .with_context(|| format!("cannot inspect open artifact {label}"))?;
        if !metadata.is_file() {
            bail!("artifact path is not a regular file: {label}");
        }
        let max_u64 = u64::try_from(max_bytes).context("artifact byte ceiling exceeds u64")?;
        if metadata.len() > max_u64 {
            bail!(
                "artifact {label} is {} bytes; limit is {max_bytes}",
                metadata.len()
            );
        }

        let read_limit = max_u64
            .checked_add(1)
            .context("artifact byte ceiling cannot be checked")?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.by_ref()
            .take(read_limit)
            .read_to_end(&mut bytes)
            .with_context(|| format!("cannot read open artifact {label}"))?;
        if bytes.len() > max_bytes {
            bail!("artifact {label} grew beyond its byte ceiling");
        }
        Ok(bytes)
    }

    fn open_directory(parent: RawFd, name: &OsStr, label: &str) -> Result<OwnedFd> {
        let name =
            CString::new(name.as_bytes()).with_context(|| format!("{label} contains NUL"))?;
        let descriptor = unsafe {
            libc::openat(
                parent,
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW,
            )
        };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error())
                .with_context(|| format!("cannot securely open {label}"));
        }
        Ok(unsafe { OwnedFd::from_raw_fd(descriptor) })
    }
}

#[cfg(not(unix))]
mod platform {
    use super::*;

    pub(super) struct Repository;

    impl Repository {
        pub(super) fn open(_root: &Path) -> Result<Self> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }

        pub(super) fn read_regular_file(
            &self,
            _path: &RepoPath,
            _max_bytes: usize,
        ) -> Result<Vec<u8>> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }

        pub(super) fn open_directory(&self, _path: &RepoPath) -> Result<PinnedDirectory> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }
    }

    pub(super) struct PinnedDirectory;

    impl PinnedDirectory {
        pub(super) fn read_regular_file(&self, _name: &str, _max_bytes: usize) -> Result<Vec<u8>> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }

        pub(super) fn list(&self, _max_entries: usize) -> Result<Vec<Vec<u8>>> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn accepts_only_the_closed_wire_grammar() {
        for valid in ["a", "A_0-9", "a.json", "one/two-three/file.raw"] {
            assert_eq!(valid.parse::<RepoPath>().unwrap().to_string(), valid);
        }
        for invalid in [
            "",
            "/a",
            "a/",
            "a//b",
            ".",
            "..",
            ".hidden",
            "a/../b",
            "a\\b",
            "a b",
            "caf\u{e9}",
        ] {
            assert!(invalid.parse::<RepoPath>().is_err(), "accepted {invalid:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn reads_and_lists_from_one_pinned_repository_handle() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("a")).unwrap();
        fs::write(root.path().join("a/file.json"), b"bytes").unwrap();
        fs::write(root.path().join("a/another.json"), b"other").unwrap();
        let physical_root = fs::canonicalize(root.path()).unwrap();
        let repository = Repository::open(&physical_root).unwrap();
        let file: RepoPath = "a/file.json".parse().unwrap();
        assert_eq!(repository.read_regular_file(&file, 5).unwrap(), b"bytes");
        assert!(repository.read_regular_file(&file, 4).is_err());
        let directory: RepoPath = "a".parse().unwrap();
        let pinned = repository.open_directory(&directory).unwrap();
        assert_eq!(
            pinned.list(2).unwrap(),
            [b"another.json".to_vec(), b"file.json".to_vec()]
        );
        assert_eq!(pinned.read_regular_file("file.json", 5).unwrap(), b"bytes");
        assert!(pinned.read_regular_file("nested/file", 5).is_err());
        assert!(pinned.list(1).is_err());
        assert!(repository.read_regular_file(&directory, 5).is_err());

        fs::rename(physical_root.join("a"), physical_root.join("old-a")).unwrap();
        fs::create_dir(physical_root.join("a")).unwrap();
        fs::write(physical_root.join("a/file.json"), b"replacement").unwrap();
        assert_eq!(
            pinned.list(2).unwrap(),
            [b"another.json".to_vec(), b"file.json".to_vec()]
        );
        assert_eq!(pinned.read_regular_file("file.json", 5).unwrap(), b"bytes");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_at_root_and_relative_components() {
        use std::os::unix::fs::symlink;

        let parent = tempfile::tempdir().unwrap();
        let physical_parent = fs::canonicalize(parent.path()).unwrap();
        let real = physical_parent.join("real");
        fs::create_dir(&real).unwrap();
        fs::create_dir(real.join("dir")).unwrap();
        fs::write(real.join("dir/file"), b"x").unwrap();
        symlink(&real, physical_parent.join("linked-root")).unwrap();
        symlink(real.join("dir"), real.join("linked-dir")).unwrap();
        symlink(real.join("dir/file"), real.join("linked-file")).unwrap();

        let linked_root = physical_parent.join("linked-root");
        assert!(Repository::open(&linked_root).is_err());
        let trailing = Path::new(&format!("{}/", linked_root.display())).to_path_buf();
        assert!(Repository::open(&trailing).is_err());

        let repository = Repository::open(&real).unwrap();
        for path in ["linked-dir/file", "linked-file"] {
            assert!(repository
                .read_regular_file(&path.parse().unwrap(), 16)
                .is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn rejects_parent_components_in_the_repository_root() {
        assert!(Repository::open(Path::new("somewhere/../elsewhere")).is_err());
    }
}
