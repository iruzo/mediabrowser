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

const root = document.documentElement;
const grid = document.querySelector(".grid");
const galleryBar = document.querySelector(".bar");
const galleryMenu = document.getElementById("menu");
const selectionMenu = document.getElementById("selection-menu");
const viewerTemplate = document.getElementById("viewer-template");
const upload = document.querySelector("form.upload");
const title = document.title;
let viewer = null;
let cell = parseInt(localStorage.getItem("cell"), 10) || 0;
let selecting = false;
const view = { sort: "name", filter: "all" };

if (cell) root.style.setProperty("--cell", `${cell}px`);

function currentPath() {
  return upload.elements.path.value;
}

function encodedPath(path) {
  return path.split("/").map(encodeURIComponent).join("/");
}

function viewerUrl(path) {
  const url = new URL(location.href);
  url.searchParams.set("view", path);
  return `${url.pathname}${url.search}`;
}

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

grid.addEventListener("click", async (event) => {
  const item = event.target.closest(".item");
  if (!item || !grid.contains(item)) return;

  const link = event.target.closest("a");
  if (link) {
    if (selecting) {
      event.preventDefault();
      item.classList.toggle("selected");
    } else if (item.dataset.kind !== "dir" && item.dataset.kind !== "text") {
      event.preventDefault();
      openViewer(item.dataset.path);
    }
    return;
  }

  const button = event.target.closest("button");
  if (!button) return;

  if (button.classList.contains("download")) {
    submitDownload([item.dataset.path]);
    return;
  }

  if (button.classList.contains("rm")) {
    button.disabled = true;
    try {
      await post("/api/rm", { path: item.dataset.path });
      item.remove();
    } catch (error) {
      alert(error.message || "Failed to remove item");
      button.disabled = false;
    }
    return;
  }

  const action = button.classList.contains("cp")
    ? "cp"
    : button.classList.contains("mv")
      ? "mv"
      : "";
  if (!action) return;
  const input = item.querySelector(`.${action}-to`);
  if (reveal(input)) return;

  const to = input.value.trim();
  if (!to) return;
  button.disabled = true;
  try {
    await post(`/api/${action}`, { from: item.dataset.path, to });
    location.reload();
  } catch (error) {
    alert(error.message || `Failed to ${action} item`);
    button.disabled = false;
  }
});

document.addEventListener("keydown", (event) => {
  const input = event.target.closest("input.to, #selection-menu input");
  if (!input || event.key !== "Enter") return;
  event.preventDefault();
  input.nextElementSibling.click();
});

grid.addEventListener(
  "toggle",
  (event) => {
    if (!event.target.matches("details.menu") || event.target.open) return;
    event.target.querySelectorAll("input.to").forEach((input) => {
      input.hidden = true;
    });
  },
  true,
);

function resizeCell(delta) {
  const size =
    cell || parseInt(getComputedStyle(root).getPropertyValue("--cell"), 10);
  cell = Math.min(480, Math.max(80, size + delta));
  root.style.setProperty("--cell", `${cell}px`);
  localStorage.setItem("cell", cell);
}

on("cellminus", () => resizeCell(-40));
on("cellplus", () => resizeCell(40));

function selectedItems() {
  const items = [...grid.querySelectorAll(".item.selected")];
  return items.filter(
    (item) =>
      !items.some(
        (parent) =>
          parent !== item &&
          parent.classList.contains("dir") &&
          item.dataset.path.startsWith(`${parent.dataset.path}/`),
      ),
  );
}

function closeSelectionPaths() {
  selectionMenu.querySelectorAll("input").forEach((input) => {
    input.hidden = true;
  });
}

on("select", () => {
  const button = document.getElementById("select");
  selecting = !selecting;
  grid.classList.toggle("selecting", selecting);
  selectionMenu.hidden = !selecting;
  button.classList.toggle("on", selecting);
  button.textContent = selecting ? "done" : "select";
  if (!selecting) {
    closeSelectionPaths();
    grid
      .querySelectorAll(".item.selected")
      .forEach((item) => item.classList.remove("selected"));
  }
});

on("selected-download", () => {
  submitDownload(selectedItems().map((item) => item.dataset.path));
});

