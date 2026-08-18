const BATCH_SIZE = 60;

function on(id, handler) {
  const element = document.getElementById(id);
  if (element) element.addEventListener("click", handler);
}

async function request(url, options) {
  const response = await fetch(url, options);
  if (!response.ok) throw new Error(await response.text());
  return response;
}

function post(action, data) {
  return request(action, {
    method: "POST",
    body: new URLSearchParams(data),
  });
}

function reveal(input, value) {
  if (!input.hidden) return false;
  if (value !== undefined) input.value = value;
  input.hidden = false;
  input.select();
  return true;
}

function encodedPath(path) {
  return path.split("/").map(encodeURIComponent).join("/");
}

function fileUrl(path) {
  return `/${encodedPath(path)}`;
}

function uiUrl(path, suffix = "") {
  const url = new URL(location.href);
  const encoded = encodedPath(path);
  url.pathname = `/ui/${encoded}${encoded ? suffix : ""}`;
  url.searchParams.delete("view");
  return `${url.pathname}${url.search}`;
}

function scopePath() {
  const path = location.pathname.slice(3);

  try {
    return path
      .split("/")
      .filter(Boolean)
      .map(decodeURIComponent)
      .join("/");
  } catch (_) {
    return null;
  }
}

function viewPath(path = scopePath()) {
  return path && !location.pathname.endsWith("/") ? path : null;
}

function cleanDirectory(path) {
  return path.replace(/^\/+|\/+$/g, "");
}

function parentPath(path) {
  const index = path.lastIndexOf("/");
  return index < 0 ? "" : path.slice(0, index);
}

function baseName(path) {
  return path.slice(path.lastIndexOf("/") + 1);
}

function displayDirectory(path) {
  return path ? `/${path}/` : "/";
}

function kindFor(state, path) {
  return state.metadata.get(path)?.kind || "text";
}

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
let activePath = scope || "";
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

document.title = scope === null ? "invalid path" : displayDirectory(scope);
if (cell) root.style.setProperty("--cell", `${cell}px`);

const imageObserver = new IntersectionObserver((entries) => {
  entries.forEach((entry) => {
    const image = entry.target;
    if (!image.isConnected) {
      imageObserver.unobserve(image);
      image.removeAttribute("src");
      return;
    }
    if (entry.isIntersecting) {
      if (!image.hasAttribute("src")) image.src = image.dataset.src;
    } else {
      image.removeAttribute("src");
    }
  });
});

const pageObserver = new IntersectionObserver(
  (entries) => {
    entries.forEach((entry) => {
      if (!entry.isIntersecting) return;
      const state = stateForSentinel.get(entry.target);
      if (!state || !state.details.isConnected) {
        pageObserver.unobserve(entry.target);
        return;
      }
      pageObserver.unobserve(entry.target);
      appendItems(state);
      pageObserver.observe(entry.target);
    });
  },
  { rootMargin: "200px 0px" },
);

async function findPaths(
  path,
  type,
  query = "",
  recursive = true,
  metadata = false,
) {
  const params = new URLSearchParams({ type });
  if (path) params.set("path", path);
  if (query) params.set("query", query);
  if (!recursive) params.set("recursive", "false");
  if (metadata) params.set("metadata", "true");
  const response = await request(`/api/find?${params}`);
  return response.json();
}

function compareNames(a, b) {
  const insensitive = a.localeCompare(b, undefined, { sensitivity: "base" });
  return insensitive || a.localeCompare(b);
}

function visiblePaths(state) {
  const paths = state.files.filter(
    (path) => view.filter === "all" || kindFor(state, path) === view.filter,
  );

  paths.sort((a, b) => {
    let value = 0;
    if (view.sort === "type") {
      const kinds = kindFor(state, a).localeCompare(kindFor(state, b));
      if (kinds) return kinds;
    } else if (view.sort === "date") {
      value = (state.metadata.get(b)?.date || 0) -
        (state.metadata.get(a)?.date || 0);
    } else if (view.sort === "size") {
      value = (state.metadata.get(b)?.size || 0) -
        (state.metadata.get(a)?.size || 0);
    }
    if (value) return value;
    return compareNames(baseName(a), baseName(b));
  });
  return paths;
}

