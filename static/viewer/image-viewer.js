function showImageViewer(file, servePath, content, zoomControls, loopControls, saveTextBtn) {
  content.innerHTML = `<img src="${servePath}" alt="${escapeHtml(file.name)}" id="viewerImage">`;
  setupMediaZoom();
  updateMediaTransform();
  zoomControls.style.display = "block";
  loopControls.style.display = "none";
  saveTextBtn.style.display = "none";
}
