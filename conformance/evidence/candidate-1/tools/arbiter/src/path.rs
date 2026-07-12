//! Repository-relative paths and handle-pinned, no-follow artifact access.

use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::{Read, Result as IoResult};
use std::path::Path;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A non-empty, normalized, ASCII repository-relative path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct RepoPath(String);

/// One canonical ASCII path component obtained from an exact directory entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ChildName(String);

/// One repository directory pinned for an entire arbiter operation.
///
/// File and directory methods descend from the retained root handle. No path
/// is validated and later reopened by name.
pub(crate) struct Repository(platform::Repository);

/// One repository directory retained across inventory and child reads.
pub(crate) struct PinnedDirectory(platform::PinnedDirectory);

/// The no-follow type observed for one entry in a pinned directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Directory,
    Regular,
    Symlink,
    Other,
}

/// One enumerated name, type, and physical identity bound to its parent.
///
/// Opening this entry verifies that the name still denotes the observed
/// object. A rename or replacement between enumeration and opening fails.
pub(crate) struct PinnedEntry<'directory> {
    parent: &'directory PinnedDirectory,
    name: ChildName,
    stamp: EntryStamp,
}

/// Immutable descriptor-visible metadata captured during enumeration.
///
/// The complete stamp, rather than identity alone, makes a same-inode rename
/// or mutation fail when the enumerated entry is subsequently opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntryStamp {
    key: FileKey,
    kind: NodeKind,
    mode: u32,
    len: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

/// Physical identity of one regular file, observed from its retained handle.
///
/// This is not conformance identity. It is used only to detect a record that
/// re-enters its own artifact set through a hard link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct FileKey(platform::PlatformFileKey);

/// One regular file retained after no-follow traversal and type validation.
pub(crate) struct PinnedFile {
    file: File,
    len: u64,
    key: FileKey,
    stamp: EntryStamp,
}

impl Repository {
    pub(crate) fn open(root: &Path) -> Result<Self> {
        platform::Repository::open(root).map(Self)
    }

    pub(crate) fn read_regular_file(&self, path: &RepoPath, max_bytes: usize) -> Result<Vec<u8>> {
        read_open_file(self.open_regular_file(path)?, &path.to_string(), max_bytes)
    }

    pub(crate) fn open_regular_file(&self, path: &RepoPath) -> Result<PinnedFile> {
        self.0.open_regular_file(path)
    }

    pub(crate) fn open_directory(&self, path: &RepoPath) -> Result<PinnedDirectory> {
        self.0.open_directory(path).map(PinnedDirectory)
    }

    pub(crate) fn pinned_root(&self) -> Result<PinnedDirectory> {
        self.0.pinned_root().map(PinnedDirectory)
    }
}

impl PinnedDirectory {
    pub(crate) fn open_regular_file(&self, path: &RepoPath) -> Result<PinnedFile> {
        self.0.open_regular_file(path)
    }

    pub(crate) fn open_directory(&self, path: &RepoPath) -> Result<PinnedDirectory> {
        self.0.open_directory(path).map(PinnedDirectory)
    }

    /// Enumerate canonical children in raw ASCII lexical order.
    ///
    /// Every returned entry carries the no-follow type and physical identity
    /// observed from this exact directory handle. Noncanonical names and
    /// ASCII-case-folded sibling collisions fail the closed inventory.
    pub(crate) fn entries(&self, max_entries: usize) -> Result<Vec<PinnedEntry<'_>>> {
        let observed = self.0.entries(max_entries)?;
        let mut folded = BTreeMap::new();
        let mut entries = Vec::with_capacity(observed.len());
        for (raw_name, stamp) in observed {
            let name = ChildName::from_bytes(&raw_name)?;
            let fold = name.as_str().to_ascii_lowercase();
            if let Some(prior) = folded.insert(fold, name.clone()) {
                bail!("directory contains an ASCII-case-folded name collision: {prior} and {name}");
            }
            entries.push(PinnedEntry {
                parent: self,
                name,
                stamp,
            });
        }
        Ok(entries)
    }
}

