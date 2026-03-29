initializeGridSize();
updateNameModeUI();

window.addEventListener('resize', resizeHandler);
window.addEventListener('resize', updateToolbarDropdownPosition);

document.addEventListener('click', function(e) {
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
    } else if (e.key === 'ArrowLeft') {
        previousMedia();
    } else if (e.key === 'ArrowRight') {
        nextMedia();
    }
});

function updateToolbarDropdownPosition() {
    const toolbarDropdown = document.getElementById('toolbarDropdown');
    if (!toolbarDropdown) {
        return;
    }

    const visualViewport = window.visualViewport;
    if (!visualViewport) {
        toolbarDropdown.style.bottom = '';
        return;
    }

    const layoutHeight = window.innerHeight;
    const visibleBottom = visualViewport.height + visualViewport.offsetTop;
    const keyboardInset = Math.max(0, layoutHeight - visibleBottom);

    toolbarDropdown.style.bottom = `${keyboardInset}px`;
}

if (window.visualViewport) {
    window.visualViewport.addEventListener('resize', updateToolbarDropdownPosition);
    window.visualViewport.addEventListener('scroll', updateToolbarDropdownPosition);
}

const searchInput = document.getElementById('searchInput');
if (searchInput) {
    searchInput.addEventListener('focus', updateToolbarDropdownPosition);
    searchInput.addEventListener('blur', updateToolbarDropdownPosition);
}

updateToolbarDropdownPosition();

loadInitialDirectory();