function clearItems(state) {
  state.grid.querySelectorAll("img").forEach((image) => {
    imageObserver.unobserve(image);
    image.removeAttribute("src");
  });
  state.grid.replaceChildren();
  state.shown = 0;
}

function createItem(path, state) {
  const item = itemTemplate.content.firstElementChild.cloneNode(true);
  const kind = kindFor(state, path);
  const link = item.querySelector("a");
  const image = item.querySelector("img");

  item.classList.add(kind);
  item.classList.toggle("selected", selected.has(path));
  item.dataset.path = path;
  link.href = kind === "text" ? fileUrl(path) : uiUrl(path);
  item.querySelector(".name").textContent = baseName(path);

  if (kind === "image") {
    image.dataset.src = fileUrl(path);
    image.alt = baseName(path);
  } else {
    image.remove();
  }

  return item;
}

function appendItems(state) {
  if (!state.details.open || state.shown >= state.visible.length) return;

  const end = Math.min(state.shown + BATCH_SIZE, state.visible.length);
  const fragment = document.createDocumentFragment();
  const pendingImages = [];

  for (const path of state.visible.slice(state.shown, end)) {
    const item = createItem(path, state);
    const image = item.querySelector("img");
    if (image) pendingImages.push(image);
    fragment.append(item);
  }

  state.grid.append(fragment);
  pendingImages.forEach((image) => imageObserver.observe(image));
  state.shown = end;
  state.sentinel.hidden = state.shown >= state.visible.length;
}

function renderState(state) {
  clearItems(state);
  state.visible = visiblePaths(state);
  state.empty.hidden = state.visible.length !== 0;
  state.empty.textContent = "empty";
  state.sentinel.hidden = state.visible.length === 0;
  appendItems(state);
}

async function loadState(state) {
  if (state.files) {
    renderState(state);
    return;
  }
  if (state.loading) return state.loading;

  state.empty.hidden = false;
  state.empty.textContent = "loading";
  state.loading = findPaths(state.path, "file", "", false, true)
    .then((items) => {
      state.files = items.map((item) => item.path);
      state.metadata = new Map(items.map((item) => [item.path, item]));
      if (state.details.isConnected) renderState(state);
    })
    .catch((error) => {
      state.empty.hidden = false;
      state.empty.textContent = error.message || "Failed to load directory";
    })
    .finally(() => {
      state.loading = null;
    });

  return state.loading;
}

function addDirectory(path, preset, metadata = new Map()) {
  const details = directoryTemplate.content.firstElementChild.cloneNode(true);
  const state = {
    path,
    details,
    grid: details.querySelector(".grid"),
    empty: details.querySelector(".empty"),
    sentinel: details.querySelector(".sentinel"),
    files: preset,
    metadata,
    visible: [],
    shown: 0,
    loading: null,
  };

  details.querySelector(".directory-name").textContent = displayDirectory(path);
  if (path === scope) details.querySelector(".directory-menu-toggle").remove();
  stateFor.set(details, state);
  stateForSentinel.set(state.sentinel, state);
  states.add(state);
  directories.append(details);
  pageObserver.observe(state.sentinel);
}

function clearDirectories() {
  states.forEach((state) => {
    clearItems(state);
    pageObserver.unobserve(state.sentinel);
  });
  states.clear();
  directories.replaceChildren();
}

function showDirectoryError(message) {
  clearDirectories();
  const output = document.createElement("p");
  output.className = "empty";
  output.textContent = message;
  directories.append(output);
}

