initializeGridSize();
updateViewModeUI();

window.addEventListener('resize', resizeHandler);

document.addEventListener('click', function(e) {
    hideContextMenu();
    var dropdown = document.getElementById('viewerDropdown');
    if (dropdown && !dropdown.contains(e.target)) {
        dropdown.classList.remove('open');
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
