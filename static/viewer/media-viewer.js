const LARGE_TEXT_FILE_SIZE = 512 * 1024;
const TEXT_CHUNK_SIZE = 64 * 1024;

let textStreamState = null;

function openMedia(file) {
  if (file.is_dir) {
    navigateToDirectory(file.path);
    return;
  }

  if (galleryContainer) {
    sessionStorage.setItem("galleryScrollPosition", galleryContainer.scrollTop);
  }

  const pathWithoutData = file.path.replace("/data", "");
  const fileUrl = "/ui" + encodeURIPath(pathWithoutData);
  window.location.href = fileUrl;
}

function showCurrentMedia() {
  if (currentMediaIndex < 0 || currentMediaIndex >= currentFiles.length) return;

  const file = currentFiles[currentMediaIndex];
  selectedFile = file;
  const viewer = document.getElementById("viewer");
  const content = document.getElementById("viewerContent");
  const zoomControls = document.getElementById("zoomControls");
  const loopControls = document.getElementById("loopControls");
  const saveTextBtn = document.getElementById("saveTextBtn");
  const pathWithoutData = file.path.replace("/data", "") || "/";
  const servePath = encodeURIPath(pathWithoutData);

  cleanupTextStreamViewer();

  if (file.file_type === "image") {
    content.innerHTML = `<img src="${servePath}" alt="${escapeHtml(file.name)}" id="viewerImage">`;
    setupMediaZoom();
    updateMediaTransform();
    zoomControls.style.display = "block";
    loopControls.style.display = "none";
    saveTextBtn.style.display = "none";
  } else if (file.file_type === "video") {
    content.innerHTML = `<video src="${servePath}" controls></video>`;
    setupMediaZoom();
    updateMediaTransform();
    zoomControls.style.display = "block";
    loopControls.style.display = "block";
    setupVideoLoop();
    saveTextBtn.style.display = "none";
  } else if (file.file_type === "audio") {
    content.innerHTML = `<audio src="${servePath}" controls></audio>`;
    zoomControls.style.display = "none";
    loopControls.style.display = "none";
    saveTextBtn.style.display = "none";
  } else {
    if (file.size > LARGE_TEXT_FILE_SIZE) {
      showLargeTextViewer(file, servePath, content);
    } else {
      showSmallTextEditor(servePath, content);
    }
    zoomControls.style.display = "none";
    loopControls.style.display = "none";
    saveTextBtn.style.display =
      file.size > LARGE_TEXT_FILE_SIZE ? "none" : "block";
  }

  viewer.classList.add("active");
  document.body.classList.add("viewer-open");
  document.getElementById("toolbarDropdown").classList.remove("open");
}

function showSmallTextEditor(servePath, content) {
  fetch(servePath)
    .then((response) => response.text())
    .then((text) => {
      const container = document.createElement("div");
      container.className = "text-editor-container";

      const textarea = document.createElement("textarea");
      textarea.className = "text-editor";
      textarea.id = "textEditor";
      textarea.value = text;

      container.appendChild(textarea);
      content.innerHTML = "";
      content.appendChild(container);
    })
    .catch(() => {
      content.innerHTML = `<div class="text-viewer">Failed to load file content</div>`;
    });
}

function showLargeTextViewer(file, servePath, content) {
  const viewer = document.createElement("div");
  viewer.className = "text-stream-viewer";

  const body = document.createElement("pre");
  body.className = "text-stream-content";

  const status = document.createElement("div");
  status.className = "text-stream-status";
  status.textContent = `0/${file.size} bytes`;

  viewer.appendChild(body);
  viewer.appendChild(status);
  content.innerHTML = "";
  content.appendChild(viewer);

  textStreamState = {
    filePath: file.path,
    servePath,
    viewer,
    body,
    status,
    offset: 0,
    size: file.size,
    loading: false,
    done: false,
    decoder: new TextDecoder(),
    onScroll: null,
  };

  textStreamState.onScroll = () => {
    if (
      !textStreamState ||
      textStreamState.loading ||
      textStreamState.done ||
      viewer.scrollTop + viewer.clientHeight < viewer.scrollHeight - 400
    ) {
      return;
    }

    loadNextTextChunk();
  };

  viewer.addEventListener("scroll", textStreamState.onScroll);
  loadNextTextChunk();
}

