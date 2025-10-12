let currentPath = '/data';
let currentFiles = [];
let selectedFile = null;
let currentMediaIndex = -1;
let currentFilter = 'all';
let currentSort = 'name';
let selectedFiles = new Set();
let selectionMode = false;
let searchTerm = '';
let intersectionObserver = null;
let gridSize = 30; // Default grid size in vh units

function loadDirectory(path = currentPath, pushState = true) {
    currentPath = path;

    // Clear selection and search when changing directories
    selectedFiles.clear();
    selectionMode = false;
    searchTerm = '';
    document.getElementById('searchInput').value = '';
    updateSelectionUI();

    if (pushState) {
        const pathHash = path === '/data' ? '' : encodeURIComponent(path.replace('/data', ''));
        window.history.pushState({ path: path }, '', pathHash ? `#dir${pathHash}` : '#');
    }

    fetch(`/api/list?path=${encodeURIComponent(path)}`)
        .then(response => response.json())
        .then(files => {
            currentFiles = files;
            renderGallery(files);
        })
        .catch(() => {
            document.getElementById('galleryGrid').innerHTML = '<div>error</div>';
        });
}

function renderGallery(files) {
    const grid = document.getElementById('galleryGrid');
    grid.innerHTML = '';

    // Disconnect previous observer
    if (intersectionObserver) {
        intersectionObserver.disconnect();
    }

    let filteredFiles = filterFiles(files);
    filteredFiles = filterBySearchTerm(filteredFiles);
    const sortedFiles = sortFiles(filteredFiles);

    sortedFiles.forEach(file => {
        const item = document.createElement('div');
        item.className = 'grid-item';
        item.dataset.filePath = file.path;
        item.dataset.fileType = file.file_type;
        item.dataset.fileName = file.name;
        item.onclick = (e) => handleFileClick(e, file);
        item.oncontextmenu = (e) => showContextMenu(e, file);

        if (selectedFiles.has(file.path)) {
            item.classList.add('selected');
        }

        if (file.is_dir) {
            item.innerHTML = `<div class="file-name">${escapeHtml(file.name)}</div>`;
            item.classList.add('directory');
        } else if (file.file_type === 'image') {
            // Create placeholder for lazy loading
            item.innerHTML = `<div class="file-name">${escapeHtml(file.name)}</div>`;
            item.classList.add('lazy-load');
        } else if (file.file_type === 'video') {
            // Create placeholder for lazy loading
            item.innerHTML = `<div class="file-name">${escapeHtml(file.name)}</div>`;
            item.classList.add('lazy-load');
        } else {
            item.innerHTML = `<div class="file-name">${escapeHtml(file.name)}</div>`;
            item.classList.add('file');
        }

        if (selectedFiles.has(file.path)) {
            const overlay = document.createElement('div');
            overlay.className = 'selection-overlay';
            item.appendChild(overlay);
        }

        grid.appendChild(item);
    });

    // Setup intersection observer for lazy loading
    setupLazyLoading();
}

function setupLazyLoading() {
    // Create intersection observer with some margin for preloading
    intersectionObserver = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                const item = entry.target;
                if (item.classList.contains('lazy-load')) {
                    loadItemContent(item);
                    intersectionObserver.unobserve(item);
                }
            }
        });
    }, {
        root: null,
        rootMargin: '50px', // Start loading 50px before entering viewport
        threshold: 0.1
    });

    // Observe all lazy-load items
    document.querySelectorAll('.lazy-load').forEach(item => {
        intersectionObserver.observe(item);
    });
}

function loadItemContent(item) {
    const filePath = item.dataset.filePath;
    const fileType = item.dataset.fileType;
    const fileName = item.dataset.fileName;

    item.classList.remove('lazy-load');

    if (fileType === 'image') {
        item.innerHTML = `<img src="/api/file?path=${encodeURIComponent(filePath)}" alt="${escapeHtml(fileName)}" loading="lazy">`;
    } else if (fileType === 'video') {
        item.innerHTML = `<video src="/api/file?path=${encodeURIComponent(filePath)}" muted>`;
    }

    // Re-add selection overlay if this item was selected
    if (selectedFiles.has(filePath)) {
        const overlay = document.createElement('div');
        overlay.className = 'selection-overlay';
        item.appendChild(overlay);
    }
}

