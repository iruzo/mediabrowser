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

let zoomLevel = 1;
let isDragging = false;
let dragStart = { x: 0, y: 0 };
let imagePos = { x: 0, y: 0 };
let rotationAngle = 0;

let loopEnabled = false;
let loopStart = 0;
let loopEnd = 0;

const videoLoadQueue = {
    queue: [],
    loading: 0,
    maxConcurrent: 6,

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
        video.preload = 'metadata';

        const onLoadComplete = () => {
            this.loading--;
            this.process();
        };

        video.addEventListener('loadeddata', onLoadComplete, { once: true });
        video.addEventListener('error', onLoadComplete, { once: true });

        item.innerHTML = '';
        item.appendChild(video);

        if (selectedFiles.has(filePath)) {
            const overlay = document.createElement('div');
            overlay.className = 'selection-overlay';
            item.appendChild(overlay);
        }
    },

    clear() {
        this.queue = [];
    }
};

let virtualScrollData = {
    filteredFiles: [],
    renderedItems: new Map(),
    scrollPosition: 0,
    itemHeight: 200,
    containerHeight: 0,
    scrollContainer: null,
    visibleRange: { start: 0, end: 0 },
    lastContainerDimensions: { width: 0, height: 0 },
    selectedItemsCache: new Set()
};
