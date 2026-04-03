function showAudioViewer(servePath, content, zoomControls, loopControls, saveTextBtn) {
  content.innerHTML = `<audio src="${servePath}" controls></audio>`;
  zoomControls.style.display = "none";
  loopControls.style.display = "none";
  saveTextBtn.style.display = "none";
}
