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
let gridSize = window.innerWidth <= 480 ? 20 : 30;

// Video loop state
let loopEnabled = false;
let loopStart = 0;
let loopEnd = 0;

// Video load queue to prevent browser connection overload
const videoLoadQueue = {
    queue: [],
    loading: 0,
    maxConcurrent: 6, // Browser connection limit

    add(item, filePath, servePath) {
        this.queue.push({ item, filePath, servePath });
        this.process();
    },

    process() {
        while (this.loading < this.maxConcurrent && this.queue.length > 0) {
            const { item, filePath, servePath } = this.queue.shift();
            this.loading++;
            this.loadVideo(item, filePath, servePath);
        }
    },

    loadVideo(item, filePath, servePath) {
        const video = document.createElement('video');
        video.src = servePath;
        video.muted = true;
        video.loading = 'lazy';

        const onLoadComplete = () => {
            this.loading--;
            this.process(); // Process next in queue
        };

        video.addEventListener('loadeddata', onLoadComplete, { once: true });
        video.addEventListener('error', onLoadComplete, { once: true });

        item.innerHTML = '';
        item.appendChild(video);

        // Re-add selection overlay if needed
        if (selectedFiles.has(filePath)) {
            const overlay = document.createElement('div');
            overlay.className = 'selection-overlay';
            item.appendChild(overlay);
        }
    },

    clear() {
        this.queue = [];
        // Don't reset loading count - let in-progress videos finish
    }
};

// Performance utilities
const performanceUtils = {
    throttle(func, delay) {
        let timeoutId;
        let lastExecTime = 0;
        return function (...args) {
            const currentTime = Date.now();
            if (currentTime - lastExecTime > delay) {
                func.apply(this, args);
                lastExecTime = currentTime;
            } else {
                clearTimeout(timeoutId);
                timeoutId = setTimeout(() => {
                    func.apply(this, args);
                    lastExecTime = Date.now();
                }, delay - (currentTime - lastExecTime));
            }
        };
    },

    debounce(func, delay) {
        let timeoutId;
        return function (...args) {
            clearTimeout(timeoutId);
            timeoutId = setTimeout(() => func.apply(this, args), delay);
        };
    }
};

// Virtual scrolling variables
let virtualScrollData = {
    filteredFiles: [],
    renderedItems: new Map(), // Track rendered DOM elements
    scrollPosition: 0,
    itemHeight: 200, // Approximate item height in pixels
    containerHeight: 0,
    scrollContainer: null,
    visibleRange: { start: 0, end: 0 },
    lastContainerDimensions: { width: 0, height: 0 },
    selectedItemsCache: new Set() // Cache for selection state
};

// Navigate to directory with full page reload
function navigateToDirectory(path) {
    const urlPath = path === '/data' ? '/ui/' : '/ui' + path.replace('/data', '') + '/';
    window.location.href = urlPath;
}

