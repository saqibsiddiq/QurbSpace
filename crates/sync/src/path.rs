//! Which paths another device may name.
//!
//! A path in a tree is a *logical* path: relative to the synced folder,
//! components separated by `/`. Every device that receives one joins it onto
//! a directory of its own and writes there. So a path is not just a name: it
//! is an instruction about where on this disk to write, or what to delete, and
//! it comes from another machine.
//!
//! Pairing says who that machine is. It says nothing about whether it is
//! telling the truth: a device that is stolen or compromised keeps its pinned
//! identity. A path of `../../.bashrc`, or `/etc/cron.d/x`, or one that
//! reaches into qurb's own store, must therefore be refused wherever one
//! arrives, not trusted because the sender was.

/// Whether `path` names somewhere inside the folder, and nowhere else.
///
/// Refused: empty paths, absolute paths, empty components (`a//b`, a trailing
/// `/`), `.` and `..` components, and NUL, which no filesystem this runs on
/// allows in a name and which some APIs treat as the end of the string.
///
/// On Windows also refused: backslashes, which Windows reads as separators,
/// so `..\..\x` would climb out; and colons, which name a drive (`C:\x`,
/// or `C:x`, relative to that drive's own directory) or a file's hidden
/// stream. Found by the platform tests' first run on Windows (2026-11-16): a
/// path naming a file elsewhere on the disk was written there.
///
/// Elsewhere backslashes are ordinary characters in a filename, and kept.
pub fn is_safe_path(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.contains('\0') {
        return false;
    }
    if cfg!(windows) && (path.contains('\\') || path.contains(':')) {
        return false;
    }
    path.split('/').all(|component| !matches!(component, "" | "." | ".."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_paths_are_safe() {
        for path in ["notes.txt", "photos/2026/beach.jpg", ".bashrc", "a..b", "...", "café/ü.txt"] {
            assert!(is_safe_path(path), "{path:?} should be allowed");
        }
    }

    #[test]
    fn paths_that_leave_the_folder_are_not() {
        for path in [
            "",
            "/etc/passwd",
            "../escape.txt",
            "photos/../../escape.txt",
            "..",
            ".",
            "./notes.txt",
            "photos//beach.jpg",
            "photos/",
            "nul\0byte",
        ] {
            assert!(!is_safe_path(path), "{path:?} should be refused");
        }
    }

    #[test]
    fn a_backslash_and_a_colon_are_names_or_ways_out_by_platform() {
        for path in ["back\\slash", "..\\..\\escape.txt", "C:\\Windows\\x.dll", "C:x", "notes.txt:hidden"] {
            assert_eq!(is_safe_path(path), !cfg!(windows), "{path:?}");
        }
    }
}
