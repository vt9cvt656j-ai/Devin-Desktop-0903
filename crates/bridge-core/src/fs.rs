use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use walkdir::WalkDir;

use crate::error::{BridgeError, Result};

/// A filesystem view confined to a single root directory.
///
/// All public methods take a path *relative to the root*. Paths are normalised
/// lexically (resolving `.`/`..`) and verified to stay within the root; for
/// existing targets the real path is also canonicalised so that symlinks
/// cannot be used to escape.
#[derive(Debug, Clone)]
pub struct ScopedFs {
    root: PathBuf,
}

/// Metadata for a single directory entry.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Entry {
    pub name: String,
    /// Path relative to the root, using `/` separators.
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
}

/// A search hit.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SearchHit {
    pub path: String,
    pub is_dir: bool,
    /// First matching line (1-indexed) when matched on content.
    pub line: Option<usize>,
    pub preview: Option<String>,
}

impl ScopedFs {
    /// Create a view rooted at `root`. The root must already exist.
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let canon = root
            .canonicalize()
            .map_err(|_| BridgeError::InvalidRoot(root.to_path_buf()))?;
        if !canon.is_dir() {
            return Err(BridgeError::InvalidRoot(root.to_path_buf()));
        }
        Ok(Self { root: canon })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a relative path to an absolute path inside the root.
    ///
    /// Rejects absolute inputs and any path that would escape the root.
    pub fn resolve(&self, rel: &str) -> Result<PathBuf> {
        let candidate = Path::new(rel);
        if candidate.is_absolute() {
            return Err(BridgeError::PathEscapesRoot(rel.to_string()));
        }

        // Lexically normalise, rejecting any traversal above the root.
        let mut normalised = PathBuf::new();
        for comp in candidate.components() {
            match comp {
                Component::Normal(c) => normalised.push(c),
                Component::CurDir => {}
                Component::ParentDir => {
                    if !normalised.pop() {
                        return Err(BridgeError::PathEscapesRoot(rel.to_string()));
                    }
                }
                Component::RootDir | Component::Prefix(_) => {
                    return Err(BridgeError::PathEscapesRoot(rel.to_string()));
                }
            }
        }

        let full = self.root.join(&normalised);

        // If the target (or its closest existing ancestor) resolves through a
        // symlink that points outside the root, reject it.
        let check = if full.exists() {
            full.clone()
        } else {
            // Walk up to the closest *existing* ancestor so a symlink anywhere
            // along the path (not just the immediate parent) is still checked.
            full.ancestors()
                .find(|p| p.exists())
                .map(Path::to_path_buf)
                .unwrap_or_else(|| self.root.clone())
        };
        if let Ok(canon) = check.canonicalize() {
            if !canon.starts_with(&self.root) {
                return Err(BridgeError::PathEscapesRoot(rel.to_string()));
            }
        }

        Ok(full)
    }

    fn to_rel_string(&self, p: &Path) -> String {
        p.strip_prefix(&self.root)
            .unwrap_or(p)
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }

    /// List the entries of a directory (non-recursive).
    pub fn list_dir(&self, rel: &str) -> Result<Vec<Entry>> {
        let full = self.resolve(rel)?;
        if !full.exists() {
            return Err(BridgeError::NotFound(rel.to_string()));
        }
        if !full.is_dir() {
            return Err(BridgeError::NotADirectory(rel.to_string()));
        }
        let mut entries = Vec::new();
        for dirent in std::fs::read_dir(&full)? {
            let dirent = dirent?;
            let meta = dirent.metadata()?;
            let path = dirent.path();
            entries.push(Entry {
                name: dirent.file_name().to_string_lossy().to_string(),
                path: self.to_rel_string(&path),
                is_dir: meta.is_dir(),
                is_symlink: meta.file_type().is_symlink(),
                size: meta.len(),
            });
        }
        entries.sort_by_key(|e| (!e.is_dir, e.name.to_lowercase()));
        Ok(entries)
    }

    /// Read the full contents of a file.
    pub fn read_file(&self, rel: &str) -> Result<Vec<u8>> {
        let full = self.resolve(rel)?;
        if !full.exists() {
            return Err(BridgeError::NotFound(rel.to_string()));
        }
        if !full.is_file() {
            return Err(BridgeError::NotAFile(rel.to_string()));
        }
        Ok(std::fs::read(&full)?)
    }