async function loadNextTextChunk() {
  if (!textStreamState || textStreamState.loading || textStreamState.done) {
    return;
  }

  const start = textStreamState.offset;
  const end = Math.min(textStreamState.size - 1, start + TEXT_CHUNK_SIZE - 1);

  textStreamState.loading = true;
  textStreamState.status.textContent = `${start}/${textStreamState.size} bytes`;

  try {
    const response = await fetch(textStreamState.servePath, {
      headers: {
        Range: `bytes=${start}-${end}`,
      },
    });

    if (!response.ok && response.status !== 206) {
      throw new Error("failed to load text chunk");
    }

    const chunk = new Uint8Array(await response.arrayBuffer());
    const text = textStreamState.decoder.decode(chunk, {
      stream: end + 1 < textStreamState.size,
    });

    textStreamState.body.textContent += text;
    textStreamState.offset = end + 1;
    textStreamState.done = textStreamState.offset >= textStreamState.size;

    if (textStreamState.done) {
      textStreamState.body.textContent += textStreamState.decoder.decode();
      textStreamState.status.textContent = `${textStreamState.size}/${textStreamState.size} bytes`;
    } else {
      textStreamState.status.textContent = `${textStreamState.offset}/${textStreamState.size} bytes`;
    }
  } catch {
    if (textStreamState) {
      textStreamState.status.textContent = "Failed to load file content";
    }
  } finally {
    if (textStreamState) {
      textStreamState.loading = false;
    }
  }
}

function cleanupTextStreamViewer() {
  if (!textStreamState) {
    return;
  }

  if (textStreamState.viewer && textStreamState.onScroll) {
    textStreamState.viewer.removeEventListener(
      "scroll",
      textStreamState.onScroll,
    );
  }

  textStreamState = null;
}

function nextMedia() {
  const mediaFiles = currentFiles.filter((f) => !f.is_dir);
  if (mediaFiles.length === 0) return;

  const currentMediaFile = mediaFiles.find((f) => f.path === selectedFile.path);
  const currentIndex = mediaFiles.indexOf(currentMediaFile);
  const nextIndex = currentIndex < mediaFiles.length - 1 ? currentIndex + 1 : 0;

  currentMediaIndex = currentFiles.findIndex(
    (f) => f.path === mediaFiles[nextIndex].path,
  );
  clearLoop();
  showCurrentMedia();
}

function previousMedia() {
  const mediaFiles = currentFiles.filter((f) => !f.is_dir);
  if (mediaFiles.length === 0) return;

  const currentMediaFile = mediaFiles.find((f) => f.path === selectedFile.path);
  const currentIndex = mediaFiles.indexOf(currentMediaFile);
  const prevIndex = currentIndex > 0 ? currentIndex - 1 : mediaFiles.length - 1;

  currentMediaIndex = currentFiles.findIndex(
    (f) => f.path === mediaFiles[prevIndex].path,
  );
  clearLoop();
  showCurrentMedia();
}

function closeViewer() {
  const video = document.querySelector("#viewerContent video");
  if (video) {
    video.pause();
    video.src = "";
    video.load();
  }

  const audio = document.querySelector("#viewerContent audio");
  if (audio) {
    audio.pause();
    audio.src = "";
    audio.load();
  }

  document.getElementById("zoomControls").style.display = "none";
  document.getElementById("loopControls").style.display = "none";
  document.getElementById("saveTextBtn").style.display = "none";

  document.getElementById("viewerDropdown").classList.remove("open");
  document.getElementById("viewer").classList.remove("active");
  document.body.classList.remove("viewer-open");
  selectedFile = null;
  cleanupTextStreamViewer();
  resetZoom();
  clearLoop();

  navigateToDirectory(currentPath);
}

function downloadCurrent() {
  if (selectedFile) {
    if (selectedFile.is_dir) {
      triggerDownload(
        `/api/download-multiple?paths=${encodeURIComponent(selectedFile.path)}`,
      );
    } else {
      triggerDownload(
        `/api/download?path=${encodeURIComponent(selectedFile.path)}`,
      );
    }
  }
}

function saveTextFile() {
  const textarea = document.getElementById("textEditor");
  if (!textarea || !selectedFile) return;

  if (!confirm(`Save changes to ${selectedFile.name}?`)) {
    return;
  }

  const content = textarea.value;

  fetch(`/api/save?path=${encodeURIComponent(selectedFile.path)}`, {
    method: "POST",
    headers: {
      "Content-Type": "text/plain",
    },
    body: content,
  })
    .then((response) => {
      if (response.ok) {
        alert("File saved successfully");
      } else {
        alert("Failed to save file");
      }
    })
    .catch((err) => {
      alert("Error saving file: " + err.message);
    });
}