function filterFiles(files) {
    if (currentFilter === 'all') return files;
    if (currentFilter === 'image') return files.filter(f => f.is_dir || f.file_type === 'image');
    if (currentFilter === 'video') return files.filter(f => f.is_dir || f.file_type === 'video');
    if (currentFilter === 'audio') return files.filter(f => f.is_dir || f.file_type === 'audio');
    if (currentFilter === 'text') return files.filter(f => f.is_dir || f.file_type === 'text');
    return files;
}

function changeFilter(filterType) {
    currentFilter = filterType;
    renderGallery(currentFiles);
}

function changeSort(sortType) {
    currentSort = sortType;
    renderGallery(currentFiles);
}

function filterBySearch(term) {
    searchTerm = term.toLowerCase();
    renderGallery(currentFiles);
}

function filterBySearchTerm(files) {
    if (!searchTerm) return files;
    return files.filter(file =>
        file.name.toLowerCase().includes(searchTerm)
    );
}

function sortFiles(files) {
    const sorted = [...files];

    sorted.sort((a, b) => {
        // Always put .. at the top
        if (a.name === '..') return -1;
        if (b.name === '..') return 1;

        // Then directories before files
        if (a.is_dir !== b.is_dir) {
            return a.is_dir ? -1 : 1;
        }

        // Then sort by selected criteria
        switch (currentSort) {
            case 'name':
                return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
            case 'date':
                return new Date(b.modified) - new Date(a.modified); // Newest first
            case 'size':
                return b.size - a.size; // Largest first
            case 'type':
                if (a.file_type !== b.file_type) {
                    return a.file_type.localeCompare(b.file_type);
                }
                return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
            default:
                return 0;
        }
    });

    return sorted;
}

function handleFileClick(e, file) {
    if (selectionMode || e.ctrlKey || e.metaKey) {
        // Multi-select mode (selection mode enabled or Ctrl/Cmd+click)
        e.preventDefault();
        toggleFileSelection(file);
    } else if (selectedFiles.size > 0) {
        // Clear selection and open file
        clearSelection();
        openMedia(file);
    } else {
        // Normal single file open
        openMedia(file);
    }
}

function toggleFileSelection(file) {
    if (selectedFiles.has(file.path)) {
        selectedFiles.delete(file.path);
    } else {
        selectedFiles.add(file.path);
    }
    updateSelectionUI();
    renderGallery(currentFiles);
}

function updateSelectionUI() {
    const hasSelection = selectedFiles.size > 0;
    const selectBtn = document.getElementById('selectBtn');

    // Update select button text and style
    selectBtn.textContent = selectionMode ? 'done' : 'select';
    if (selectionMode) {
        selectBtn.classList.add('active');
    } else {
        selectBtn.classList.remove('active');
    }

    document.getElementById('downloadBtn').style.display = hasSelection ? 'inline-block' : 'none';
    document.getElementById('deleteBtn').style.display = hasSelection ? 'inline-block' : 'none';
    document.getElementById('clearBtn').style.display = hasSelection ? 'inline-block' : 'none';
}

function clearSelection() {
    selectedFiles.clear();
    selectionMode = false;
    updateSelectionUI();
    renderGallery(currentFiles);
}

function toggleSelectionMode() {
    selectionMode = !selectionMode;
    if (!selectionMode) {
        // Exiting selection mode, clear selections
        selectedFiles.clear();
    }
    updateSelectionUI();
    renderGallery(currentFiles);
}

function downloadSelected() {
    if (selectedFiles.size === 0) return;

    selectedFiles.forEach(filePath => {
        window.open(`/api/download?path=${encodeURIComponent(filePath)}`);
    });
}

function deleteSelected() {
    if (selectedFiles.size === 0) return;

    const fileCount = selectedFiles.size;
    if (!confirm(`Delete ${fileCount} selected file(s)?`)) return;

    const deletePromises = Array.from(selectedFiles).map(filePath =>
        fetch(`/api/delete?path=${encodeURIComponent(filePath)}`, { method: 'DELETE' })
    );

    Promise.all(deletePromises)
        .then(() => {
            clearSelection();
            loadDirectory(currentPath, false);
        })
        .catch(() => alert('Some files failed to delete'));
}

