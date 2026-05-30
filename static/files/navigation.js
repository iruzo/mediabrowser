function navigateToDirectory(path) {
  window.location.href = path ? `/ui/${encodeURIPath(path)}/` : "/ui/";
}

function loadInitialDirectory() {
  const pathname = decodeURIComponent(window.location.pathname);

  const isFileUrl =
    pathname !== "/ui" && pathname !== "/ui/" && !pathname.endsWith("/");

  if (isFileUrl) {
    const filePath = pathname.substring(4).replace(/^\/+/, "");
    const lastSlashIndex = filePath.lastIndexOf("/");
    const dirPath = lastSlashIndex >= 0 ? filePath.substring(0, lastSlashIndex) : "";

    currentPath = dirPath;

    fetchDirectory(dirPath)
      .then((files) => {
        currentFiles = files;

        const file = currentFiles.find((f) => f.path === filePath);
        if (file) {
          selectedFile = file;
          currentMediaIndex = currentFiles.findIndex(
            (f) => f.path === file.path,
          );
          resetZoom();
          showCurrentMedia();
        }
      })
      .catch(() => {
        galleryGrid.innerHTML = "<div>error</div>";
      });
  } else {
    let dirPath = "";
    if (pathname.startsWith("/ui/") && pathname !== "/ui/") {
      dirPath = pathname.substring(4).replace(/\/$/, "");
    }

    currentPath = dirPath;

    fetchDirectory(dirPath)
      .then((files) => {
        currentDirectoryFiles = files;
        currentFiles = files;
        renderGallery(files);
      })
      .catch(() => {
        galleryGrid.innerHTML = "<div>error</div>";
      });
  }
}

async function fetchDirectory(path) {
  const response = await fetch(`/api/list?path=${encodeURIComponent(path)}`);
  if (!response.ok) {
    throw new Error("failed to load directory");
  }

  const files = await response.json();
  return files.map((file) => ({
    ...file,
    file_type: file.is_dir ? "directory" : determineFileType(file.name),
  }));
}

function refreshView() {
  navigateToDirectory(currentPath);
}
