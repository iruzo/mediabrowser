initializeGridSize();
updateViewModeUI();

window.addEventListener('resize', resizeHandler);

document.addEventListener('click', function(e) {
    hideContextMenu();
    var viewerDropdown = document.getElementById('viewerDropdown');
    if (viewerDropdown && !viewerDropdown.contains(e.target)) {
        viewerDropdown.classList.remove('open');
    }
    var toolbarDropdown = document.getElementById('toolbarDropdown');
    if (toolbarDropdown && !toolbarDropdown.contains(e.target)) {
        toolbarDropdown.classList.remove('open');
    }
});
document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
        closeViewer();
        hideContextMenu();
    } else if (e.key === 'ArrowLeft') {
        previousMedia();
    } else if (e.key === 'ArrowRight') {
        nextMedia();
    }
});

loadInitialDirectory();
