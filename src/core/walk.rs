use std::fs::{DirEntry, FileType};
use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct WalkEntry {
    pub(crate) path: PathBuf,
    pub(crate) file_type: FileType,
}

// Iterative (not recursive) so traversal depth can never overflow the native stack
pub(crate) fn walk(root: &Path) -> io::Result<Vec<WalkEntry>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let read_dir = std::fs::read_dir(&dir).map_err(|error| with_path(error, &dir))?;

        for entry in read_dir {
            let entry = entry.map_err(|error| with_path(error, &dir))?;
            let path = entry.path();
            let file_type = entry_file_type(&entry).map_err(|error| with_path(error, &path))?;

            if file_type.is_dir() {
                stack.push(path.clone());
            }
            out.push(WalkEntry { path, file_type });
        }
    }

    Ok(out)
}

fn with_path(error: io::Error, path: &Path) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}

// DirEntry::file_type() never makes a system call on unix (cached from the
// dirent readdir already returned), but on Windows it can misreport reparse
// points: https://github.com/rust-lang/rust/issues/46484
#[cfg(windows)]
fn entry_file_type(entry: &DirEntry) -> io::Result<FileType> {
    Ok(entry.metadata()?.file_type())
}

#[cfg(not(windows))]
fn entry_file_type(entry: &DirEntry) -> io::Result<FileType> {
    entry.file_type()
}