function destinationPath(dir, path) {
  const name = path.slice(path.lastIndexOf("/") + 1);
  return `${dir.replace(/\/+$/, "")}/${name}`;
}

function bindSelectedPath(action) {
  const button = document.getElementById(`selected-${action}`);
  const input = document.getElementById(`selected-${action}-to`);

  async function run() {
    const items = selectedItems();
    if (!items.length) return;

    const dir = currentPath();
    if (reveal(input, dir || "/")) return;

    const to = input.value.trim();
    if (!to) return;
    button.disabled = true;
    const failed = [];

    for (const item of items) {
      try {
        await post(`/api/${action}`, {
          from: item.dataset.path,
          to: destinationPath(to, item.dataset.path),
        });
      } catch (error) {
        failed.push(
          `${item.dataset.path}: ${error.message || `Failed to ${action} item`}`,
        );
      }
    }

    if (failed.length) alert(failed.join("\n"));
    location.reload();
  }

  button.addEventListener("click", run);
}

bindSelectedPath("cp");
bindSelectedPath("mv");

on("selected-rm", async () => {
  const items = selectedItems();
  const failed = [];

  for (const item of items) {
    try {
      await post("/api/rm", { path: item.dataset.path });
      item.remove();
    } catch (error) {
      failed.push(
        `${item.dataset.path}: ${error.message || "Failed to remove item"}`,
      );
    }
  }

  if (failed.length) alert(failed.join("\n"));
});

const nameInput = document.getElementById("name");

on("mkdir", async () => {
  const name = nameInput.value.trim();
  if (!name) return;

  const dir = currentPath();
  const path = dir ? `${dir}/${name}` : name;

  try {
    await post("/api/mkdir", { path });
    location.reload();
  } catch (error) {
    alert(error.message || "Failed to create folder");
  }
});

function applyView() {
  const items = [...grid.querySelectorAll(".item")];

  items.forEach((item) => {
    item.hidden =
      view.filter !== "all" &&
      !item.classList.contains("dir") &&
      !item.classList.contains(view.filter);
  });

  items.sort((a, b) => {
    const dirs =
      Number(b.dataset.kind === "dir") - Number(a.dataset.kind === "dir");
    if (dirs) return dirs;

    const names = () => {
      const insensitive = a.dataset.path.localeCompare(
        b.dataset.path,
        undefined,
        {
          sensitivity: "base",
        },
      );
      return insensitive || a.dataset.path.localeCompare(b.dataset.path);
    };
    if (a.dataset.kind === "dir") return names();

    let value = 0;
    if (view.sort === "type")
      value = a.dataset.kind.localeCompare(b.dataset.kind);
    else if (view.sort === "date")
      value = Number(b.dataset.date) - Number(a.dataset.date);
    else if (view.sort === "size")
      value = Number(b.dataset.size) - Number(a.dataset.size);
    return value || names();
  });
  grid.append(...items);
}

document.querySelectorAll("a[href^='?sort=']").forEach((link) => {
  link.addEventListener("click", (event) => {
    event.preventDefault();
    view.sort = new URL(link.href).searchParams.get("sort");
    link.parentElement
      .querySelectorAll("a")
      .forEach((item) => item.classList.toggle("on", item === link));
    applyView();
  });
});

document.querySelectorAll("a[href^='?filter=']").forEach((link) => {
  link.addEventListener("click", (event) => {
    event.preventDefault();
    view.filter = new URL(link.href).searchParams.get("filter");
    link.parentElement
      .querySelectorAll("a")
      .forEach((item) => item.classList.toggle("on", item === link));
    applyView();
  });
});

const search = document.querySelector("form.search");

search.addEventListener("submit", async (event) => {
  event.preventDefault();

  if (!search.elements.query.value.trim()) {
    location.reload();
    return;
  }

  const button = event.submitter || search.querySelector("button");
  if (button) button.disabled = true;

  try {
    const query = new URLSearchParams(new FormData(search));
    let response = await request(`${search.action}?${query}`);

    const paths = await response.json();
    const body = new URLSearchParams();
    paths.forEach((path) => body.append("path", path));
    response = await request("/ui", { method: "POST", body });

    const page = new DOMParser().parseFromString(
      await response.text(),
      "text/html",
    );
    grid.replaceChildren(...page.querySelector(".grid").children);
    applyView();
  } catch (error) {
    alert(error.message || "Failed to search files");
  } finally {
    if (button) button.disabled = false;
  }
});