    /// Write bytes to a file, creating parent directories as needed.
    pub fn write_file(&self, rel: &str, bytes: &[u8]) -> Result<()> {
        let full = self.resolve(rel)?;
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full, bytes)?;
        Ok(())
    }

    /// Create a directory (and parents).
    pub fn mkdir(&self, rel: &str) -> Result<()> {
        let full = self.resolve(rel)?;
        std::fs::create_dir_all(&full)?;
        Ok(())
    }

    /// Delete a file or directory (recursively for directories).
    pub fn delete(&self, rel: &str) -> Result<()> {
        let full = self.resolve(rel)?;
        if !full.exists() {
            return Err(BridgeError::NotFound(rel.to_string()));
        }
        if full == self.root {
            return Err(BridgeError::BadRequest(
                "refusing to delete the root".into(),
            ));
        }
        if full.is_dir() {
            std::fs::remove_dir_all(&full)?;
        } else {
            std::fs::remove_file(&full)?;
        }
        Ok(())
    }

    /// Search within the root for entries whose name matches `query`, and,
    /// when `content` is true, lines inside text files that contain `query`.
    pub fn search(
        &self,
        rel: &str,
        query: &str,
        content: bool,
        limit: usize,
    ) -> Result<Vec<SearchHit>> {
        let base = self.resolve(rel)?;
        if !base.is_dir() {
            return Err(BridgeError::NotADirectory(rel.to_string()));
        }
        let needle = query.to_lowercase();
        let mut hits = Vec::new();
        for entry in WalkDir::new(&base)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if hits.len() >= limit {
                break;
            }
            let path = entry.path();
            if path == self.root {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.contains(&needle) {
                hits.push(SearchHit {
                    path: self.to_rel_string(path),
                    is_dir: entry.file_type().is_dir(),
                    line: None,
                    preview: None,
                });
                continue;
            }
            if content && entry.file_type().is_file() {
                if let Some(hit) = self.content_match(path, &needle) {
                    hits.push(hit);
                }
            }
        }
        Ok(hits)
    }

    fn content_match(&self, path: &Path, needle: &str) -> Option<SearchHit> {
        let bytes = std::fs::read(path).ok()?;
        // Skip obvious binaries.
        if bytes.iter().take(8000).any(|&b| b == 0) {
            return None;
        }
        let text = String::from_utf8_lossy(&bytes);
        for (i, line) in text.lines().enumerate() {
            if line.to_lowercase().contains(needle) {
                let preview = line.trim();
                let preview = if preview.len() > 200 {
                    let mut end = 200;
                    while !preview.is_char_boundary(end) {
                        end -= 1;
                    }
                    &preview[..end]
                } else {
                    preview
                };
                return Some(SearchHit {
                    path: self.to_rel_string(path),
                    is_dir: false,
                    line: Some(i + 1),
                    preview: Some(preview.to_string()),
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup() -> (tempfile::TempDir, ScopedFs) {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("hello.txt"), b"hello world\nsecond line").unwrap();
        fs::write(dir.path().join("sub/notes.md"), b"# Title\nalpha beta").unwrap();
        let sfs = ScopedFs::new(dir.path()).unwrap();
        (dir, sfs)
    }

    #[test]
    fn lists_sorted_dirs_first() {
        let (_d, sfs) = setup();
        let entries = sfs.list_dir("").unwrap();
        assert_eq!(entries[0].name, "sub");
        assert!(entries[0].is_dir);
        assert!(entries.iter().any(|e| e.name == "hello.txt"));
    }

    #[test]
    fn reads_and_writes() {
        let (_d, sfs) = setup();
        assert_eq!(
            sfs.read_file("hello.txt").unwrap(),
            b"hello world\nsecond line"
        );
        sfs.write_file("nested/new.txt", b"data").unwrap();
        assert_eq!(sfs.read_file("nested/new.txt").unwrap(), b"data");
    }

    #[test]
    fn rejects_parent_traversal() {
        let (_d, sfs) = setup();
        let err = sfs.read_file("../outside.txt").unwrap_err();
        assert!(matches!(err, BridgeError::PathEscapesRoot(_)));
        let err = sfs.read_file("sub/../../etc/passwd").unwrap_err();
        assert!(matches!(err, BridgeError::PathEscapesRoot(_)));
    }

    #[test]
    fn rejects_absolute_paths() {
        let (_d, sfs) = setup();
        let err = sfs.read_file("/etc/passwd").unwrap_err();
        assert!(matches!(err, BridgeError::PathEscapesRoot(_)));
    }

    #[test]
    fn search_matches_name_and_content() {
        let (_d, sfs) = setup();
        let by_name = sfs.search("", "notes", false, 10).unwrap();
        assert!(by_name.iter().any(|h| h.path == "sub/notes.md"));

        let by_content = sfs.search("", "beta", true, 10).unwrap();
        let hit = by_content
            .iter()
            .find(|h| h.path == "sub/notes.md")
            .unwrap();
        assert_eq!(hit.line, Some(2));
    }

    #[test]
    fn refuses_to_delete_root() {
        let (_d, sfs) = setup();
        assert!(sfs.delete("").is_err());
    }

    #[test]
    fn delete_removes_files_and_dirs() {
        let (_d, sfs) = setup();
        sfs.delete("hello.txt").unwrap();
        assert!(sfs.read_file("hello.txt").is_err());
        sfs.delete("sub").unwrap();
        assert!(sfs.list_dir("sub").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_writes_through_symlink_to_outside() {
        let (_d, sfs) = setup();
        let outside = tempfile::tempdir().unwrap();
        // A symlink inside the root that points outside it.
        std::os::unix::fs::symlink(outside.path(), _d.path().join("link")).unwrap();

        // Even when neither the target nor its immediate parent exist yet, the
        // closest existing ancestor (the symlink) must be detected.
        let err = sfs
            .write_file("link/newdir/secret.txt", b"escape")
            .unwrap_err();
        assert!(matches!(err, BridgeError::PathEscapesRoot(_)));
        assert!(!outside.path().join("newdir").exists());
    }

    #[test]
    fn content_preview_handles_multibyte_utf8() {
        let (_d, sfs) = setup();
        // A long line of CJK characters: a naive `&s[..200]` byte slice would
        // panic in the middle of a multi-byte char.
        let long_line: String = "中文内容".repeat(100);
        sfs.write_file("cjk.txt", long_line.as_bytes()).unwrap();
        let hits = sfs.search("", "中文", true, 10).unwrap();
        assert!(hits.iter().any(|h| h.path == "cjk.txt"));
    }
}
