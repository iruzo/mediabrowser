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
    showImageViewer(file, servePath, content, zoomControls, loopControls, saveTextBtn);
  } else if (file.file_type === "video") {
    showVideoViewer(file, servePath, content, zoomControls, loopControls, saveTextBtn);
  } else if (file.file_type === "audio") {
    showAudioViewer(servePath, content, zoomControls, loopControls, saveTextBtn);
  } else {
    showTextViewer(file, servePath, content, zoomControls, loopControls, saveTextBtn);
  }

  viewer.classList.add("active");
  document.body.classList.add("viewer-open");
  document.getElementById("toolbarDropdown").classList.remove("open");
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