const showUpload = document.querySelector("button.show-upload");
const files = upload.elements.file;
const progress = document.getElementById("progress");

showUpload.addEventListener("click", () => files.click());

files.addEventListener("change", () => {
  if (files.files.length) upload.requestSubmit();
});

upload.addEventListener("submit", async (event) => {
  event.preventDefault();
  showUpload.disabled = true;
  progress.hidden = false;
  progress.textContent = `uploading ${files.files.length}`;

  try {
    await request(upload.action, {
      method: upload.method,
      body: new FormData(upload),
    });
    location.reload();
  } catch (error) {
    alert(error.message || "Failed to upload files");
    showUpload.disabled = false;
    progress.hidden = true;
    files.value = "";
  }
});

function visibleFiles() {
  return [...grid.querySelectorAll(".item:not(.dir)")].filter(
    (item) => !item.hidden && !item.classList.contains("text"),
  );
}

function openViewer(path, push = true) {
  if (viewer) closeViewer(false);

  const item = [...grid.querySelectorAll(".item")].find(
    (candidate) => candidate.dataset.path === path,
  );
  const type = item ? item.dataset.kind : "text";
  if (type === "text") {
    location.href = `/${encodedPath(path)}`;
    return;
  }

  const files = visibleFiles();
  const index = Math.max(
    0,
    files.findIndex((item) => item.dataset.path === path),
  );
  const prev = files.length
    ? files[(index + files.length - 1) % files.length].dataset.path
    : path;
  const next = files.length
    ? files[(index + 1) % files.length].dataset.path
    : path;
  const content = viewerTemplate.content.cloneNode(true);
  const main = content.querySelector(".viewer");
  const bar = content.querySelector(".viewer-bar");
  const menu = content.querySelector("#menu");
  const media = main.querySelector(`[data-type="${type}"]`);

  galleryMenu.remove();
  grid.style.display = "none";
  galleryBar.style.display = "none";

  media.hidden = false;
  media.id = type === "audio" ? "" : "media";
  media.src = `/${encodedPath(path)}`;
  if (type === "image") media.alt = path.slice(path.lastIndexOf("/") + 1);
  bar.querySelector("#prev").href = viewerUrl(prev);
  bar.querySelector("#close").href = location.pathname;
  bar.querySelector("#next").href = viewerUrl(next);
  menu.querySelectorAll("[data-types]").forEach((control) => {
    control.hidden = !control.dataset.types.split(" ").includes(type);
  });
  document.body.append(content);
  document.title = path.slice(path.lastIndexOf("/") + 1);
  viewer = { path, type, main, bar, menu, media };
  bindViewer(prev, next);

  if (push) history.pushState({ view: path }, "", viewerUrl(path));
}

function closeViewer(push = true) {
  if (!viewer) return;
  viewer.main.remove();
  viewer.bar.remove();
  viewer.menu.remove();
  document.body.append(galleryMenu);
  grid.style.display = "";
  galleryBar.style.display = "";
  document.title = title;
  viewer = null;

  if (push) {
    const url = new URL(location.href);
    url.searchParams.delete("view");
    history.pushState({}, "", `${url.pathname}${url.search}`);
  }
}