impl PinnedEntry<'_> {
    pub(crate) fn name(&self) -> &ChildName {
        &self.name
    }

    pub(crate) const fn kind(&self) -> NodeKind {
        self.stamp.kind
    }

    pub(crate) fn open_regular(&self) -> Result<PinnedFile> {
        if self.stamp.kind != NodeKind::Regular {
            bail!("directory entry is not a regular file: {}", self.name);
        }
        self.parent.0.open_regular_entry(&self.name, self.stamp)
    }

    pub(crate) fn open_directory(&self) -> Result<PinnedDirectory> {
        if self.stamp.kind != NodeKind::Directory {
            bail!("directory entry is not a directory: {}", self.name);
        }
        self.parent
            .0
            .open_directory_entry(&self.name, self.stamp)
            .map(PinnedDirectory)
    }
}

impl PinnedFile {
    pub(crate) const fn len(&self) -> u64 {
        self.len
    }

    pub(crate) const fn key(&self) -> FileKey {
        self.key
    }

    /// Verify that descriptor-visible metadata still equals the stamp taken
    /// when this exact handle was opened.
    pub(crate) fn verify_unchanged(&self) -> Result<()> {
        platform::verify_file_stamp(&self.file, &self.stamp)
            .context("retained artifact changed while it was consumed")
    }

    /// Consume this retained handle completely and verify it before exposing
    /// any bytes to the caller.
    pub(crate) fn read_all_verified(self, label: &str, max_bytes: usize) -> Result<Vec<u8>> {
        read_open_file(self, label, max_bytes)
    }
}

impl RepoPath {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn join_child(&self, child: &ChildName) -> Result<Self> {
        format!("{self}/{child}").parse()
    }

    /// Return the non-empty component suffix when `self` is strictly beneath
    /// `root`. Equality and textual prefixes without a `/` boundary return
    /// `None`.
    pub(crate) fn strict_relative_to(&self, root: &RepoPath) -> Option<Self> {
        self.0
            .strip_prefix(root.as_str())
            .and_then(|suffix| suffix.strip_prefix('/'))
            .and_then(|suffix| suffix.parse().ok())
    }

    #[allow(dead_code)] // Reserved for manifest root-antichain validation.
    pub(crate) fn ascii_folded(&self) -> String {
        self.0.to_ascii_lowercase()
    }
}

