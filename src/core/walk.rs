use std::fs::{DirEntry, FileType};
use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct WalkEntry {
    pub(crate) path: PathBuf,
    pub(crate) file_type: FileType,
}

// Iterative (not recursive) so traversal depth can never overflow the native stack
pub(crate) fn walk(root: &Path) -> io::Result<Vec<WalkEntry>> {
    if std::fs::symlink_metadata(root)?.file_type().is_symlink() {
        return Err(io::Error::from(io::ErrorKind::NotFound));
    }

    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let read_dir = std::fs::read_dir(&dir).map_err(|error| with_path(error, &dir))?;

        for entry in read_dir {
            let entry = entry.map_err(|error| with_path(error, &dir))?;
            let path = entry.path();
            let file_type = entry_file_type(&entry).map_err(|error| with_path(error, &path))?;

            if file_type.is_symlink() {
                continue;
            }

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

#[cfg(windows)]
fn entry_file_type(entry: &DirEntry) -> io::Result<FileType> {
    Ok(std::fs::symlink_metadata(entry.path())?.file_type())
}

#[cfg(not(windows))]
fn entry_file_type(entry: &DirEntry) -> io::Result<FileType> {
    entry.file_type()
}

#[cfg(all(test, unix))]
mod tests {
    use super::walk;
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn skips_symlinks() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("mediabrowser-walk-{suffix}"));
        std::fs::create_dir_all(root.join("directory")).expect("create test directory");
        std::fs::write(root.join("file.txt"), []).expect("create test file");
        std::fs::write(root.join("directory/nested.txt"), []).expect("create nested file");
        symlink("file.txt", root.join("file-link")).expect("create file link");
        symlink("directory", root.join("directory-link")).expect("create directory link");

        let mut paths = walk(&root)
            .expect("walk test directory")
            .into_iter()
            .map(|entry| {
                entry
                    .path
                    .strip_prefix(&root)
                    .expect("entry below root")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect::<Vec<_>>();
        paths.sort_unstable();

        assert_eq!(paths, ["directory", "directory/nested.txt", "file.txt"]);

        assert_eq!(
            walk(&root.join("directory-link"))
                .err()
                .expect("linked root must be missing")
                .kind(),
            std::io::ErrorKind::NotFound
        );
        std::fs::remove_dir_all(root).expect("remove test directory");
    }
}