function bindViewer(prev, next) {
  const current = viewer;
  current.bar.querySelector("#prev").addEventListener("click", (event) => {
    event.preventDefault();
    openViewer(prev);
  });
  current.bar.querySelector("#next").addEventListener("click", (event) => {
    event.preventDefault();
    openViewer(next);
  });
  current.bar.querySelector("#close").addEventListener("click", (event) => {
    event.preventDefault();
    closeViewer();
  });
  current.menu
    .querySelector("#viewer-download")
    .addEventListener("click", () => {
      submitDownload([current.path]);
    });

  const media = current.media.id === "media" ? current.media : null;
  let zoom = 1;
  let angle = 0;
  let pos = { x: 0, y: 0 };
  let drag = null;

  function transform() {
    if (!media) return;
    if (zoom <= 1) pos = { x: 0, y: 0 };
    media.classList.toggle("zoomed", zoom > 1);
    media.style.transform = `rotate(${angle}deg) scale(${zoom}) translate(${pos.x / zoom}px, ${pos.y / zoom}px)`;
  }

  if (media) {
    media.addEventListener("wheel", (event) => {
      event.preventDefault();
      zoom = Math.max(1, zoom * (event.deltaY > 0 ? 0.9 : 1.1));
      transform();
    });
    media.addEventListener("pointerdown", (event) => {
      if (zoom <= 1) return;
      event.preventDefault();
      media.setPointerCapture(event.pointerId);
      drag = { x: event.clientX - pos.x, y: event.clientY - pos.y };
    });
    media.addEventListener("pointermove", (event) => {
      if (!drag) return;
      pos = { x: event.clientX - drag.x, y: event.clientY - drag.y };
      transform();
    });
    const stopDragging = (event) => {
      if (media.hasPointerCapture(event.pointerId))
        media.releasePointerCapture(event.pointerId);
      drag = null;
    };
    media.addEventListener("pointerup", stopDragging);
    media.addEventListener("pointercancel", stopDragging);
    media.addEventListener("dblclick", () => {
      zoom = zoom > 1 ? 1 : 2;
      transform();
    });
  }

  current.menu.querySelectorAll("[data-zoom]").forEach((button) => {
    button.addEventListener("click", () => {
      const op = button.dataset.zoom;
      if (op === "in") zoom *= 1.25;
      else if (op === "out") zoom = Math.max(1, zoom / 1.25);
      else {
        zoom = 1;
        angle = 0;
      }
      transform();
    });
  });
  current.menu.querySelectorAll("[data-move]").forEach((button) => {
    button.addEventListener("click", () => {
      if (zoom <= 1) return;
      const [dx, dy] = button.dataset.move.split(" ").map(Number);
      pos.x += dx;
      pos.y += dy;
      transform();
    });
  });
  current.menu.querySelectorAll("[data-rot]").forEach((button) => {
    button.addEventListener("click", () => {
      angle += Number(button.dataset.rot);
      transform();
    });
  });

  if (current.type === "video") bindVideo(current);
}

function parseTime(text) {
  const parts = text.trim().split(":");
  if (parts.length !== 2) return null;
  const mins = parseInt(parts[0], 10);
  const secs = parseInt(parts[1], 10);
  return Number.isInteger(mins) &&
    Number.isInteger(secs) &&
    mins >= 0 &&
    secs >= 0 &&
    secs < 60
    ? mins * 60 + secs
    : null;
}

function bindVideo(current) {
  const video = current.media;
  const start = current.menu.querySelector("#loopstart");
  const end = current.menu.querySelector("#loopend");
  let loopStart = 0;
  let loopEnd = 0;

  video.addEventListener("timeupdate", () => {
    if (loopEnd > loopStart && video.currentTime >= loopEnd)
      video.currentTime = loopStart;
  });
  start.addEventListener("change", () => {
    const time = parseTime(start.value);
    if (time === null) start.value = "";
    else loopStart = time;
  });
  end.addEventListener("change", () => {
    const time = parseTime(end.value);
    if (time === null) end.value = "";
    else loopEnd = time;
  });
  current.menu.querySelector("#loopclear").addEventListener("click", () => {
    loopStart = 0;
    loopEnd = 0;
    start.value = "";
    end.value = "";
  });
  current.menu.querySelectorAll("[data-seek]").forEach((button) => {
    button.addEventListener("click", () => {
      video.currentTime = Math.max(
        0,
        video.currentTime + Number(button.dataset.seek),
      );
    });
  });
}

document.addEventListener("keydown", (event) => {
  if (
    !viewer ||
    event.target.matches("input, textarea") ||
    document.querySelector(":popover-open")
  )
    return;
  const link = { Escape: "close", ArrowLeft: "prev", ArrowRight: "next" }[
    event.key
  ];
  if (link) viewer.bar.querySelector(`#${link}`).click();
});

addEventListener("popstate", () => {
  const path = new URL(location.href).searchParams.get("view");
  if (path) openViewer(path, false);
  else closeViewer(false);
});

const initialView = new URL(location.href).searchParams.get("view");
if (initialView) openViewer(initialView, false);