impl ChildName {
    fn from_bytes(value: &[u8]) -> Result<Self> {
        let value = std::str::from_utf8(value).context("directory entry name is not UTF-8")?;
        let path = value.parse::<RepoPath>()?;
        if path.0.contains('/') {
            bail!("directory entry name contains more than one component");
        }
        Ok(Self(path.0))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl Read for PinnedFile {
    fn read(&mut self, buffer: &mut [u8]) -> IoResult<usize> {
        self.file.read(buffer)
    }
}

fn read_open_file(mut file: PinnedFile, label: &str, max_bytes: usize) -> Result<Vec<u8>> {
    let max_u64 = u64::try_from(max_bytes).context("artifact byte ceiling exceeds u64")?;
    if file.len() > max_u64 {
        bail!(
            "artifact {label} is {} bytes; limit is {max_bytes}",
            file.len()
        );
    }

    let read_limit = max_u64
        .checked_add(1)
        .context("artifact byte ceiling cannot be checked")?;
    let capacity = usize::try_from(file.len()).context("artifact length exceeds usize")?;
    let mut bytes = Vec::with_capacity(capacity);
    file.by_ref()
        .take(read_limit)
        .read_to_end(&mut bytes)
        .with_context(|| format!("cannot read open artifact {label}"))?;
    if bytes.len() > max_bytes {
        bail!("artifact {label} grew beyond its byte ceiling");
    }
    if bytes.len() as u64 != file.len() {
        bail!("artifact {label} changed length while it was read");
    }
    file.verify_unchanged()
        .with_context(|| format!("cannot finalize open artifact {label}"))?;
    Ok(bytes)
}

impl fmt::Display for RepoPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl fmt::Display for ChildName {
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
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Component;

    use anyhow::{Context, Result};
    use errno::{errno, set_errno, Errno};

    use super::*;

    pub(super) struct Repository {
        root: OwnedFd,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub(super) struct PlatformFileKey {
        device: u64,
        inode: u64,
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
            let mut current = open_anchor(libc::AT_FDCWD, start, "repository root anchor")?;
            for component in root.components() {
                match component {
                    Component::RootDir | Component::CurDir => {}
                    Component::Normal(name) => {
                        current = open_directory_exact(
                            current.as_raw_fd(),
                            name.as_bytes(),
                            None,
                            "repository root",
                        )?;
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

        pub(super) fn open_regular_file(&self, relative: &RepoPath) -> Result<PinnedFile> {
            let descriptor = open_relative_exact(self.root.as_raw_fd(), relative, false)?;
            pinned_file(descriptor, &relative.to_string())
        }

        pub(super) fn open_directory(&self, relative: &RepoPath) -> Result<PinnedDirectory> {
            open_relative_exact(self.root.as_raw_fd(), relative, true)
                .map(|handle| PinnedDirectory { handle })
        }

        pub(super) fn pinned_root(&self) -> Result<PinnedDirectory> {
            let current = c".";
            let descriptor = unsafe {
                libc::openat(
                    self.root.as_raw_fd(),
                    current.as_ptr(),
                    libc::O_RDONLY | libc::O_CLOEXEC | libc::O_DIRECTORY | libc::O_NOFOLLOW,
                )
            };
            if descriptor < 0 {
                return Err(std::io::Error::last_os_error())
                    .context("cannot duplicate pinned repository root");
            }
            Ok(PinnedDirectory {
                handle: unsafe { OwnedFd::from_raw_fd(descriptor) },
            })
        }
    }

    pub(super) struct PinnedDirectory {
        handle: OwnedFd,
    }

    impl PinnedDirectory {
        pub(super) fn open_regular_file(&self, relative: &RepoPath) -> Result<PinnedFile> {
            let descriptor = open_relative_exact(self.handle.as_raw_fd(), relative, false)?;
            pinned_file(descriptor, &relative.to_string())
        }

        pub(super) fn open_directory(&self, relative: &RepoPath) -> Result<PinnedDirectory> {
            open_relative_exact(self.handle.as_raw_fd(), relative, true)
                .map(|handle| PinnedDirectory { handle })
        }

        pub(super) fn open_regular_entry(
            &self,
            name: &ChildName,
            expected: EntryStamp,
        ) -> Result<PinnedFile> {
            let descriptor = open_regular_exact(
                self.handle.as_raw_fd(),
                name.as_str().as_bytes(),
                Some(expected),
                &name.to_string(),
            )?;
            pinned_file_with_stamp(descriptor, expected, &name.to_string())
        }

        pub(super) fn open_directory_entry(
            &self,
            name: &ChildName,
            expected: EntryStamp,
        ) -> Result<PinnedDirectory> {
            open_directory_exact(
                self.handle.as_raw_fd(),
                name.as_str().as_bytes(),
                Some(expected),
                &name.to_string(),
            )
            .map(|handle| PinnedDirectory { handle })
        }

        pub(super) fn entries(&self, max_entries: usize) -> Result<Vec<(Vec<u8>, EntryStamp)>> {
            let mut entries = Vec::new();
            visit_directory(self.handle.as_raw_fd(), |name| {
                if entries.len() == max_entries {
                    bail!("directory exceeds its entry ceiling of {max_entries}");
                }
                let stamp = entry_stamp_at(self.handle.as_raw_fd(), name)?;
                entries.push((name.to_vec(), stamp));
                Ok(())
            })?;
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            Ok(entries)
        }
    }

    struct DirectoryStream(*mut libc::DIR);

    impl Drop for DirectoryStream {
        fn drop(&mut self) {
            unsafe { libc::closedir(self.0) };
        }
    }

    fn pinned_file(descriptor: OwnedFd, label: &str) -> Result<PinnedFile> {
        let stamp = entry_stamp_from_fd(descriptor.as_raw_fd())?;
        pinned_file_with_stamp(descriptor, stamp, label)
    }

    fn pinned_file_with_stamp(
        descriptor: OwnedFd,
        stamp: EntryStamp,
        label: &str,
    ) -> Result<PinnedFile> {
        if stamp.kind != NodeKind::Regular {
            bail!("artifact path is not a regular file: {label}");
        }
        Ok(PinnedFile {
            len: stamp.len,
            key: stamp.key,
            stamp,
            file: File::from(descriptor),
        })
    }

    fn open_relative_exact(root: RawFd, relative: &RepoPath, directory: bool) -> Result<OwnedFd> {
        let mut parent = root;
        let mut opened = None;
        let component_count = relative.0.split('/').count();
        for (index, component) in relative.0.split('/').enumerate() {
            let is_final = index + 1 == component_count;
            let descriptor = if !is_final || directory {
                open_directory_exact(parent, component.as_bytes(), None, &relative.to_string())?
            } else {
                open_regular_exact(parent, component.as_bytes(), None, &relative.to_string())?
            };
            parent = descriptor.as_raw_fd();
            opened = Some(descriptor);
        }
        opened.context("repository-relative path has no component")
    }

    fn open_anchor(parent: RawFd, name: &OsStr, label: &str) -> Result<OwnedFd> {
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

    fn open_directory_exact(
        parent: RawFd,
        name: &[u8],
        expected: Option<EntryStamp>,
        label: &str,
    ) -> Result<OwnedFd> {
        open_node_exact(
            parent,
            name,
            libc::O_DIRECTORY,
            NodeKind::Directory,
            expected,
            label,
        )
    }

    fn open_regular_exact(
        parent: RawFd,
        name: &[u8],
        expected: Option<EntryStamp>,
        label: &str,
    ) -> Result<OwnedFd> {
        open_node_exact(
            parent,
            name,
            libc::O_NONBLOCK,
            NodeKind::Regular,
            expected,
            label,
        )
    }

    fn open_node_exact(
        parent: RawFd,
        name: &[u8],
        type_flag: libc::c_int,
        wanted_kind: NodeKind,
        expected: Option<EntryStamp>,
        label: &str,
    ) -> Result<OwnedFd> {
        let name = CString::new(name).with_context(|| format!("{label} contains NUL"))?;
        let descriptor = unsafe {
            libc::openat(
                parent,
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | type_flag,
            )
        };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error())
                .with_context(|| format!("cannot securely open {label}"));
        }
        let descriptor = unsafe { OwnedFd::from_raw_fd(descriptor) };
        let actual = entry_stamp_from_fd(descriptor.as_raw_fd())?;
        if actual.kind != wanted_kind {
            bail!("opened path has the wrong type for {label}");
        }
        if let Some(expected) = expected {
            if expected != actual {
                bail!("directory entry changed before it could be opened: {label}");
            }
        } else {
            ensure_exact_mapping(parent, name.as_bytes(), actual, label)?;
        }
        Ok(descriptor)
    }

    fn ensure_exact_mapping(
        parent: RawFd,
        requested: &[u8],
        opened: EntryStamp,
        label: &str,
    ) -> Result<()> {
        let mut exact = None;
        let mut folded_alias = None;
        visit_directory(parent, |name| {
            if name == requested {
                exact = Some(entry_stamp_at(parent, name)?);
            } else if name.eq_ignore_ascii_case(requested) {
                folded_alias = Some(name.to_vec());
            }
            Ok(())
        })?;
        if let Some(alias) = folded_alias {
            bail!(
                "path component has an ASCII-case-folded sibling while opening {label}: {:?}",
                String::from_utf8_lossy(&alias)
            );
        }
        let exact = exact.with_context(|| {
            format!("path component is not present with exact raw spelling while opening {label}")
        })?;
        if exact.key != opened.key || exact.kind != opened.kind {
            bail!("path component changed during exact-spelling validation: {label}");
        }
        Ok(())
    }

    fn entry_stamp_at(parent: RawFd, name: &[u8]) -> Result<EntryStamp> {
        let name = CString::new(name).context("directory entry contains NUL")?;
        let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe {
            libc::fstatat(
                parent,
                name.as_ptr(),
                status.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result < 0 {
            return Err(std::io::Error::last_os_error())
                .context("cannot inspect directory entry without following links");
        }
        let status = unsafe { status.assume_init() };
        Ok(entry_stamp(&status))
    }

    fn entry_stamp_from_fd(descriptor: RawFd) -> Result<EntryStamp> {
        let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
        let result = unsafe { libc::fstat(descriptor, status.as_mut_ptr()) };
        if result < 0 {
            return Err(std::io::Error::last_os_error()).context("cannot inspect opened entry");
        }
        let status = unsafe { status.assume_init() };
        Ok(entry_stamp(&status))
    }

    #[allow(clippy::unnecessary_cast)] // libc field widths vary across Unix targets.
    fn entry_stamp(status: &libc::stat) -> EntryStamp {
        let mode = status.st_mode as libc::mode_t;
        let kind = match mode & libc::S_IFMT {
            libc::S_IFDIR => NodeKind::Directory,
            libc::S_IFREG => NodeKind::Regular,
            libc::S_IFLNK => NodeKind::Symlink,
            _ => NodeKind::Other,
        };
        EntryStamp {
            key: FileKey(PlatformFileKey {
                device: status.st_dev as u64,
                inode: status.st_ino as u64,
            }),
            kind,
            mode: status.st_mode as u32,
            len: status.st_size as u64,
            modified_seconds: status.st_mtime as i64,
            modified_nanoseconds: status.st_mtime_nsec as i64,
            changed_seconds: status.st_ctime as i64,
            changed_nanoseconds: status.st_ctime_nsec as i64,
        }
    }

    fn visit_directory(parent: RawFd, mut visit: impl FnMut(&[u8]) -> Result<()>) -> Result<()> {
        let current = c".";
        let descriptor = unsafe {
            libc::openat(
                parent,
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
            if name != b"." && name != b".." {
                visit(name)?;
            }
        }
        Ok(())
    }

    pub(super) fn verify_file_stamp(file: &File, expected: &EntryStamp) -> Result<()> {
        let actual = entry_stamp_from_fd(file.as_raw_fd())
            .context("cannot reinspect retained artifact handle")?;
        if &actual != expected {
            bail!("file identity, mode, length, or timestamp metadata differs");
        }
        Ok(())
    }
}

#[cfg(not(unix))]
mod platform {
    use super::*;

    pub(super) struct Repository;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub(super) struct PlatformFileKey;

    impl Repository {
        pub(super) fn open(_root: &Path) -> Result<Self> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }

        pub(super) fn open_regular_file(&self, _path: &RepoPath) -> Result<PinnedFile> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }

        pub(super) fn open_directory(&self, _path: &RepoPath) -> Result<PinnedDirectory> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }

        pub(super) fn pinned_root(&self) -> Result<PinnedDirectory> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }
    }

    pub(super) struct PinnedDirectory;

    impl PinnedDirectory {
        pub(super) fn open_regular_file(&self, _path: &RepoPath) -> Result<PinnedFile> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }

        pub(super) fn open_directory(&self, _path: &RepoPath) -> Result<PinnedDirectory> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }

        pub(super) fn open_regular_entry(
            &self,
            _name: &ChildName,
            _expected: EntryStamp,
        ) -> Result<PinnedFile> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }

        pub(super) fn open_directory_entry(
            &self,
            _name: &ChildName,
            _expected: EntryStamp,
        ) -> Result<PinnedDirectory> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }

        pub(super) fn entries(&self, _max_entries: usize) -> Result<Vec<(Vec<u8>, EntryStamp)>> {
            bail!("secure no-follow artifact access is not implemented on this platform")
        }
    }

