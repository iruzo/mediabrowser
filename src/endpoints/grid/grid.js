function on(id, handler) {
  const element = document.getElementById(id);
  if (element) element.addEventListener("click", handler);
}

async function post(action, data) {
  const response = await fetch(action, {
    method: "POST",
    body: new URLSearchParams(data),
  });
  if (!response.ok) throw new Error(await response.text());
}

const images = new Set(["bmp", "gif", "ico", "jpeg", "jpg", "png", "svg", "webp"]);
const videos = new Set(["avi", "flv", "m4v", "mkv", "mov", "mp4", "ogv", "webm", "wmv"]);
const audio = new Set(["aac", "flac", "m4a", "mp3", "ogg", "wav", "wma"]);
const root = document.documentElement;
const grid = document.querySelector(".grid");
const galleryBar = document.querySelector(".bar");
const galleryMenu = document.getElementById("menu");
const selectionMenu = document.getElementById("selection-menu");
const title = document.title;
let viewer = null;
let cell = parseInt(localStorage.getItem("cell"), 10) || 0;
let selecting = false;
const view = { sort: "name", filter: "all" };

if (cell) root.style.setProperty("--cell", `${cell}px`);

function extension(path) {
  const name = path.slice(path.lastIndexOf("/") + 1);
  const dot = name.lastIndexOf(".");
  return dot < 0 ? "" : name.slice(dot + 1).toLowerCase();
}

function kind(path, directory) {
  if (directory) return "dir";
  const ext = extension(path);
  if (images.has(ext)) return "image";
  if (videos.has(ext)) return "video";
  if (audio.has(ext)) return "audio";
  return "text";
}

function encodedPath(path) {
  return path.split("/").map(encodeURIComponent).join("/");
}

function viewerUrl(path) {
  const url = new URL(location.href);
  url.searchParams.set("view", path);
  return `${url.pathname}${url.search}`;
}

function makeItem(box) {
  const source = box.querySelector(".actions [name=path]");
  const open = box.querySelector(".open");
  const path = source.value;
  const directory = open.getAttribute("href").startsWith("/ui/");
  const type = kind(path, directory);
  const item = document.createElement("div");
  const link = document.createElement("a");
  const name = document.createElement("span");
  const image = open.querySelector("img");
  const menu = box.querySelector("details.menu");

  item.className = `item ${type}`;
  item.dataset.path = path;
  link.href = directory ? open.getAttribute("href") : viewerUrl(path);
  name.className = "name";
  name.textContent = open.textContent.trim() || path.slice(path.lastIndexOf("/") + 1);

  if (image) link.append(image);
  link.append(name);

  link.addEventListener("click", (event) => {
    if (selecting) {
      event.preventDefault();
      item.classList.toggle("selected");
    } else if (!directory && type !== "text") {
      event.preventDefault();
      openViewer(path);
    }
  });

  if (!directory && type === "text") {
    link.href = `/${encodedPath(path)}`;
  }

  bindBox(box);
  item.append(link, menu);
  return item;
}

function installBoxes(boxes) {
  const items = [...boxes].map(makeItem);
  grid.replaceChildren(...items);
  applyView();
}

installBoxes(grid.querySelectorAll(".box"));

function resizeCell(delta) {
  const size = cell || parseInt(getComputedStyle(root).getPropertyValue("--cell"), 10);
  cell = Math.min(480, Math.max(80, size + delta));
  root.style.setProperty("--cell", `${cell}px`);
  localStorage.setItem("cell", cell);
}

on("cellminus", () => resizeCell(-40));
on("cellplus", () => resizeCell(40));

function selectedItems() {
  const items = [...grid.querySelectorAll(".item.selected")];
  return items.filter((item) => !items.some((parent) => parent !== item
    && parent.classList.contains("dir")
    && item.dataset.path.startsWith(`${parent.dataset.path}/`)));
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
    grid.querySelectorAll(".item.selected").forEach((item) => item.classList.remove("selected"));
  }
});

on("selected-download", () => {
  const items = selectedItems();
  if (!items.length) return;

  const form = document.createElement("form");
  form.method = "post";
  form.action = "/api/download";
  form.hidden = true;
  items.forEach((item) => {
    const input = document.createElement("input");
    input.name = "path";
    input.value = item.dataset.path;
    form.append(input);
  });
  document.body.append(form);
  form.requestSubmit();
  setTimeout(() => form.remove(), 0);
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

    if (input.hidden) {
      const dir = document.querySelector("form.upload [name=path]").value;
      input.value = dir || "/";
      input.hidden = false;
      input.select();
      return;
    }

    const dir = input.value.trim();
    if (!dir) return;
    button.disabled = true;
    const failed = [];

    for (const item of items) {
      try {
        await post(`/api/${action}`, {
          from: item.dataset.path,
          to: destinationPath(dir, item.dataset.path),
        });
      } catch (error) {
        failed.push(`${item.dataset.path}: ${error.message || `Failed to ${action} item`}`);
      }
    }

    if (failed.length) alert(failed.join("\n"));
    location.reload();
  }

  button.addEventListener("click", run);
  input.addEventListener("keydown", (event) => {
    if (event.key !== "Enter") return;
    event.preventDefault();
    run();
  });
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
      failed.push(`${item.dataset.path}: ${error.message || "Failed to remove item"}`);
    }
  }

  if (failed.length) alert(failed.join("\n"));
});