// Load initial directory from current URL
function loadInitialDirectory() {
    const pathname = decodeURIComponent(window.location.pathname);

    // Check if URL points to a file (no trailing slash, not just /ui or /ui/)
    const isFileUrl = pathname !== '/ui' && pathname !== '/ui/' && !pathname.endsWith('/');

    if (isFileUrl) {
        // Extract directory and file paths
        const pathWithoutUI = pathname.substring(3); // Remove '/ui'
        const lastSlashIndex = pathWithoutUI.lastIndexOf('/');
        const dirPathSuffix = pathWithoutUI.substring(0, lastSlashIndex);
        const dataPath = dirPathSuffix ? '/data' + dirPathSuffix : '/data';
        const filePath = '/data' + pathWithoutUI;

        currentPath = dataPath;

        // Load directory and then open the file
        const servePath = dataPath === '/data' ? '/' : dataPath.replace('/data', '') + '/';
        fetch(servePath)
            .then(response => response.text())
            .then(html => {
                const files = parseServerHtml(html, dataPath);
                currentFiles = files;
                // Don't render gallery yet - it's hidden behind the viewer
                // It will render when viewer is closed

                // Find and open the file
                const file = currentFiles.find(f => f.path === filePath);
                if (file) {
                    selectedFile = file;
                    currentMediaIndex = currentFiles.findIndex(f => f.path === file.path);
                    resetZoom();
                    showCurrentMedia();
                }
            })
            .catch(() => {
                document.getElementById('galleryGrid').innerHTML = '<div>error</div>';
            });
    } else {
        // Directory URL
        let dataPath = '/data';
        if (pathname.startsWith('/ui/') && pathname !== '/ui/') {
            const pathWithoutUI = pathname.substring(3); // Remove '/ui'
            dataPath = '/data' + pathWithoutUI.slice(0, -1);
        }

        currentPath = dataPath;

        // Fetch and parse the directory listing
        const servePath = dataPath === '/data' ? '/' : dataPath.replace('/data', '') + '/';
        fetch(servePath)
            .then(response => response.text())
            .then(html => {
                const files = parseServerHtml(html, dataPath);
                currentFiles = files;
                renderGallery(files);
            })
            .catch(() => {
                document.getElementById('galleryGrid').innerHTML = '<div>error</div>';
            });
    }
}

function parseServerHtml(html, currentPath) {
    const parser = new DOMParser();
    const doc = parser.parseFromString(html, 'text/html');
    const links = doc.querySelectorAll('ul li a');
    const files = [];

    links.forEach(link => {
        const href = link.getAttribute('href');
        const text = link.textContent.trim();

        if (!href || href === '../') return;

        const isDir = href.endsWith('/');
        const name = isDir ? text.replace(/\/$/, '') : text;
        const filePath = currentPath === '/data'
            ? `/data/${name}`
            : `${currentPath}/${name}`;

        const fileType = isDir ? 'directory' : determineFileType(name);

        files.push({
            name: name,
            path: filePath,
            is_dir: isDir,
            file_type: fileType,
            size: 0,
            modified: ''
        });
    });

    return files;
}

function determineFileType(filename) {
    const ext = filename.split('.').pop().toLowerCase();

    const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'svg', 'ico'];
    const videoExts = ['mp4', 'avi', 'mkv', 'mov', 'wmv', 'flv', 'webm', 'm4v', 'ogv'];
    const audioExts = ['mp3', 'wav', 'flac', 'aac', 'ogg', 'wma', 'm4a'];

    if (imageExts.includes(ext)) return 'image';
    if (videoExts.includes(ext)) return 'video';
    if (audioExts.includes(ext)) return 'audio';

    // Everything else opens with text editor
    return 'text';
}

function renderGallery(files) {
    const grid = document.getElementById('galleryGrid');

    // Disconnect previous observer
    if (intersectionObserver) {
        intersectionObserver.disconnect();
    }

    // Clear video load queue when re-rendering
    videoLoadQueue.clear();

    let filteredFiles = filterFiles(files);
    filteredFiles = filterBySearchTerm(filteredFiles);
    virtualScrollData.filteredFiles = sortFiles(filteredFiles);

    // Initialize virtual scrolling
    initializeVirtualScroll(grid);

    // Restore scroll position if returning from file view
    const savedScroll = sessionStorage.getItem('galleryScrollPosition');
    if (savedScroll) {
        const container = document.querySelector('.gallery-container');
        if (container) {
            // Use requestAnimationFrame to ensure DOM is ready
            requestAnimationFrame(() => {
                container.scrollTop = parseInt(savedScroll);
                sessionStorage.removeItem('galleryScrollPosition');
            });
        }
    }
}