async function loadDirectories(query = "") {
  if (scope === null) {
    showDirectoryError("invalid path");
    return;
  }

  const load = ++directoryLoad;
  currentQuery = query;
  closeViewer(false);
  closeActionMenu();
  setSelecting(false);
  clearDirectories();
  activePath = scope;

  try {
    if (!query) {
      const paths = await findPaths(scope, "dir");
      if (load !== directoryLoad) return;
      const unique = new Set(
        paths.map(cleanDirectory).filter((path) => path !== scope),
      );
      addDirectory(scope, null);
      [...unique].sort(compareNames).forEach((path) => addDirectory(path, null));
    } else {
      const items = await findPaths(scope, "all", query, true, true);
      if (load !== directoryLoad) return;
      const foundDirectories = new Set();
      const filesByDirectory = new Map();
      const metadataByDirectory = new Map();

      items.forEach((item) => {
        const path = item.path;
        if (path.endsWith("/")) {
          foundDirectories.add(cleanDirectory(path));
          return;
        }

        const dir = parentPath(path);
        foundDirectories.add(dir);
        if (!filesByDirectory.has(dir)) filesByDirectory.set(dir, []);
        filesByDirectory.get(dir).push(path);
        if (!metadataByDirectory.has(dir)) metadataByDirectory.set(dir, new Map());
        metadataByDirectory.get(dir).set(path, item);
      });

      [...foundDirectories].sort(compareNames).forEach((path) => {
        addDirectory(
          path,
          filesByDirectory.get(path) || null,
          metadataByDirectory.get(path),
        );
      });

      if (!foundDirectories.size) showDirectoryError("no results");
    }

    if (initialView) {
      const path = initialView;
      initialView = null;
      await showViewPath(path, false);
    }
  } catch (error) {
    if (load !== directoryLoad) return;
    showDirectoryError(error.message || "Failed to load directories");
  }
}

directories.addEventListener(
  "toggle",
  (event) => {
    if (!event.target.matches("details.directory")) return;
    const state = stateFor.get(event.target);

    if (event.target.open) {
      if (selecting && activePath !== state.path && !currentQuery) {
        setSelecting(false);
      }
      activePath = state.path;
      states.forEach((item) => {
        item.grid.classList.toggle("selecting", selecting && item === state);
      });
      loadState(state);
    } else {
      clearItems(state);
    }
  },
  true,
);

function submitDownload(paths) {
  if (!paths.length) return;
  const form = document.createElement("form");
  form.method = "post";
  form.action = "/api/download";
  form.hidden = true;

  paths.forEach((path) => {
    const input = document.createElement("input");
    input.name = "path";
    input.value = path;
    form.append(input);
  });

  document.body.append(form);
  form.requestSubmit();
  setTimeout(() => form.remove(), 0);
}

function currentState() {
  return [...states].find((state) => state.path === activePath) || null;
}

function closeSelectionPaths() {
  selectionMenu.querySelectorAll("input").forEach((input) => {
    input.hidden = true;
  });
}

function setSelecting(value) {
  if (value) closeActionMenu();
  selecting = value;
  const button = document.getElementById("select");
  const state = currentState();

  states.forEach((item) => {
    item.grid.classList.toggle("selecting", value && item === state);
  });
  directories.classList.toggle("selecting", value);
  selectionMenu.hidden = !value;
  button.classList.toggle("on", value);
  button.textContent = value ? "done" : "select";

  if (!value) {
    closeSelectionPaths();
    selected.clear();
    directories
      .querySelectorAll(".item.selected, .directory.selected")
      .forEach((item) => {
        item.classList.remove("selected");
      });
  }
}

function selectedPaths() {
  const state = currentState();
  const files = new Set(
    currentQuery
      ? [...states].flatMap((item) => item.files || [])
      : state?.files || [],
  );
  const dirs = new Set(
    [...states]
      .map((item) => item.path)
      .filter((path) => path !== scope),
  );
  const paths = [...selected].filter(
    (path) => files.has(path) || dirs.has(path),
  );

  return paths.filter(
    (path) =>
      !paths.some(
        (parent) =>
          parent !== path && dirs.has(parent) && path.startsWith(`${parent}/`),
      ),
  );
}

async function runSelected(paths, operation, message) {
  const failed = [];

  for (const path of paths) {
    try {
      await operation(path);
    } catch (error) {
      failed.push(`${path}: ${error.message || message}`);
    }
  }

  if (failed.length) alert(failed.join("\n"));
}

function destinationPath(dir, path) {
  const parent = dir.replace(/^\/+|\/+$/g, "");
  return parent ? `${parent}/${baseName(path)}` : baseName(path);
}

