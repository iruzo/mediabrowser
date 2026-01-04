initializeGridSize();

window.addEventListener('resize', resizeHandler);

document.addEventListener('click', hideContextMenu);
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