const nameInput = document.getElementById("name");
const mkdir = document.querySelector("form.mkdir");

on("mkdir", async () => {
  const name = nameInput.value.trim();
  if (!name) return;

  const dir = document.querySelector("form.upload [name=path]").value;
  mkdir.elements.path.value = dir ? `${dir}/${name}` : name;

  try {
    await post(mkdir.action, new FormData(mkdir));
    location.reload();
  } catch (error) {
    alert(error.message || "Failed to create folder");
  }
});

function applyView() {
  const items = [...grid.querySelectorAll(".item")];

  items.forEach((item) => {
    item.hidden = view.filter !== "all"
      && !item.classList.contains("dir")
      && !item.classList.contains(view.filter);
  });

  if (view.sort === "name" || view.sort === "type") {
    items.sort((a, b) => {
      const dirs = Number(b.classList.contains("dir")) - Number(a.classList.contains("dir"));
      if (dirs) return dirs;
      if (view.sort === "type") {
        const types = [...a.classList].join(" ").localeCompare([...b.classList].join(" "));
        if (types) return types;
      }
      return a.dataset.path.localeCompare(b.dataset.path, undefined, { sensitivity: "base" });
    });
    grid.append(...items);
  }
}

document.querySelectorAll("a[href^='?sort=']").forEach((link) => {
  link.addEventListener("click", (event) => {
    event.preventDefault();
    view.sort = new URL(link.href).searchParams.get("sort");
    link.parentElement.querySelectorAll("a").forEach((item) => item.classList.toggle("on", item === link));
    applyView();
  });
});

document.querySelectorAll("a[href^='?filter=']").forEach((link) => {
  link.addEventListener("click", (event) => {
    event.preventDefault();
    view.filter = new URL(link.href).searchParams.get("filter");
    link.parentElement.querySelectorAll("a").forEach((item) => item.classList.toggle("on", item === link));
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
    let response = await fetch(`${search.action}?${query}`);
    if (!response.ok) throw new Error(await response.text());

    const paths = await response.json();
    const body = new URLSearchParams();
    paths.forEach((path) => body.append("path", path));
    response = await fetch("/ui", { method: "POST", body });
    if (!response.ok) throw new Error(await response.text());

    const page = new DOMParser().parseFromString(await response.text(), "text/html");
    installBoxes(page.querySelector(".grid").children);
  } catch (error) {
    alert(error.message || "Failed to search files");
  } finally {
    if (button) button.disabled = false;
  }
});

const upload = document.querySelector("form.upload");
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
    const response = await fetch(upload.action, {
      method: upload.method,
      body: new FormData(upload),
    });
    if (!response.ok) throw new Error(await response.text());
    location.reload();
  } catch (error) {
    alert(error.message || "Failed to upload files");
    showUpload.disabled = false;
    progress.hidden = true;
    files.value = "";
  }
});

function visibleFiles() {
  return [...grid.querySelectorAll(".item:not(.dir)")]
    .filter((item) => !item.hidden && !item.classList.contains("text"));
}

function viewerMenu(type) {
  let html = '<button type="button" id="viewer-download">download</button>';
  if (type === "image" || type === "video") {
    html += '<div class="menu-row"><span>zoom</span><button type="button" data-zoom="in">+</button><button type="button" data-zoom="out">-</button><button type="button" data-zoom="reset">reset</button></div>';
    html += '<div class="menu-row"><span>move</span><button type="button" data-move="0 25">up</button><button type="button" data-move="0 -25">down</button><button type="button" data-move="25 0">left</button><button type="button" data-move="-25 0">right</button></div>';
    html += '<div class="menu-row"><span>rotate</span><button type="button" data-rot="-45">left</button><button type="button" data-rot="45">right</button></div>';
  }
  if (type === "video") {
    html += '<div class="menu-row"><span>loop</span><input id="loopstart" placeholder="0:00"><span>-</span><input id="loopend" placeholder="0:00"><button type="button" id="loopclear">clear</button></div>';
    html += '<div class="menu-row"><span>seek</span><button type="button" data-seek="-1">-1s</button><button type="button" data-seek="1">+1s</button></div>';
  }
  return html;
}