function bindSelectedPath(action) {
  const button = document.getElementById(`selected-${action}`);
  const input = document.getElementById(`selected-${action}-to`);

  button.addEventListener("click", async () => {
    const paths = selectedPaths();
    if (!paths.length) return;
    if (reveal(input, activePath || "/")) return;

    const to = input.value.trim();
    if (!to) return;
    button.disabled = true;
    await runSelected(
      paths,
      (path) =>
        post(`/api/${action}`, {
          from: path,
          to: destinationPath(to, path),
        }),
      `Failed to ${action} item`,
    );
    location.reload();
  });
}

function removePath(path) {
  selected.delete(path);

  states.forEach((state) => {
    if (!state.files || !state.files.includes(path)) return;
    const index = state.visible.indexOf(path);
    if (index >= 0 && index < state.shown) state.shown -= 1;
    state.files = state.files.filter((item) => item !== path);
    state.visible = state.visible.filter((item) => item !== path);
    state.metadata.delete(path);

    const item = [...state.grid.querySelectorAll(".item")].find(
      (candidate) => candidate.dataset.path === path,
    );
    const image = item?.querySelector("img");
    if (image) {
      imageObserver.unobserve(image);
      image.removeAttribute("src");
    }
    if (item) item.remove();

    state.empty.hidden = state.visible.length !== 0;
    state.sentinel.hidden = state.shown >= state.visible.length;
    appendItems(state);
  });
}

function selectPath(path, element) {
  const value = !selected.has(path);
  if (value) selected.add(path);
  else selected.delete(path);
  element.classList.toggle("selected", value);
}

function resetActionMenu() {
  actionTarget = null;
  actionMenu.querySelectorAll("input.to").forEach((input) => {
    input.hidden = true;
  });
}

function closeActionMenu() {
  if (actionMenu.matches(":popover-open")) actionMenu.hidePopover();
  resetActionMenu();
}

function isActionTarget(target) {
  return actionTarget?.toggle === target.toggle;
}

function toggleActionMenu(toggle, path, directory) {
  if (
    actionMenu.matches(":popover-open") &&
    actionTarget?.toggle === toggle
  ) {
    closeActionMenu();
    return;
  }

  closeActionMenu();
  actionTarget = { toggle, path, directory };
  actionMenu.querySelectorAll("input.to").forEach((input) => {
    input.value = path;
    input.hidden = true;
  });
  if (galleryMenu.matches(":popover-open")) galleryMenu.hidePopover();
  actionMenu.showPopover();
}

galleryBar.querySelector(".menu-toggle").addEventListener("click", () => {
  closeActionMenu();
});

directories.addEventListener("click", (event) => {
  const menuToggle = event.target.closest(".action-menu-toggle");
  if (menuToggle) {
    event.preventDefault();
    const item = menuToggle.closest(".item");
    if (item) {
      toggleActionMenu(menuToggle, item.dataset.path, false);
    } else {
      const state = stateFor.get(menuToggle.closest("details.directory"));
      toggleActionMenu(menuToggle, state.path, true);
    }
    return;
  }

  closeActionMenu();

  const summary = event.target.closest("summary");
  if (summary?.parentElement?.matches("details.directory") && selecting) {
    const state = stateFor.get(summary.parentElement);
    if (
      state.path !== scope &&
      (currentQuery || parentPath(state.path) === activePath)
    ) {
      event.preventDefault();
      selectPath(state.path, state.details);
      return;
    }
  }

  const item = event.target.closest(".item");
  if (!item || !directories.contains(item)) return;

  const link = event.target.closest("a");
  if (link) {
    if (selecting && item.closest(".grid").classList.contains("selecting")) {
      event.preventDefault();
      selectPath(item.dataset.path, item);
    } else if (!item.classList.contains("text")) {
      event.preventDefault();
      const state = stateFor.get(item.closest("details.directory"));
      openViewer(item.dataset.path, state);
    }
    return;
  }
});

