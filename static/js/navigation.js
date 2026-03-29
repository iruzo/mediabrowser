function navigateToDirectory(path) {
  const urlPath =
    path === "/data" ? "/ui/" : "/ui" + path.replace("/data", "") + "/";
  window.location.href = urlPath;
}

function loadInitialDirectory() {
  const pathname = decodeURIComponent(window.location.pathname);

  const isFileUrl =
    pathname !== "/ui" && pathname !== "/ui/" && !pathname.endsWith("/");

  if (isFileUrl) {
    const pathWithoutUI = pathname.substring(3);
    const lastSlashIndex = pathWithoutUI.lastIndexOf("/");
    const dirPathSuffix = pathWithoutUI.substring(0, lastSlashIndex);
    const dataPath = dirPathSuffix ? "/data" + dirPathSuffix : "/data";
    const filePath = "/data" + pathWithoutUI;

    currentPath = dataPath;

    fetchDirectory(dataPath)
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
    let dataPath = "/data";
    if (pathname.startsWith("/ui/") && pathname !== "/ui/") {
      const pathWithoutUI = pathname.substring(3);
      dataPath = "/data" + pathWithoutUI.slice(0, -1);
    }

    currentPath = dataPath;

    fetchDirectory(dataPath)
      .then((files) => {
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
