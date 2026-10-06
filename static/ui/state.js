const root = document.documentElement;
const directories = document.getElementById("directories");
const directoryTemplate = document.getElementById("directory-template");
const itemTemplate = document.getElementById("item-template");
const viewerRoot = document.getElementById("viewer-root");
const viewerMain = viewerRoot.querySelector(".viewer");
const viewerBar = viewerRoot.querySelector(".viewer-bar");
const viewerMenu = document.getElementById("viewer-menu");
const viewerPrevious = document.getElementById("prev");
const viewerClose = document.getElementById("close");
const viewerNext = document.getElementById("next");
const viewerMedia = {
  image: viewerMain.querySelector('[data-type="image"]'),
  video: viewerMain.querySelector('[data-type="video"]'),
  audio: viewerMain.querySelector('[data-type="audio"]'),
};
const viewerControls = viewerMenu.querySelectorAll("[data-types]");
const loopStartInput = document.getElementById("loopstart");
const loopEndInput = document.getElementById("loopend");
const galleryBar = document.querySelector(".gallery-bar");
const galleryMenu = document.getElementById("menu");
const actionMenu = document.getElementById("action-menu");
const uploadButton = actionMenu.querySelector(".upload-files");
const selectionMenu = document.getElementById("selection-menu");
const search = document.querySelector("form.search");
const upload = document.querySelector("form.upload");
const files = upload.elements.file;
const progress = document.getElementById("progress");
const routePath = scopePath();
const routeView = viewPath(routePath);
const scope = routeView ? parentPath(routeView) : routePath;
const states = new Set();
const stateFor = new WeakMap();
const stateForSentinel = new WeakMap();
const view = { sort: "name", filter: "all" };
const selected = new Set();
let cell = parseInt(localStorage.getItem("cell"), 10) || 0;
let selecting = false;
let viewer = null;
let zoom = 1;
let angle = 0;
let position = { x: 0, y: 0 };
let drag = null;
let loopStart = 0;
let loopEnd = 0;
let initialView = routeView;
let directoryLoad = 0;
let currentQuery = "";
let actionTarget = null;

const imageResources = new WeakMap();

document.title = scope === null ? "invalid path" : displayDirectory(scope);
if (cell) root.style.setProperty("--cell", `${cell}px`);
