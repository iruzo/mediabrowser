let currentPath = '/data';
let currentFiles = [];
let selectedFile = null;
let currentMediaIndex = -1;
let currentFilter = 'all';
let currentSort = 'name';
let selectedFiles = new Set();
let selectionMode = false;
let searchTerm = '';
let showNames = localStorage.getItem('showNames') === 'true';
let intersectionObserver = null;
let gridSize = window.innerWidth <= 480 ? 20 : 30;
const galleryContainer = document.querySelector('.gallery-container');
const galleryGrid = document.getElementById('galleryGrid');

let zoomLevel = 1;
let isDragging = false;
let dragStart = { x: 0, y: 0 };
let imagePos = { x: 0, y: 0 };
let rotationAngle = 0;
let mediaZoomListenersInitialized = false;

let loopEnabled = false;
let loopStart = 0;
let loopEnd = 0;

let virtualScrollData = {
    filteredFiles: [],
    renderedItems: new Map(),
    scrollPosition: 0,
    itemHeight: 200,
    containerHeight: 0,
    scrollContainer: null,
    scrollHandler: null,
    spacer: null,
    visibleRange: { start: 0, end: 0 },
    lastContainerDimensions: { width: 0, height: 0 },
    selectedItemsCache: new Set()
};