actionMenu.addEventListener("click", async (event) => {
  const button = event.target.closest("button");
  if (!button || actionTarget === null) return;

  const target = actionTarget;

  if (button.classList.contains("download")) {
    submitDownload([target.path]);
    return;
  }

  const action = button.classList.contains("rm")
    ? "rm"
    : button.classList.contains("cp")
      ? "cp"
      : button.classList.contains("mv")
        ? "mv"
        : "";
  if (!action) return;

  let data = { path: target.path };
  if (action !== "rm") {
    const input = actionMenu.querySelector(`.${action}-to`);
    if (reveal(input)) return;
    const to = input.value.trim();
    if (!to) return;
    data = { from: target.path, to };
  }

  button.disabled = true;
  try {
    await post(`/api/${action}`, data);
    if (action === "rm") {
      if (isActionTarget(target)) closeActionMenu();
      if (target.directory) await loadDirectories(currentQuery);
      else removePath(target.path);
    } else {
      location.reload();
    }
  } catch (error) {
    const operation = action === "rm" ? "remove" : action;
    const type = target.directory ? "directory" : "item";
    alert(error.message || `Failed to ${operation} ${type}`);
  } finally {
    button.disabled = false;
  }
});

actionMenu.addEventListener("toggle", () => {
  if (actionMenu.matches(":popover-open")) return;
  resetActionMenu();
});

document.addEventListener("keydown", (event) => {
  const input = event.target.closest("input.to, #selection-menu input");
  if (!input || event.key !== "Enter") return;
  event.preventDefault();
  input.nextElementSibling.click();
});

on("select", () => setSelecting(!selecting));
on("selected-download", () => submitDownload(selectedPaths()));
bindSelectedPath("cp");
bindSelectedPath("mv");

on("selected-rm", async () => {
  const paths = selectedPaths();
  let reload = false;
  await runSelected(
    paths,
    async (path) => {
      const isDirectory = [...states].some((state) => state.path === path);
      await post("/api/rm", { path });
      if (isDirectory) reload = true;
      else removePath(path);
    },
    "Failed to remove item",
  );
  if (reload) await loadDirectories(currentQuery);
});

function resizeCell(delta) {
  const size =
    cell || parseInt(getComputedStyle(root).getPropertyValue("--cell"), 10);
  cell = Math.min(480, Math.max(80, size + delta));
  root.style.setProperty("--cell", `${cell}px`);
  localStorage.setItem("cell", cell);
}

on("cellminus", () => resizeCell(-40));
on("cellplus", () => resizeCell(40));

galleryMenu.addEventListener("click", (event) => {
  const link = event.target.closest("a");
  if (!link) return;

  const params = new URL(link.href).searchParams;
  const name = params.has("sort")
    ? "sort"
    : params.has("filter")
      ? "filter"
      : "";
  if (!name) return;

  event.preventDefault();
  view[name] = params.get(name);
  link.parentElement.querySelectorAll("a").forEach((other) => {
    other.classList.toggle("on", other === link);
  });
  states.forEach((state) => {
    if (state.details.open && state.files) renderState(state);
  });
});

search.addEventListener("submit", (event) => {
  event.preventDefault();
  loadDirectories(search.elements.query.value.trim());
});

on("mkdir", async () => {
  const input = document.getElementById("name");
  const name = input.value.trim();
  if (!name) return;
  const path = activePath ? `${activePath}/${name}` : name;

  try {
    await post("/api/mkdir", { path });
    input.value = "";
    await loadDirectories(search.elements.query.value.trim());
  } catch (error) {
    alert(error.message || "Failed to create folder");
  }
});

const showUpload = document.querySelector(".show-upload");

showUpload.addEventListener("click", () => {
  files.click();
});

files.addEventListener("change", () => {
  if (files.files.length) upload.requestSubmit();
});

upload.addEventListener("submit", async (event) => {
  event.preventDefault();
  showUpload.disabled = true;
  upload.elements.path.value = activePath;
  progress.hidden = false;
  progress.textContent = `uploading ${files.files.length}`;

  try {
    await request(upload.action, {
      method: upload.method,
      body: new FormData(upload),
    });
    files.value = "";
    progress.hidden = true;
    if (currentQuery) {
      await loadDirectories(currentQuery);
      return;
    }
    const state = [...states].find((item) => item.path === activePath);
    if (state) {
      state.files = null;
      await loadState(state);
    }
  } catch (error) {
    alert(error.message || "Failed to upload files");
    progress.hidden = true;
    files.value = "";
  } finally {
    showUpload.disabled = false;
  }
});