    pub(super) fn verify_file_stamp(_file: &File, _expected: &EntryStamp) -> Result<()> {
        bail!("secure no-follow artifact access is not implemented on this platform")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
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

    #[test]
    fn joins_folds_and_checks_strict_component_containment() {
        let root: RepoPath = "root".parse().unwrap();
        let child = ChildName::from_bytes(b"Nested").unwrap();
        let joined = root.join_child(&child).unwrap();
        assert_eq!(joined.as_str(), "root/Nested");
        assert_eq!(joined.ascii_folded(), "root/nested");
        assert_eq!(joined.strict_relative_to(&root).unwrap().as_str(), "Nested");
        assert!(root.strict_relative_to(&root).is_none());
        assert!("rooted/file"
            .parse::<RepoPath>()
            .unwrap()
            .strict_relative_to(&root)
            .is_none());
    }

    #[cfg(unix)]
    #[test]
    fn reads_and_enumerates_from_one_pinned_repository_handle() {
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
        let entries = pinned.entries(2).unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name().as_str())
                .collect::<Vec<_>>(),
            ["another.json", "file.json"]
        );
        assert_eq!(
            entries[1]
                .open_regular()
                .unwrap()
                .read_all_verified("file.json", 5)
                .unwrap(),
            b"bytes"
        );
        assert!(pinned.entries(1).is_err());
        assert!(repository.read_regular_file(&directory, 5).is_err());