let zoomLevel = 1;
let isDragging = false;
let dragStart = { x: 0, y: 0 };
let imagePos = { x: 0, y: 0 };

function openMedia(file) {
    if (file.is_dir) {
        loadDirectory(file.path, true);
        return;
    }

    selectedFile = file;
    currentMediaIndex = currentFiles.findIndex(f => f.path === file.path);
    // Reset zoom when opening a new image from gallery
    resetZoom();
    showCurrentMedia();
}

function showCurrentMedia() {
    if (currentMediaIndex < 0 || currentMediaIndex >= currentFiles.length) return;

    const file = currentFiles[currentMediaIndex];
    selectedFile = file;
    const viewer = document.getElementById('viewer');
    const content = document.getElementById('viewerContent');
    const zoomControls = document.getElementById('zoomControls');

    if (file.file_type === 'image') {
        content.innerHTML = `<img src="/api/file?path=${encodeURIComponent(file.path)}" alt="${escapeHtml(file.name)}" id="viewerImage">`;
        setupMediaZoom();
        // Keep current zoom level AND position
        updateMediaTransform();
        zoomControls.style.display = 'block';
    } else if (file.file_type === 'video') {
        content.innerHTML = `<video src="/api/file?path=${encodeURIComponent(file.path)}" controls></video>`;
        setupMediaZoom();
        // Keep current zoom level AND position
        updateMediaTransform();
        zoomControls.style.display = 'block';
    } else if (file.file_type === 'audio') {
        content.innerHTML = `<audio src="/api/file?path=${encodeURIComponent(file.path)}" controls></audio>`;
        zoomControls.style.display = 'none';
    } else {
        // Show text content for non-media files
        fetch(`/api/file?path=${encodeURIComponent(file.path)}`)
            .then(response => response.text())
            .then(text => {
                content.innerHTML = `<div class="text-viewer">${escapeHtml(text)}</div>`;
            })
            .catch(() => {
                content.innerHTML = `<div class="text-viewer">Failed to load file content</div>`;
            });
        zoomControls.style.display = 'none';
    }

    viewer.classList.add('active');

    // Don't modify history when opening from direct URL
    if (!window.location.hash.startsWith('#file')) {
        // Store current directory state before opening viewer
        const currentDirHash = currentPath === '/data' ? '#' : `#dir${encodeURIComponent(currentPath.replace('/data', ''))}`;
        window.history.replaceState({ path: currentPath }, '', currentDirHash);
        const fileHash = `#file${encodeURIComponent(file.path.replace('/data', ''))}`;
        window.history.pushState({ viewer: true, path: currentPath, file: file.path }, '', fileHash);
    }
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}


function nextMedia() {
    const mediaFiles = currentFiles.filter(f => !f.is_dir);
    if (mediaFiles.length === 0) return;

    const currentMediaFile = mediaFiles.find(f => f.path === selectedFile.path);
    const currentIndex = mediaFiles.indexOf(currentMediaFile);
    const nextIndex = currentIndex < mediaFiles.length - 1 ? currentIndex + 1 : 0;

    currentMediaIndex = currentFiles.findIndex(f => f.path === mediaFiles[nextIndex].path);

    // Update URL with new file
    const file = currentFiles[currentMediaIndex];
    const fileHash = `#file${file.path.replace('/data', '')}`;
    window.history.replaceState({ viewer: true, path: currentPath, file: file.path }, '', fileHash);

    showCurrentMedia();
}

function previousMedia() {
    const mediaFiles = currentFiles.filter(f => !f.is_dir);
    if (mediaFiles.length === 0) return;

    const currentMediaFile = mediaFiles.find(f => f.path === selectedFile.path);
    const currentIndex = mediaFiles.indexOf(currentMediaFile);
    const prevIndex = currentIndex > 0 ? currentIndex - 1 : mediaFiles.length - 1;

    currentMediaIndex = currentFiles.findIndex(f => f.path === mediaFiles[prevIndex].path);

    // Update URL with new file
    const file = currentFiles[currentMediaIndex];
    const fileHash = `#file${file.path.replace('/data', '')}`;
    window.history.replaceState({ viewer: true, path: currentPath, file: file.path }, '', fileHash);

    showCurrentMedia();
}