function viewerFiles(state) {
  return visiblePaths(state).filter((path) => kindFor(state, path) !== "text");
}

function stopDragging(event) {
  if (!drag || (event && drag.id !== event.pointerId)) return;
  if (drag.media.hasPointerCapture(drag.id)) {
    drag.media.releasePointerCapture(drag.id);
  }
  drag = null;
}

function resetLoop() {
  loopStart = 0;
  loopEnd = 0;
  loopStartInput.value = "";
  loopEndInput.value = "";
}

function resetViewer() {
  if (viewerMenu.matches(":popover-open")) viewerMenu.hidePopover();
  stopDragging();
  resetLoop();
  zoom = 1;
  angle = 0;
  position.x = 0;
  position.y = 0;

  const media = viewer.media;
  if (viewer.type === "image") {
    media.removeAttribute("alt");
  } else {
    if (document.pictureInPictureElement === media) {
      document.exitPictureInPicture();
    }
    if (document.fullscreenElement === media) document.exitFullscreen();
    media.pause();
    media.volume = 1;
    media.muted = false;
    media.loop = false;
    media.defaultPlaybackRate = 1;
    media.playbackRate = 1;
  }
  media.removeAttribute("src");
  if (viewer.type !== "image") media.load();
  media.hidden = true;
  media.classList.remove("zoomed");
  media.style.removeProperty("transform");
  viewer = null;
}

function closeViewer(push = true) {
  if (!viewer) return;
  resetViewer();
  viewerRoot.hidden = true;
  directories.hidden = false;
  galleryBar.hidden = false;
  galleryMenu.hidden = false;
  selectionMenu.hidden = !selecting;
  document.title = displayDirectory(scope);

  if (push) history.pushState({}, "", uiUrl(scope, "/"));
}

function openViewer(path, state, push = true) {
  if (!state || !state.files) return;
  const type = kindFor(state, path);
  const media = viewerMedia[type];
  if (!media) {
    location.replace(fileUrl(path));
    return;
  }
  if (viewer) resetViewer();

  const paths = viewerFiles(state);
  const index = Math.max(0, paths.indexOf(path));
  const previous = paths.length
    ? paths[(index + paths.length - 1) % paths.length]
    : path;
  const next = paths.length ? paths[(index + 1) % paths.length] : path;

  viewer = { path, type, state, media, previous, next };
  media.hidden = false;
  media.src = fileUrl(path);
  if (type === "image") media.alt = baseName(path);
  viewerPrevious.href = uiUrl(previous);
  viewerClose.href = uiUrl(scope, "/");
  viewerNext.href = uiUrl(next);
  viewerControls.forEach((control) => {
    control.hidden = !control.dataset.types.split(" ").includes(type);
  });

  closeActionMenu();
  if (galleryMenu.matches(":popover-open")) galleryMenu.hidePopover();
  galleryMenu.hidden = true;
  directories.hidden = true;
  galleryBar.hidden = true;
  selectionMenu.hidden = true;
  viewerRoot.hidden = false;
  document.title = baseName(path);

  if (push) history.pushState({}, "", uiUrl(path));
}

function transformViewer() {
  if (!viewer || viewer.type === "audio") return;
  if (zoom <= 1) {
    position.x = 0;
    position.y = 0;
  }
  viewer.media.classList.toggle("zoomed", zoom > 1);
  viewer.media.style.transform = `rotate(${angle}deg) scale(${zoom}) translate(${position.x / zoom}px, ${position.y / zoom}px)`;
}

function parseTime(text) {
  const parts = text.trim().split(":");
  if (parts.length !== 2) return null;
  const minutes = parseInt(parts[0], 10);
  const seconds = parseInt(parts[1], 10);
  return Number.isInteger(minutes) &&
      Number.isInteger(seconds) &&
      minutes >= 0 &&
      seconds >= 0 &&
      seconds < 60
    ? minutes * 60 + seconds
    : null;
}

function isVisualEvent(event) {
  return viewer && viewer.type !== "audio" && event.target === viewer.media;
}