        fs::rename(physical_root.join("a"), physical_root.join("old-a")).unwrap();
        fs::create_dir(physical_root.join("a")).unwrap();
        fs::write(physical_root.join("a/file.json"), b"replacement").unwrap();
        let renamed_entries = pinned.entries(2).unwrap();
        assert_eq!(
            renamed_entries
                .iter()
                .map(|entry| entry.name().as_str())
                .collect::<Vec<_>>(),
            ["another.json", "file.json"]
        );
        assert_eq!(
            renamed_entries[1]
                .open_regular()
                .unwrap()
                .read_all_verified("file.json", 5)
                .unwrap(),
            b"bytes"
        );
    }

    #[cfg(unix)]
    #[test]
    fn pins_files_and_distinguishes_hard_links_from_copies() {
        use std::io::Read as _;

        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("original"), b"original").unwrap();
        fs::hard_link(root.path().join("original"), root.path().join("linked")).unwrap();
        fs::copy(root.path().join("original"), root.path().join("copied")).unwrap();
        let physical_root = fs::canonicalize(root.path()).unwrap();
        let repository = Repository::open(&physical_root).unwrap();

        let original_path: RepoPath = "original".parse().unwrap();
        let linked_path: RepoPath = "linked".parse().unwrap();
        let copied_path: RepoPath = "copied".parse().unwrap();
        let mut original = repository.open_regular_file(&original_path).unwrap();
        assert_eq!(
            original.key(),
            repository.open_regular_file(&linked_path).unwrap().key()
        );
        assert_ne!(
            original.key(),
            repository.open_regular_file(&copied_path).unwrap().key()
        );

        fs::rename(
            physical_root.join("original"),
            physical_root.join("renamed"),
        )
        .unwrap();
        fs::write(physical_root.join("original"), b"replacement").unwrap();
        let mut bytes = Vec::new();
        original.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"original");
        assert!(original.verify_unchanged().is_err());
        assert_eq!(
            repository.read_regular_file(&original_path, 11).unwrap(),
            b"replacement"
        );
    }

    #[cfg(unix)]
    #[test]
    fn descends_only_from_retained_directory_handles() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("a/nested")).unwrap();
        fs::write(root.path().join("a/nested/file"), b"nested").unwrap();
        let physical_root = fs::canonicalize(root.path()).unwrap();
        let repository = Repository::open(&physical_root).unwrap();
        let pinned_root = repository.pinned_root().unwrap();
        let a = pinned_root.open_directory(&"a".parse().unwrap()).unwrap();
        let nested = a.open_directory(&"nested".parse().unwrap()).unwrap();
        assert_eq!(
            nested
                .open_regular_file(&"file".parse().unwrap())
                .unwrap()
                .read_all_verified("file", 6)
                .unwrap(),
            b"nested"
        );
        assert_eq!(
            pinned_root
                .open_regular_file(&"a/nested/file".parse().unwrap())
                .unwrap()
                .len(),
            6
        );
    }

    #[cfg(unix)]
    #[test]
    fn enumerated_entries_bind_name_type_and_identity_to_their_parent() {
        use std::os::unix::fs::symlink;
        use std::os::unix::net::UnixListener;

        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("directory")).unwrap();
        fs::write(root.path().join("regular"), b"bytes").unwrap();
        symlink("regular", root.path().join("symlink")).unwrap();
        let _socket = UnixListener::bind(root.path().join("socket")).unwrap();
        let physical = fs::canonicalize(root.path()).unwrap();
        let repository = Repository::open(&physical).unwrap();
        let pinned = repository.pinned_root().unwrap();
        let entries = pinned.entries(4).unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name().as_str())
                .collect::<Vec<_>>(),
            ["directory", "regular", "socket", "symlink"]
        );

        let directory = &entries[0];
        assert_eq!(directory.kind(), NodeKind::Directory);
        assert!(directory.open_directory().is_ok());
        assert!(directory.open_regular().is_err());

        let regular = &entries[1];
        assert_eq!(regular.kind(), NodeKind::Regular);
        assert!(regular.open_regular().is_ok());
        assert!(regular.open_directory().is_err());

        assert_eq!(entries[2].kind(), NodeKind::Other);
        assert!(entries[2].open_regular().is_err());
        assert_eq!(entries[3].kind(), NodeKind::Symlink);
        assert!(entries[3].open_regular().is_err());
        assert!(entries[3].open_directory().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn enumerated_entry_replacement_fails_instead_of_opening_new_bytes() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("member"), b"original").unwrap();
        let physical = fs::canonicalize(root.path()).unwrap();
        let repository = Repository::open(&physical).unwrap();
        let pinned = repository.pinned_root().unwrap();
        let entries = pinned.entries(1).unwrap();
        let member = &entries[0];

        fs::rename(physical.join("member"), physical.join("old-member")).unwrap();
        fs::write(physical.join("member"), b"replacement").unwrap();
        assert!(member.open_regular().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn enumerated_entry_case_rename_fails_even_when_the_inode_is_unchanged() {
        use std::os::unix::fs::MetadataExt as _;

        let root = tempfile::tempdir().unwrap();
        let original = root.path().join("Member");
        let renamed = root.path().join("member");
        fs::write(&original, b"bytes").unwrap();
        let physical = fs::canonicalize(root.path()).unwrap();
        let repository = Repository::open(&physical).unwrap();
        let pinned = repository.pinned_root().unwrap();
        let entries = pinned.entries(1).unwrap();
        let before = fs::metadata(&original).unwrap();

        fs::rename(&original, &renamed).unwrap();
        let after = fs::metadata(&renamed).unwrap();
        assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
        assert!(entries[0].open_regular().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn exact_open_and_inventory_reject_casefold_aliases() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("Name"), b"first").unwrap();
        fs::write(root.path().join("name"), b"second").unwrap();
        let physical = fs::canonicalize(root.path()).unwrap();
        let actual_names = fs::read_dir(&physical)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect::<Vec<_>>();
        let repository = Repository::open(&physical).unwrap();
        let pinned = repository.pinned_root().unwrap();

        if actual_names.len() == 2 {
            assert!(pinned.entries(2).is_err());
            assert!(repository
                .open_regular_file(&"Name".parse().unwrap())
                .is_err());
            assert!(repository
                .open_regular_file(&"name".parse().unwrap())
                .is_err());
        } else {
            let actual = actual_names.first().unwrap();
            let wrong_case = if actual == "Name" { "name" } else { "Name" };
            assert!(repository
                .open_regular_file(&wrong_case.parse().unwrap())
                .is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn same_length_in_place_mutation_fails_explicit_finalization() {
        use std::io::Read as _;

        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("member");
        fs::write(&path, b"original").unwrap();
        let physical = fs::canonicalize(root.path()).unwrap();
        let repository = Repository::open(&physical).unwrap();
        let mut pinned = repository
            .open_regular_file(&"member".parse().unwrap())
            .unwrap();
        let before = fs::metadata(&path).unwrap().modified().unwrap();
        for value in [b"mutation".as_slice(), b"changed!".as_slice()] {
            fs::write(&path, value).unwrap();
            if fs::metadata(&path).unwrap().modified().unwrap() != before {
                break;
            }
        }
        assert_ne!(fs::metadata(&path).unwrap().modified().unwrap(), before);
        let mut bytes = Vec::new();
        pinned.read_to_end(&mut bytes).unwrap();
        assert!(pinned.verify_unchanged().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn closed_inventory_rejects_noncanonical_names() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(".hidden"), b"x").unwrap();
        let physical = fs::canonicalize(root.path()).unwrap();
        let repository = Repository::open(&physical).unwrap();
        assert!(repository.pinned_root().unwrap().entries(1).is_err());
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

    #[cfg(unix)]
    #[test]
    fn repository_root_rejects_an_observed_users_case_alias() {
        let exact = Path::new("/Users");
        let alias = Path::new("/users");
        if exact.is_dir() && alias.is_dir() {
            assert!(Repository::open(exact).is_ok());
            assert!(Repository::open(alias).is_err());
        }
    }
}
