if (scope === null) {
  galleryBar.hidden = true;
  showDirectoryError("invalid path");
} else {
  loadDirectories();
}