function bindViewer() {
  viewerBar.addEventListener("click", (event) => {
    const link = event.target.closest("a");
    if (!link || !viewer) return;
    event.preventDefault();
    const current = viewer;
    if (link === viewerPrevious) {
      openViewer(current.previous, current.state);
    } else if (link === viewerNext) {
      openViewer(current.next, current.state);
    } else if (link === viewerClose) {
      closeViewer();
    }
  });

  viewerMenu.addEventListener("click", (event) => {
    const button = event.target.closest("button");
    if (!button || !viewer) return;

    if (button.id === "viewer-download") {
      submitDownload([viewer.path]);
    } else if (button.dataset.seek) {
      viewer.media.currentTime = Math.max(
        0,
        viewer.media.currentTime + Number(button.dataset.seek),
      );
    } else if (button.dataset.zoom) {
      if (button.dataset.zoom === "in") zoom *= 1.25;
      else if (button.dataset.zoom === "out") zoom = Math.max(1, zoom / 1.25);
      else {
        zoom = 1;
        angle = 0;
      }
      transformViewer();
    } else if (button.dataset.move && zoom > 1) {
      const [dx, dy] = button.dataset.move.split(" ").map(Number);
      position.x += dx;
      position.y += dy;
      transformViewer();
    } else if (button.dataset.rot) {
      angle += Number(button.dataset.rot);
      transformViewer();
    } else if (button.id === "loopclear") {
      resetLoop();
    }
  });

  viewerMain.addEventListener("wheel", (event) => {
    if (!isVisualEvent(event)) return;
    event.preventDefault();
    zoom = Math.max(1, zoom * (event.deltaY > 0 ? 0.9 : 1.1));
    transformViewer();
  });
  viewerMain.addEventListener("pointerdown", (event) => {
    if (!isVisualEvent(event) || zoom <= 1) return;
    event.preventDefault();
    viewer.media.setPointerCapture(event.pointerId);
    drag = {
      id: event.pointerId,
      media: viewer.media,
      x: event.clientX - position.x,
      y: event.clientY - position.y,
    };
  });
  viewerMain.addEventListener("pointermove", (event) => {
    if (!drag || drag.id !== event.pointerId) return;
    position.x = event.clientX - drag.x;
    position.y = event.clientY - drag.y;
    transformViewer();
  });
  viewerMain.addEventListener("pointerup", stopDragging);
  viewerMain.addEventListener("pointercancel", stopDragging);
  viewerMain.addEventListener("dblclick", (event) => {
    if (!isVisualEvent(event)) return;
    zoom = zoom > 1 ? 1 : 2;
    transformViewer();
  });

  viewerMedia.video.addEventListener("timeupdate", () => {
    if (
      loopEnd > loopStart &&
      viewerMedia.video.currentTime >= loopEnd
    ) {
      viewerMedia.video.currentTime = loopStart;
    }
  });
  loopStartInput.addEventListener("change", () => {
    const time = parseTime(loopStartInput.value);
    if (time === null) loopStartInput.value = "";
    else loopStart = time;
  });
  loopEndInput.addEventListener("change", () => {
    const time = parseTime(loopEndInput.value);
    if (time === null) loopEndInput.value = "";
    else loopEnd = time;
  });
}

bindViewer();

async function showViewPath(path, push) {
  const dir = parentPath(path);
  const state = [...states].find((item) => item.path === dir);
  if (!state) return;
  state.details.open = true;
  activePath = state.path;
  await loadState(state);
  openViewer(path, state, push);
}

document.addEventListener("keydown", (event) => {
  if (
    !viewer ||
    event.target.matches("input, textarea") ||
    document.querySelector(":popover-open")
  ) {
    return;
  }

  const link = {
    Escape: viewerClose,
    ArrowLeft: viewerPrevious,
    ArrowRight: viewerNext,
  }[event.key];
  if (link) link.click();
});

addEventListener("popstate", async () => {
  const path = viewPath();
  if (path) await showViewPath(path, false);
  else closeViewer(false);
});

if (scope === null) {
  galleryBar.hidden = true;
  showDirectoryError("invalid path");
} else {
  loadDirectories();
}