function initializeVirtualScroll(grid) {
    // Clear existing content and event listeners
    grid.innerHTML = '';
    virtualScrollData.renderedItems.clear();

    if (virtualScrollData.scrollContainer) {
        virtualScrollData.scrollContainer.removeEventListener('scroll', handleVirtualScroll);
    }

    // Set up container for virtual scrolling
    const container = grid.parentElement; // gallery-container
    virtualScrollData.scrollContainer = container;
    virtualScrollData.containerHeight = container.clientHeight;

    // Cache container dimensions
    virtualScrollData.lastContainerDimensions = {
        width: container.clientWidth,
        height: container.clientHeight
    };

    // Calculate grid dimensions
    const gridSizeVh = parseInt(getComputedStyle(document.documentElement).getPropertyValue('--grid-size')) || 30;
    const baseGridSizePx = (gridSizeVh / 100) * window.innerHeight;

    // Calculate columns and adjust item size to fill width
    const containerWidth = container.clientWidth - 4; // Account for padding
    const minColumns = Math.max(1, Math.floor(containerWidth / baseGridSizePx)); // Ensure at least 1 column
    const actualGridSizePx = minColumns === 1 ?
        containerWidth :
        (containerWidth - (minColumns - 1) * 2) / minColumns; // Distribute space evenly with 2px gaps

    virtualScrollData.itemHeight = actualGridSizePx + 2; // Add gap
    virtualScrollData.actualItemSize = actualGridSizePx;
    virtualScrollData.columns = minColumns;
    virtualScrollData.rows = Math.ceil(virtualScrollData.filteredFiles.length / minColumns);

    // Create spacer div to represent total height
    const spacer = document.createElement('div');
    spacer.id = 'virtual-spacer';
    spacer.style.height = `${virtualScrollData.rows * virtualScrollData.itemHeight}px`;
    spacer.style.position = 'relative';
    grid.appendChild(spacer);

    // Add throttled scroll listener
    const throttledScroll = performanceUtils.throttle(handleVirtualScroll, 16); // 60fps
    container.addEventListener('scroll', throttledScroll);

    // Initial render
    renderVisibleItems();
}

function handleVirtualScroll() {
    virtualScrollData.scrollPosition = virtualScrollData.scrollContainer.scrollTop;
    renderVisibleItems();
}

function renderVisibleItems() {
    const containerHeight = virtualScrollData.scrollContainer.clientHeight;
    const scrollTop = virtualScrollData.scrollPosition;
    const itemHeight = virtualScrollData.itemHeight;
    const columns = virtualScrollData.columns;

    if (columns <= 0) return;

    // Calculate which rows should be visible (minimal buffer)
    const startRow = Math.max(0, Math.floor(scrollTop / itemHeight) - 1); // 1 row buffer above
    const endRow = Math.min(
        virtualScrollData.rows - 1,
        Math.ceil((scrollTop + containerHeight) / itemHeight) + 1 // 1 row buffer below
    );

    // Convert rows to item indices
    const startIndex = startRow * columns;
    const endIndex = Math.min(virtualScrollData.filteredFiles.length - 1, ((endRow + 1) * columns) - 1);

    virtualScrollData.visibleRange = { start: startIndex, end: endIndex };

    const spacer = document.getElementById('virtual-spacer');

    // Remove items that are no longer visible with proper cleanup
    for (const [index, item] of virtualScrollData.renderedItems) {
        if (index < startIndex || index > endIndex) {
            // Clean up event listeners and observers
            item.onclick = null;
            item.oncontextmenu = null;

            // Clean up image/video elements
            const img = item.querySelector('img');
            const video = item.querySelector('video');
            if (img) {
                img.onload = null;
                img.onerror = null;
                img.src = '';
            }
            if (video) {
                video.src = '';
                video.load();
            }

            // Remove from observer and DOM
            if (intersectionObserver) {
                intersectionObserver.unobserve(item);
            }
            item.remove();
            virtualScrollData.renderedItems.delete(index);
        }
    }

    // Add new visible items
    for (let i = startIndex; i <= endIndex; i++) {
        if (!virtualScrollData.renderedItems.has(i) && i < virtualScrollData.filteredFiles.length) {
            const file = virtualScrollData.filteredFiles[i];
            const item = createGridItem(file, i);
            virtualScrollData.renderedItems.set(i, item);
            spacer.appendChild(item);
        }
    }

    // Setup lazy loading for newly added items
    setupLazyLoading();
}