function openViewer(path, push = true) {
  if (viewer) closeViewer(false);

  const type = kind(path, false);
  if (type === "text") {
    location.href = `/${encodedPath(path)}`;
    return;
  }

  const files = visibleFiles();
  const index = Math.max(0, files.findIndex((item) => item.dataset.path === path));
  const prev = files.length ? files[(index + files.length - 1) % files.length].dataset.path : path;
  const next = files.length ? files[(index + 1) % files.length].dataset.path : path;
  const main = document.createElement("main");
  const bar = document.createElement("nav");
  const menu = document.createElement("div");
  const media = document.createElement(type === "image" ? "img" : type === "video" ? "video" : "audio");

  galleryMenu.remove();
  grid.style.display = "none";
  galleryBar.style.display = "none";

  main.className = "viewer";
  media.id = type === "audio" ? "" : "media";
  media.src = `/${encodedPath(path)}`;
  if (type === "image") media.alt = path.slice(path.lastIndexOf("/") + 1);
  else media.controls = true;
  main.append(media);

  bar.className = "bar viewer-bar";
  bar.innerHTML = `<a id="prev" href="${viewerUrl(prev)}">prev</a><a id="close" href="${location.pathname}">close</a><a id="next" href="${viewerUrl(next)}">next</a><button type="button" class="menu-toggle" popovertarget="menu">menu</button>`;

  menu.id = "menu";
  menu.popover = "auto";
  menu.innerHTML = viewerMenu(type);
  document.body.append(main, bar, menu);
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
  current.menu.querySelector("#viewer-download").addEventListener("click", () => {
    const form = document.createElement("form");
    const input = document.createElement("input");
    form.method = "post";
    form.action = "/api/download";
    input.name = "path";
    input.value = current.path;
    form.append(input);
    form.hidden = true;
    document.body.append(form);
    form.requestSubmit();
    setTimeout(() => form.remove(), 0);
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
    media.addEventListener("mousedown", (event) => {
      if (zoom <= 1) return;
      event.preventDefault();
      drag = { x: event.clientX - pos.x, y: event.clientY - pos.y };
    });
    media.addEventListener("dblclick", () => {
      zoom = zoom > 1 ? 1 : 2;
      transform();
    });
  }

  document.addEventListener("mousemove", (event) => {
    if (!drag || viewer !== current) return;
    pos = { x: event.clientX - drag.x, y: event.clientY - drag.y };
    transform();
  });
  document.addEventListener("mouseup", () => {
    drag = null;
  }, { once: true });

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
  return Number.isInteger(mins) && Number.isInteger(secs) && mins >= 0 && secs >= 0 && secs < 60
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
    if (loopEnd > loopStart && video.currentTime >= loopEnd) video.currentTime = loopStart;
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
      video.currentTime = Math.max(0, video.currentTime + Number(button.dataset.seek));
    });
  });
}

document.addEventListener("keydown", (event) => {
  if (!viewer || event.target.matches("input, textarea") || document.querySelector(":popover-open")) return;
  const link = { Escape: "close", ArrowLeft: "prev", ArrowRight: "next" }[event.key];
  if (link) viewer.bar.querySelector(`#${link}`).click();
});

addEventListener("popstate", () => {
  const path = new URL(location.href).searchParams.get("view");
  if (path) openViewer(path, false);
  else closeViewer(false);
});

const initialView = new URL(location.href).searchParams.get("view");
if (initialView) openViewer(initialView, false);

// Kept for compatibility with server-rendered action tests; legacy mode replaces these menus.
function bindRm(button) {
  button.addEventListener("click", async () => {
    button.disabled = true;

    try {
      await post(button.form.action, new FormData(button.form));
      button.closest(".item, .box").remove();
    } catch (error) {
      alert(error.message || "Failed to remove item");
      button.disabled = false;
    }
  });
}

function bindPath(button, action) {
  const form = button.form;
  const input = form.querySelector(`input.${action}-to`);

  button.addEventListener("click", async () => {
    if (input.hidden) {
      input.hidden = false;
      input.select();
      return;
    }

    const to = input.value.trim();
    if (!to) return;
    button.disabled = true;

    try {
      await post(`/api/${action}`, { from: form.elements.path.value, to });
      location.reload();
    } catch (error) {
      alert(error.message || `Failed to ${action} item`);
      button.disabled = false;
    }
  });

  input.addEventListener("keydown", (event) => {
    if (event.key !== "Enter") return;
    event.preventDefault();
    button.click();
  });
}

function bindMenu(menu) {
  menu.addEventListener("toggle", () => {
    if (menu.open) return;
    menu.querySelectorAll("input.to").forEach((input) => {
      input.hidden = true;
    });
  });
}

function bindBox(box) {
  bindRm(box.querySelector("button.rm"));
  bindPath(box.querySelector("button.cp"), "cp");
  bindPath(box.querySelector("button.mv"), "mv");
  bindMenu(box.querySelector("details.menu"));
}
