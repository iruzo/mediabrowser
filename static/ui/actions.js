function submitPaths(action, paths) {
  if (!paths.length) return;
  const form = document.createElement("form");
  form.method = "post";
  form.action = action;
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

function submitCat(path) {
  submitPaths("/api/cat", [path]);
}

function submitDownload(paths) {
  submitPaths("/api/download", paths);
}

function closeSelectionPaths() {
  selectionMenu.querySelectorAll("input").forEach((input) => {
    input.value = "";
    input.hidden = true;
  });
}

function setSelecting(value) {
  if (value) closeActionMenu();
  selecting = value;
  const button = document.getElementById("select");

  states.forEach((item) => {
    item.grid.classList.toggle("selecting", value);
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
  const files = new Set([...states].flatMap((item) => item.files || []));
  const dirs = new Set(
    [...states].map((item) => item.path).filter((path) => path !== scope),
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
    if (reveal(input)) return;

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
      releaseImage(image);
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
  if (actionMenu.matches(":popover-open") && actionTarget?.toggle === toggle) {
    closeActionMenu();
    return;
  }

  closeActionMenu();
  actionTarget = { toggle, path, directory };
  actionMenu.querySelectorAll("input.to").forEach((input) => {
    input.value = "";
    input.hidden = true;
  });
  uploadButton.hidden = !directory;
  actionMenu.querySelectorAll(".download, .cp, .mv, .rm").forEach((button) => {
    button.hidden = directory && path === scope;
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
    if (state.path !== scope) {
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
    } else {
      event.preventDefault();
      if (item.classList.contains("text")) {
        submitCat(item.dataset.path);
      } else {
        const state = stateFor.get(item.closest("details.directory"));
        openViewer(item.dataset.path, state);
      }
    }
    return;
  }
});

actionMenu.addEventListener("click", async (event) => {
  const button = event.target.closest("button");
  if (!button || actionTarget === null) return;

  const target = actionTarget;

  if (button === uploadButton) {
    upload.elements.path.value = target.path;
    closeActionMenu();
    files.click();
    return;
  }

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
  const path = scope ? `${scope}/${name}` : name;
  const submit = document.getElementById("mkdir");
  submit.disabled = true;

  try {
    await post("/api/mkdir", { path });
    input.value = "";
    await loadDirectories(search.elements.query.value.trim());
  } catch (error) {
    alert(error.message || "Failed to create folder");
  } finally {
    submit.disabled = false;
  }
});

document.getElementById("name").addEventListener("keydown", (event) => {
  if (event.key !== "Enter") return;
  event.preventDefault();
  document.getElementById("mkdir").click();
});

files.addEventListener("change", () => {
  if (files.files.length) upload.requestSubmit();
});

upload.addEventListener("submit", async (event) => {
  event.preventDefault();
  uploadButton.disabled = true;
  const path = upload.elements.path.value;
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
    const state = [...states].find((item) => item.path === path);
    if (state) {
      state.files = null;
      await loadState(state);
    }
  } catch (error) {
    alert(error.message || "Failed to upload files");
    progress.hidden = true;
    files.value = "";
  } finally {
    uploadButton.disabled = false;
  }
});
