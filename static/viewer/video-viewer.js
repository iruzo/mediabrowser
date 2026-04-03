function showVideoViewer(file, servePath, content, zoomControls, loopControls, saveTextBtn) {
  content.innerHTML = `<video src="${servePath}" controls></video>`;
  setupMediaZoom();
  updateMediaTransform();
  zoomControls.style.display = "block";
  loopControls.style.display = "block";
  setupVideoLoop();
  saveTextBtn.style.display = "none";
}