function createGridItem(file, index) {
    const item = document.createElement('div');
    item.className = 'grid-item';
    item.dataset.filePath = file.path;
    item.dataset.fileType = file.file_type;
    item.dataset.fileName = file.name;
    item.onclick = (e) => handleFileClick(e, file);
    item.oncontextmenu = (e) => showContextMenu(e, file);

    // Calculate grid position
    const columns = virtualScrollData.columns;
    const row = Math.floor(index / columns);
    const col = index % columns;

    // Use calculated grid size
    const gridSizePx = virtualScrollData.actualItemSize;

    // Position item absolutely within the spacer
    item.style.position = 'absolute';
    item.style.top = `${row * virtualScrollData.itemHeight + 2}px`; // Add small gap
    item.style.left = `${col * (gridSizePx + 2) + 2}px`;
    item.style.width = `${gridSizePx}px`;
    item.style.height = `${gridSizePx}px`;

    if (selectedFiles.has(file.path)) {
        item.classList.add('selected');
    }

    if (file.is_dir) {
        item.innerHTML = `<div class="file-name">${escapeHtml(file.name)}</div>`;
        item.classList.add('directory');
    } else if (file.file_type === 'image') {
        item.innerHTML = `<div class="file-name">${escapeHtml(file.name)}</div>`;
        item.classList.add('lazy-load');
    } else if (file.file_type === 'video') {
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

    return item;
}

function setupLazyLoading() {
    if (intersectionObserver) {
        intersectionObserver.disconnect();
    }

    intersectionObserver = new IntersectionObserver((entries) => {
        // Process entries in batches to avoid blocking
        const processEntry = (entry) => {
            if (entry.isIntersecting) {
                const item = entry.target;
                if (item.classList.contains('lazy-load')) {
                    loadItemContent(item);
                    intersectionObserver.unobserve(item);
                }
            }
        };

        // Process entries with requestAnimationFrame for better performance
        const processEntries = () => {
            entries.forEach(processEntry);
        };
        requestAnimationFrame(processEntries);
    }, {
        root: null,
        rootMargin: '25px', // Reduced from 50px for less aggressive preloading
        threshold: 0.2 // Increased from 0.1 for fewer callbacks
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
        // Add WebP support detection and progressive loading
        const img = document.createElement('img');
        img.loading = 'lazy';
        img.alt = escapeHtml(fileName);
        img.style.opacity = '0';
        img.style.transition = 'opacity 0.2s';

        img.onload = () => {
            img.style.opacity = '1';
        };

        img.onerror = () => {
            img.style.opacity = '0.5';
            img.alt = 'Failed to load';
        };

        const pathWithoutData = filePath.replace('/data', '') || '/';
        const servePath = encodeURIPath(pathWithoutData);
        img.src = servePath;
        item.innerHTML = '';
        item.appendChild(img);
    } else if (fileType === 'video') {
        const pathWithoutData = filePath.replace('/data', '') || '/';
        const servePath = encodeURIPath(pathWithoutData);

        // Add to video load queue instead of loading immediately
        videoLoadQueue.add(item, filePath, servePath);
        return; // Queue will handle adding selection overlay
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

// Debounced search function
const debouncedFilterBySearch = performanceUtils.debounce((term) => {
    searchTerm = term.toLowerCase();
    renderGallery(currentFiles);
}, 300);

function filterBySearch(term) {
    // Update search term immediately for responsiveness
    searchTerm = term.toLowerCase();
    // But debounce the actual filtering/rendering
    debouncedFilterBySearch(term);
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

    // Update virtual rendered items selection state
    updateVirtualItemsSelection();
}

function updateVirtualItemsSelection() {
    // Create a new cache of currently selected items
    const currentSelected = new Set(selectedFiles);

    // Only update items whose selection state has changed
    for (const [index, item] of virtualScrollData.renderedItems) {
        const file = virtualScrollData.filteredFiles[index];
        if (file) {
            const isSelected = currentSelected.has(file.path);
            const wasSelected = virtualScrollData.selectedItemsCache.has(file.path);

            // Only update if selection state changed
            if (isSelected !== wasSelected) {
                item.classList.toggle('selected', isSelected);

                // Handle selection overlay
                const existingOverlay = item.querySelector('.selection-overlay');
                if (isSelected && !existingOverlay) {
                    const overlay = document.createElement('div');
                    overlay.className = 'selection-overlay';
                    item.appendChild(overlay);
                } else if (!isSelected && existingOverlay) {
                    existingOverlay.remove();
                }
            }
        }
    }

    // Update selection cache
    virtualScrollData.selectedItemsCache = currentSelected;
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

    const selectedPaths = Array.from(selectedFiles);

    // Check if any selected item is a directory
    const hasDirectory = selectedPaths.some(path => {
        const file = currentFiles.find(f => f.path === path);
        return file && file.is_dir;
    });

    // Use ZIP download if multiple files or if any selection is a directory
    if (selectedFiles.size === 1 && !hasDirectory) {
        const filePath = selectedPaths[0];
        window.open(`/api/download?path=${encodeURIComponent(filePath)}`);
    } else {
        const paths = selectedPaths.map(path => encodeURIComponent(path)).join(',');
        window.open(`/api/download-multiple?paths=${paths}`);
    }
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
            navigateToDirectory(currentPath);
        })
        .catch(() => alert('Some files failed to delete'));
}

let zoomLevel = 1;
let isDragging = false;
let dragStart = { x: 0, y: 0 };
let imagePos = { x: 0, y: 0 };

function openMedia(file) {
    if (file.is_dir) {
        navigateToDirectory(file.path);
        return;
    }

    // Save scroll position before navigating to file
    const container = document.querySelector('.gallery-container');
    if (container) {
        sessionStorage.setItem('galleryScrollPosition', container.scrollTop);
    }

    // Navigate to file URL with full page reload
    const pathWithoutData = file.path.replace('/data', '');
    const fileUrl = '/ui' + encodeURIPath(pathWithoutData);
    window.location.href = fileUrl;
}

function showCurrentMedia() {
    if (currentMediaIndex < 0 || currentMediaIndex >= currentFiles.length) return;

    const file = currentFiles[currentMediaIndex];
    selectedFile = file;
    const viewer = document.getElementById('viewer');
    const content = document.getElementById('viewerContent');
    const zoomControls = document.getElementById('zoomControls');
    const loopControls = document.getElementById('loopControls');
    const pathWithoutData = file.path.replace('/data', '') || '/';
    const servePath = encodeURIPath(pathWithoutData);

    if (file.file_type === 'image') {
        content.innerHTML = `<img src="${servePath}" alt="${escapeHtml(file.name)}" id="viewerImage">`;
        setupMediaZoom();
        // Keep current zoom level AND position
        updateMediaTransform();
        zoomControls.style.display = 'block';
        loopControls.style.display = 'none';
    } else if (file.file_type === 'video') {
        content.innerHTML = `<video src="${servePath}" controls></video>`;
        setupMediaZoom();
        // Keep current zoom level AND position
        updateMediaTransform();
        zoomControls.style.display = 'block';
        loopControls.style.display = 'block';
        setupVideoLoop();
    } else if (file.file_type === 'audio') {
        content.innerHTML = `<audio src="${servePath}" controls></audio>`;
        zoomControls.style.display = 'none';
        loopControls.style.display = 'none';
    } else {
        // Show text content for non-media files
        fetch(servePath)
            .then(response => response.text())
            .then(text => {
                content.innerHTML = `
                    <div class="text-editor-container">
                        <textarea class="text-editor" id="textEditor">${escapeHtml(text)}</textarea>
                        <div class="text-editor-controls">
                            <button class="viewer-btn" onclick="saveTextFile()">save</button>
                            <button class="viewer-btn" onclick="cancelTextEdit()">cancel</button>
                        </div>
                    </div>
                `;
            })
            .catch(() => {
                content.innerHTML = `<div class="text-viewer">Failed to load file content</div>`;
            });
        zoomControls.style.display = 'none';
        loopControls.style.display = 'none';
    }

    viewer.classList.add('active');
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function encodeURIPath(path) {
    return path.split('/').map(segment => encodeURIComponent(segment)).join('/');
}


function nextMedia() {
    const mediaFiles = currentFiles.filter(f => !f.is_dir);
    if (mediaFiles.length === 0) return;

    const currentMediaFile = mediaFiles.find(f => f.path === selectedFile.path);
    const currentIndex = mediaFiles.indexOf(currentMediaFile);
    const nextIndex = currentIndex < mediaFiles.length - 1 ? currentIndex + 1 : 0;

    currentMediaIndex = currentFiles.findIndex(f => f.path === mediaFiles[nextIndex].path);
    clearLoop();
    showCurrentMedia();
}

function previousMedia() {
    const mediaFiles = currentFiles.filter(f => !f.is_dir);
    if (mediaFiles.length === 0) return;

    const currentMediaFile = mediaFiles.find(f => f.path === selectedFile.path);
    const currentIndex = mediaFiles.indexOf(currentMediaFile);
    const prevIndex = currentIndex > 0 ? currentIndex - 1 : mediaFiles.length - 1;

    currentMediaIndex = currentFiles.findIndex(f => f.path === mediaFiles[prevIndex].path);
    clearLoop();
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
    zoomLevel *= 1.05;
    updateMediaTransform();
}

function zoomOut() {
    zoomLevel *= 0.95;
    if (zoomLevel < 1) zoomLevel = 1;
    updateMediaTransform();
}

function resetZoom() {
    zoomLevel = 1;
    imagePos = { x: 0, y: 0 };
    isDragging = false;
    updateMediaTransform();
}

function moveUp() {
    if (zoomLevel > 1) {
        imagePos.y += 25; // Move up by 25px
        updateMediaTransform();
    }
}

function moveDown() {
    if (zoomLevel > 1) {
        imagePos.y -= 25; // Move down by 25px
        updateMediaTransform();
    }
}

function moveLeft() {
    if (zoomLevel > 1) {
        imagePos.x += 25; // Move left by 25px
        updateMediaTransform();
    }
}

function moveRight() {
    if (zoomLevel > 1) {
        imagePos.x -= 25; // Move right by 25px
        updateMediaTransform();
    }
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

    // Hide zoom controls and loop controls
    document.getElementById('zoomControls').style.display = 'none';
    document.getElementById('loopControls').style.display = 'none';

    document.getElementById('viewer').classList.remove('active');
    selectedFile = null;
    resetZoom();
    clearLoop();

    // Navigate back to directory
    navigateToDirectory(currentPath);
}

// Video loop functions
function setupVideoLoop() {
    const video = document.querySelector('#viewerContent video');
    if (!video) return;

    // Add timeupdate listener for loop monitoring
    video.addEventListener('timeupdate', () => {
        if (loopEnabled && loopEnd > loopStart && video.currentTime >= loopEnd) {
            video.currentTime = loopStart;
        }
    });

    // Update display if loop was already set
    updateLoopDisplay();
}

function setLoopStart() {
    const video = document.querySelector('#viewerContent video');
    if (!video) return;

    loopStart = video.currentTime;

    // If loop end is already set and is before start, clear it
    if (loopEnd > 0 && loopEnd <= loopStart) {
        loopEnd = 0;
    }

    // Enable loop if both points are set
    if (loopEnd > loopStart) {
        loopEnabled = true;
    }

    updateLoopDisplay();
}

function setLoopEnd() {
    const video = document.querySelector('#viewerContent video');
    if (!video) return;

    loopEnd = video.currentTime;

    // Validate that end is after start
    if (loopEnd <= loopStart) {
        alert('Loop end must be after loop start');
        loopEnd = 0;
        return;
    }

    loopEnabled = true;
    updateLoopDisplay();
}

function clearLoop() {
    loopEnabled = false;
    loopStart = 0;
    loopEnd = 0;
    updateLoopDisplay();
}

function formatTime(seconds) {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
}

function updateLoopDisplay() {
    const display = document.getElementById('loopDisplay');
    if (!display) return;

    if (loopEnabled && loopEnd > loopStart) {
        display.textContent = `Loop: ${formatTime(loopStart)} - ${formatTime(loopEnd)}`;
        display.style.display = 'block';
    } else if (loopStart > 0) {
        display.textContent = `Start: ${formatTime(loopStart)}`;
        display.style.display = 'block';
    } else {
        display.textContent = '';
        display.style.display = 'none';
    }
}

function saveTextFile() {
    const textarea = document.getElementById('textEditor');
    if (!textarea || !selectedFile) return;

    // Security confirmation before saving
    if (!confirm(`Save changes to ${selectedFile.name}?`)) {
        return;
    }

    const content = textarea.value;

    fetch(`/api/save?path=${encodeURIComponent(selectedFile.path)}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'text/plain',
        },
        body: content
    })
        .then(response => {
            if (response.ok) {
                alert('File saved successfully');
            } else {
                alert('Failed to save file');
            }
        })
        .catch(err => {
            alert('Error saving file: ' + err.message);
        });
}

function cancelTextEdit() {
    closeViewer();
}

function goPrev() {
    let pathname = window.location.pathname;

    // Remove /ui prefix
    if (pathname.startsWith('/ui')) {
        pathname = pathname.substring(3);
    }

    // Remove trailing slash if exists
    if (pathname.endsWith('/')) {
        pathname = pathname.slice(0, -1);
    }

    // If empty or root, we're already at root
    if (!pathname || pathname === '/') {
        return;
    }

    // Remove last segment
    const lastSlash = pathname.lastIndexOf('/');
    const parentPath = pathname.substring(0, lastSlash);

    // Navigate to parent
    const uiPath = parentPath ? '/ui' + parentPath + '/' : '/ui/';
    window.location.href = uiPath;
}

function triggerUpload() {
    document.getElementById('fileInput').click();
}

async function uploadFiles(files) {
    const CONCURRENT_UPLOADS = 10;
    const totalFiles = files.length;
    let completedCount = 0;
    let successCount = 0;
    let failCount = 0;
    const currentlyUploading = new Set();
    const failedFiles = [];

    // Show progress overlay
    const progressOverlay = document.getElementById('uploadProgress');
    const progressText = document.getElementById('uploadProgressText');
    const progressFiles = document.getElementById('uploadProgressFiles');
    progressOverlay.style.display = 'flex';

    function updateProgress() {
        progressText.textContent = `Uploading: ${completedCount}/${totalFiles}`;
        progressFiles.innerHTML = Array.from(currentlyUploading)
            .map(name => `<div>- ${escapeHtml(name)}</div>`)
            .join('');
    }

    async function uploadFile(file) {
        currentlyUploading.add(file.name);
        updateProgress();

        const formData = new FormData();
        formData.append('file', file);

        try {
            const response = await fetch(`/api/upload?path=${encodeURIComponent(currentPath)}`, {
                method: 'POST',
                body: formData
            });

            if (!response.ok) {
                const text = await response.text();
                failedFiles.push(`${file.name}: ${text}`);
                failCount++;
            } else {
                await response.json();
                successCount++;
            }
        } catch (err) {
            failedFiles.push(`${file.name}: ${err.message}`);
            failCount++;
        } finally {
            currentlyUploading.delete(file.name);
            completedCount++;
            updateProgress();
        }
    }

    // Upload files in batches of CONCURRENT_UPLOADS
    const fileArray = Array.from(files);
    for (let i = 0; i < fileArray.length; i += CONCURRENT_UPLOADS) {
        const batch = fileArray.slice(i, i + CONCURRENT_UPLOADS);
        await Promise.all(batch.map(file => uploadFile(file)));
    }

    // Hide progress overlay
    progressOverlay.style.display = 'none';

    // Show summary
    if (failCount > 0) {
        const summary = `Upload completed: ${successCount} succeeded, ${failCount} failed\n\nFailed files:\n${failedFiles.join('\n')}`;
        alert(summary);
    } else if (totalFiles > 0) {
        alert(`Successfully uploaded ${successCount} file(s)`);
    }

    // Refresh directory
    navigateToDirectory(currentPath);
}

function createFolder() {
    const name = prompt('folder name:');
    if (name) {
        const folderPath = `${currentPath}/${name}`;
        fetch(`/api/mkdir?path=${encodeURIComponent(folderPath)}`, { method: 'POST' })
            .then(() => navigateToDirectory(currentPath))
            .catch(() => alert('failed to create folder'));
    }
}

function refreshView() {
    navigateToDirectory(currentPath);
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
        if (selectedFile.is_dir) {
            window.open(`/api/download-multiple?paths=${encodeURIComponent(selectedFile.path)}`);
        } else {
            window.open(`/api/download?path=${encodeURIComponent(selectedFile.path)}`);
        }
    }
    hideContextMenu();
}

function deleteFile() {
    if (selectedFile && confirm(`delete ${selectedFile.name}?`)) {
        fetch(`/api/delete?path=${encodeURIComponent(selectedFile.path)}`, { method: 'DELETE' })
            .then(() => navigateToDirectory(currentPath))
            .catch(() => alert('delete failed'));
    }
    hideContextMenu();
}

function downloadCurrent() {
    if (selectedFile) {
        if (selectedFile.is_dir) {
            window.open(`/api/download-multiple?paths=${encodeURIComponent(selectedFile.path)}`);
        } else {
            window.open(`/api/download?path=${encodeURIComponent(selectedFile.path)}`);
        }
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

// No longer needed - browser handles back/forward with full page reloads

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

    // Recalculate virtual scroll grid if we have files loaded
    if (virtualScrollData.filteredFiles.length > 0) {
        const container = document.querySelector('.gallery-container');
        const currentScroll = container ? container.scrollTop : 0;

        const grid = document.getElementById('galleryGrid');
        initializeVirtualScroll(grid);

        // Restore scroll position after recalculation
        if (container) {
            requestAnimationFrame(() => {
                container.scrollTop = currentScroll;
            });
        }
    }
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

// Resize handler with debouncing and dimension checking
const resizeHandler = performanceUtils.debounce(() => {
    if (virtualScrollData.filteredFiles.length > 0) {
        const container = virtualScrollData.scrollContainer;
        if (container) {
            const currentWidth = container.clientWidth;
            const currentHeight = container.clientHeight;

            // Only recalculate if dimensions actually changed significantly
            if (Math.abs(currentWidth - virtualScrollData.lastContainerDimensions.width) > 10 ||
                Math.abs(currentHeight - virtualScrollData.lastContainerDimensions.height) > 10) {

                // Preserve scroll position
                const scrollPercentage = container.scrollTop / (container.scrollHeight - container.clientHeight || 1);

                // Recalculate grid dimensions
                const grid = document.getElementById('galleryGrid');
                initializeVirtualScroll(grid);

                // Restore scroll position
                requestAnimationFrame(() => {
                    const newScrollTop = scrollPercentage * (container.scrollHeight - container.clientHeight);
                    container.scrollTop = newScrollTop;
                });

                // Update cached dimensions
                virtualScrollData.lastContainerDimensions = { width: currentWidth, height: currentHeight };
            }
        }
    }
}, 250);

window.addEventListener('resize', resizeHandler);

// Load initial directory on page load
loadInitialDirectory();
