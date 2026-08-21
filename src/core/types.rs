use std::io;
use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;
use tokio::fs;

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn data_dir() -> &'static Path {
    DATA_DIR
        .get_or_init(|| {
            std::env::var("DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data"))
        })
        .as_path()
}

pub fn data_path(path: &str) -> Option<PathBuf> {
    let path = Path::new(path.trim_start_matches('/'));

    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }

    Some(data_dir().join(path))
}

pub(crate) async fn ensure_no_symlinks(path: &Path) -> io::Result<()> {
    ensure_no_symlinks_from(data_dir(), path).await
}

pub(crate) async fn path_metadata(path: &Path) -> io::Result<std::fs::Metadata> {
    path_metadata_from(data_dir(), path).await
}

pub(crate) async fn create_data_dirs(path: &Path) -> io::Result<()> {
    create_data_dirs_from(data_dir(), path).await
}

async fn create_data_dirs_from(root: &Path, path: &Path) -> io::Result<()> {
    ensure_no_symlinks_from(root, path).await?;
    fs::create_dir_all(path).await?;

    let metadata = path_metadata_from(root, path).await?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(io::Error::from(io::ErrorKind::AlreadyExists))
    }
}

async fn path_metadata_from(root: &Path, path: &Path) -> io::Result<std::fs::Metadata> {
    no_symlink_metadata_from(root, path)
        .await?
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
}

async fn ensure_no_symlinks_from(root: &Path, path: &Path) -> io::Result<()> {
    no_symlink_metadata_from(root, path).await.map(|_| ())
}

async fn no_symlink_metadata_from(
    root: &Path,
    path: &Path,
) -> io::Result<Option<std::fs::Metadata>> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    let mut current = root.to_path_buf();
    // DATA_DIR is the trusted anchor; links are rejected below it.
    let mut metadata = match fs::metadata(&current).await {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };

    for component in relative.components() {
        if metadata.is_none() {
            break;
        }
        current.push(component);
        metadata = component_metadata(&current).await?;
    }

    Ok(metadata)
}

async fn component_metadata(path: &Path) -> io::Result<Option<std::fs::Metadata>> {
    match fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(io::Error::from(io::ErrorKind::NotFound))
        }
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::{create_data_dirs_from, ensure_no_symlinks_from, path_metadata_from};
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn treats_symlinks_as_missing() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let base = std::env::temp_dir().join(format!("mediabrowser-links-{suffix}"));
        let root = base.join("root");
        let outside = base.join("outside.txt");

        std::fs::create_dir_all(root.join("directory")).expect("create test directory");
        std::fs::write(root.join("file.txt"), b"file").expect("create test file");
        std::fs::write(&outside, b"outside").expect("create outside file");
        symlink("file.txt", root.join("file-link")).expect("create file link");
        symlink("directory", root.join("directory-link")).expect("create directory link");
        symlink("missing", root.join("broken-link")).expect("create broken link");
        symlink(&outside, root.join("outside-link")).expect("create outside link");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("create runtime");

        runtime.block_on(async {
            assert!(path_metadata_from(&root, &root.join("file.txt"))
                .await
                .expect("read regular file")
                .is_file());
            assert!(ensure_no_symlinks_from(&root, &root.join("new/path"))
                .await
                .is_ok());

            for path in [
                root.join("file-link"),
                root.join("directory-link"),
                root.join("directory-link/child"),
                root.join("broken-link"),
                root.join("outside-link"),
            ] {
                assert_eq!(
                    path_metadata_from(&root, &path)
                        .await
                        .expect_err("link must be missing")
                        .kind(),
                    std::io::ErrorKind::NotFound
                );
            }

            assert_eq!(
                create_data_dirs_from(&root, &root.join("directory-link/new"))
                    .await
                    .expect_err("linked parent must be missing")
                    .kind(),
                std::io::ErrorKind::NotFound
            );
            assert!(!root.join("directory/new").exists());

            create_data_dirs_from(&root, &root.join("new/directory"))
                .await
                .expect("create regular directories");
            assert!(root.join("new/directory").is_dir());
        });

        std::fs::remove_dir_all(base).expect("remove test files");
    }
}
