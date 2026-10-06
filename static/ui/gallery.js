function releaseImage(image) {
  const resource = imageResources.get(image);
  if (resource) {
    resource.controller.abort();
    if (resource.url) URL.revokeObjectURL(resource.url);
    imageResources.delete(image);
  }
  image.removeAttribute("src");
}

async function loadImage(image) {
  if (imageResources.has(image)) return;

  const resource = {
    controller: new AbortController(),
    url: null,
  };
  imageResources.set(image, resource);

  try {
    const url = await catUrl(image.dataset.path, resource.controller.signal);
    if (!image.isConnected || imageResources.get(image) !== resource) {
      URL.revokeObjectURL(url);
      return;
    }
    resource.url = url;
    image.src = url;
  } catch (error) {
    if (error.name !== "AbortError") image.removeAttribute("src");
  }
}

const imageObserver = new IntersectionObserver((entries) => {
  entries.forEach((entry) => {
    const image = entry.target;
    if (!image.isConnected) {
      imageObserver.unobserve(image);
      releaseImage(image);
      return;
    }
    if (entry.isIntersecting) {
      loadImage(image);
    } else {
      releaseImage(image);
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
  const response = await post("/api/find", params);
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
      value =
        (state.metadata.get(b)?.date || 0) - (state.metadata.get(a)?.date || 0);
    } else if (view.sort === "size") {
      value =
        (state.metadata.get(b)?.size || 0) - (state.metadata.get(a)?.size || 0);
    }
    if (value) return value;
    return compareNames(baseName(a), baseName(b));
  });
  return paths;
}

function clearItems(state) {
  state.grid.querySelectorAll("img").forEach((image) => {
    imageObserver.unobserve(image);
    releaseImage(image);
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
  link.href = uiUrl(path);
  item.querySelector(".name").textContent = baseName(path);

  if (kind === "image") {
    image.dataset.path = path;
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

  try {
    if (!query) {
      const paths = await findPaths(scope, "dir");
      if (load !== directoryLoad) return;
      const unique = new Set(
        paths.map(cleanDirectory).filter((path) => path !== scope),
      );
      addDirectory(scope, null);
      [...unique]
        .sort(compareNames)
        .forEach((path) => addDirectory(path, null));
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
        if (!metadataByDirectory.has(dir))
          metadataByDirectory.set(dir, new Map());
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
      state.grid.classList.toggle("selecting", selecting);
      loadState(state);
    } else {
      clearItems(state);
    }
  },
  true,
);