function setupMediaZoom() {
    const img = document.getElementById('viewerImage');
    const video = document.querySelector('#viewerContent video');
    const element = img || video;

    if (!element) return;

    element.addEventListener('wheel', (e) => {
        e.preventDefault();
        const delta = e.deltaY > 0 ? 0.9 : 1.1;
        zoomLevel *= delta;
        updateMediaTransform();
    });

    element.addEventListener('mousedown', (e) => {
        if (zoomLevel > 1) {
            e.stopPropagation();
            isDragging = true;
            dragStart.x = e.clientX - imagePos.x;
            dragStart.y = e.clientY - imagePos.y;
            element.classList.add('zoomed');
        }
    });

    document.addEventListener('mousemove', (e) => {
        if (isDragging) {
            imagePos.x = e.clientX - dragStart.x;
            imagePos.y = e.clientY - dragStart.y;
            updateMediaTransform();
        }
    });

    document.addEventListener('mouseup', () => {
        isDragging = false;
        const currentElement = document.getElementById('viewerImage') || document.querySelector('#viewerContent video');
        if (currentElement && zoomLevel <= 1) {
            currentElement.classList.remove('zoomed');
        }
    });

    element.addEventListener('dblclick', (e) => {
        if (zoomLevel === 1) {
            e.stopPropagation();
            zoomLevel = 2;
            updateMediaTransform();
        } else {
            e.stopPropagation();
            resetZoom();
        }
    });
}


function zoomIn() {
    zoomLevel *= 1.2;
    updateMediaTransform();
}

function zoomOut() {
    zoomLevel *= 0.8;
    if (zoomLevel < 1) zoomLevel = 1;
    updateMediaTransform();
}

function resetZoom() {
    zoomLevel = 1;
    imagePos = { x: 0, y: 0 };
    isDragging = false;
    updateMediaTransform();
}

function updateMediaTransform() {
    const img = document.getElementById('viewerImage');
    const video = document.querySelector('#viewerContent video');
    const element = img || video;

    if (!element) return;

    if (zoomLevel > 1) {
        element.classList.add('zoomed');
        element.style.transform = `scale(${zoomLevel}) translate(${imagePos.x / zoomLevel}px, ${imagePos.y / zoomLevel}px)`;
    } else {
        element.classList.remove('zoomed');
        element.style.transform = 'scale(1) translate(0, 0)';
        imagePos = { x: 0, y: 0 };
    }
}

function closeViewer() {
    // Stop any playing videos or audio
    const video = document.querySelector('#viewerContent video');
    if (video) {
        video.pause();
        video.src = '';
        video.load();
    }

    const audio = document.querySelector('#viewerContent audio');
    if (audio) {
        audio.pause();
        audio.src = '';
        audio.load();
    }

    // Hide zoom controls
    document.getElementById('zoomControls').style.display = 'none';

    document.getElementById('viewer').classList.remove('active');
    selectedFile = null;
    resetZoom();

    if (window.location.hash.startsWith('#file')) {
        // Navigate to the directory instead of going back
        const currentDirHash = currentPath === '/data' ? '#' : `#dir${encodeURIComponent(currentPath.replace('/data', ''))}`;
        window.location.hash = currentDirHash;
    }
}

function goUp() {
    const parent = currentPath.substring(0, currentPath.lastIndexOf('/')) || '/data';
    if (parent !== '/data' || currentPath !== '/data') {
        loadDirectory(parent, true);
    }
}

function triggerUpload() {
    document.getElementById('fileInput').click();
}

function uploadFiles(files) {
    const formData = new FormData();
    for (const file of files) {
        formData.append('file', file);
    }

    fetch(`/api/upload?path=${encodeURIComponent(currentPath)}`, {
        method: 'POST',
        body: formData
    })
    .then(() => loadDirectory(currentPath, false))
    .catch(() => alert('upload failed'));
}

function createFolder() {
    const name = prompt('folder name:');
    if (name) {
        const folderPath = `${currentPath}/${name}`;
        fetch(`/api/mkdir?path=${encodeURIComponent(folderPath)}`, { method: 'POST' })
            .then(() => loadDirectory(currentPath, false))
            .catch(() => alert('failed to create folder'));
    }
}

function refreshView() {
    loadDirectory(currentPath, false);
}

function showContextMenu(e, file) {
    e.preventDefault();
    selectedFile = file;
    const menu = document.getElementById('contextMenu');
    menu.style.display = 'block';
    menu.style.left = e.pageX + 'px';
    menu.style.top = e.pageY + 'px';
}

function downloadFile() {
    if (selectedFile) {
        window.open(`/api/download?path=${encodeURIComponent(selectedFile.path)}`);
    }
    hideContextMenu();
}

function deleteFile() {
    if (selectedFile && confirm(`delete ${selectedFile.name}?`)) {
        fetch(`/api/delete?path=${encodeURIComponent(selectedFile.path)}`, { method: 'DELETE' })
            .then(() => loadDirectory(currentPath, false))
            .catch(() => alert('delete failed'));
    }
    hideContextMenu();
}

function downloadCurrent() {
    if (selectedFile) {
        window.open(`/api/download?path=${encodeURIComponent(selectedFile.path)}`);
    }
}

function hideContextMenu() {
    document.getElementById('contextMenu').style.display = 'none';
}

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

window.addEventListener('popstate', (e) => {
    if (document.getElementById('viewer').classList.contains('active')) {
        // Stop any playing videos or audio
        const video = document.querySelector('#viewerContent video');
        if (video) {
            video.pause();
            video.src = '';
            video.load();
        }

        const audio = document.querySelector('#viewerContent audio');
        if (audio) {
            audio.pause();
            audio.src = '';
            audio.load();
        }

        // Hide zoom controls
        document.getElementById('zoomControls').style.display = 'none';

        // Close viewer without triggering another history change
        document.getElementById('viewer').classList.remove('active');
        selectedFile = null;
        resetZoom();
        return;
    }

    handleUrlChange();
});

function handleUrlChange() {
    if (window.location.hash.startsWith('#file')) {
        const fileHashPath = decodeURIComponent(window.location.hash.replace('#file', ''));
        const filePath = '/data' + fileHashPath;

        // Calculate directory path correctly
        let dirPath;
        if (fileHashPath.includes('/')) {
            // File is in a subdirectory
            const lastSlashIndex = fileHashPath.lastIndexOf('/');
            dirPath = '/data' + fileHashPath.substring(0, lastSlashIndex);
        } else {
            // File is in root directory
            dirPath = '/data';
        }

        // Load directory first and then open file
        loadDirectoryAndOpenFile(dirPath, filePath);

    } else if (window.location.hash.startsWith('#dir')) {
        const path = '/data' + decodeURIComponent(window.location.hash.replace('#dir', ''));
        loadDirectory(path, false);
    } else if (!window.location.hash || window.location.hash === '#') {
        loadDirectory('/data', false);
    }
}

function loadDirectoryAndOpenFile(dirPath, filePath) {
    currentPath = dirPath;
    document.getElementById('pathIndicator').textContent = dirPath.replace('/data', '') || '/';

    fetch(`/api/list?path=${encodeURIComponent(dirPath)}`)
        .then(response => response.json())
        .then(files => {
            currentFiles = files;
            renderGallery(files);

            // Now find and open the file
            const file = currentFiles.find(f => f.path === filePath);
            if (file) {
                selectedFile = file;
                currentMediaIndex = currentFiles.findIndex(f => f.path === file.path);
                resetZoom();
                showCurrentMedia();
            }
        })
        .catch(() => {
            document.getElementById('galleryGrid').innerHTML = '<div>error loading directory</div>';
        });
}

// Grid size controls
function increaseGridSize() {
    gridSize = Math.min(gridSize + 5, 35); // Max size 30vh
    updateGridSize();
}

function decreaseGridSize() {
    gridSize = Math.max(gridSize - 5, 10); // Min size 8vh
    updateGridSize();
}

function updateGridSize() {
    document.documentElement.style.setProperty('--grid-size', gridSize + 'vh');
    // Store in localStorage for persistence
    localStorage.setItem('gridSize', gridSize);
}

// Load saved grid size on startup
function initializeGridSize() {
    const savedSize = localStorage.getItem('gridSize');
    if (savedSize) {
        gridSize = parseInt(savedSize);
    }
    updateGridSize();
}

// Initialize grid size
initializeGridSize();

// Handle initial URL on page load
if (window.location.hash) {
    handleUrlChange();
} else {
    loadDirectory();
}
