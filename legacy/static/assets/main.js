function on(id, handler) {
  const el = document.getElementById(id);
  if (el) el.addEventListener("click", handler);
}

// gallery: grid cell size
const root = document.documentElement;
let cell = parseInt(localStorage.getItem("cell"), 10) || 0;
if (cell) root.style.setProperty("--cell", cell + "px");

function resizeCell(delta) {
  const size =
    cell || parseInt(getComputedStyle(root).getPropertyValue("--cell"), 10);
  cell = Math.min(480, Math.max(80, size + delta));
  root.style.setProperty("--cell", cell + "px");
  localStorage.setItem("cell", cell);
}

on("cellminus", () => resizeCell(-40));
on("cellplus", () => resizeCell(40));

// gallery: select all / none
on("selectall", () => {
  const boxes = [...document.querySelectorAll(".item input[type=checkbox]")];
  const all = boxes.length > 0 && boxes.every((box) => box.checked);
  boxes.forEach((box) => {
    box.checked = !all;
  });
});

// gallery: upload
const fileInput = document.getElementById("file");
on("upload", () => fileInput.click());

async function uploadFiles(files) {
  const dir = document.querySelector("#grid [name=dir]").value;
  const progress = document.getElementById("progress");
  const failed = [];

  progress.hidden = false;
  for (let i = 0; i < files.length; i++) {
    progress.textContent = `uploading ${i + 1}/${files.length}`;
    const body = new FormData();
    body.append("path", dir);
    body.append("file", files[i]);
    try {
      const response = await fetch("/api/upload", { method: "POST", body });
      if (!response.ok) {
        failed.push(`${files[i].name}: ${await response.text()}`);
      }
    } catch (err) {
      failed.push(`${files[i].name}: ${err.message}`);
    }
  }

  if (failed.length > 0) {
    alert(`failed uploads:\n${failed.join("\n")}`);
  }
  location.reload();
}

if (fileInput) {
  fileInput.addEventListener("change", () => {
    if (fileInput.files.length > 0) uploadFiles([...fileInput.files]);
  });
}

// viewer: keyboard navigation
document.addEventListener("keydown", (e) => {
  if (e.target.matches("input, textarea")) return;
  if (document.querySelector(":popover-open")) return;
  const id = { Escape: "close", ArrowLeft: "prev", ArrowRight: "next" }[e.key];
  const link = id && document.getElementById(id);
  if (link) link.click();
});

// viewer: zoom, pan, rotate
const media = document.getElementById("media");
let zoom = 1;
let angle = 0;
let pos = { x: 0, y: 0 };
let drag = null;

function applyTransform() {
  if (zoom <= 1) pos = { x: 0, y: 0 };
  media.classList.toggle("zoomed", zoom > 1);
  media.style.transform = `rotate(${angle}deg) scale(${zoom}) translate(${pos.x / zoom}px, ${pos.y / zoom}px)`;
}

if (media) {
  media.addEventListener("wheel", (e) => {
    e.preventDefault();
    zoom = Math.max(1, zoom * (e.deltaY > 0 ? 0.9 : 1.1));
    applyTransform();
  });

  media.addEventListener("mousedown", (e) => {
    if (zoom > 1) {
      e.preventDefault();
      drag = { x: e.clientX - pos.x, y: e.clientY - pos.y };
    }
  });

  document.addEventListener("mousemove", (e) => {
    if (drag) {
      pos = { x: e.clientX - drag.x, y: e.clientY - drag.y };
      applyTransform();
    }
  });

  document.addEventListener("mouseup", () => {
    drag = null;
  });

  media.addEventListener("dblclick", () => {
    zoom = zoom > 1 ? 1 : 2;
    applyTransform();
  });

  document.querySelectorAll("[data-zoom]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const op = btn.dataset.zoom;
      if (op === "in") zoom *= 1.25;
      else if (op === "out") zoom = Math.max(1, zoom / 1.25);
      else {
        zoom = 1;
        angle = 0;
      }
      applyTransform();
    });
  });

  document.querySelectorAll("[data-move]").forEach((btn) => {
    btn.addEventListener("click", () => {
      if (zoom <= 1) return;
      const [dx, dy] = btn.dataset.move.split(" ").map(Number);
      pos.x += dx;
      pos.y += dy;
      applyTransform();
    });
  });

  document.querySelectorAll("[data-rot]").forEach((btn) => {
    btn.addEventListener("click", () => {
      angle += Number(btn.dataset.rot);
      applyTransform();
    });
  });
}

// viewer: video A-B loop and seek
const video = media && media.tagName === "VIDEO" ? media : null;

function parseTime(text) {
  const parts = text.trim().split(":");
  if (parts.length !== 2) return null;
  const mins = parseInt(parts[0], 10);
  const secs = parseInt(parts[1], 10);
  if (isNaN(mins) || isNaN(secs) || mins < 0 || secs < 0 || secs >= 60) {
    return null;
  }
  return mins * 60 + secs;
}

if (video) {
  const startInput = document.getElementById("loopstart");
  const endInput = document.getElementById("loopend");
  let loopStart = 0;
  let loopEnd = 0;

  video.addEventListener("timeupdate", () => {
    if (loopEnd > loopStart && video.currentTime >= loopEnd) {
      video.currentTime = loopStart;
    }
  });

  startInput.addEventListener("change", () => {
    const time = parseTime(startInput.value);
    if (time === null) startInput.value = "";
    else loopStart = time;
  });

  endInput.addEventListener("change", () => {
    const time = parseTime(endInput.value);
    if (time === null) endInput.value = "";
    else loopEnd = time;
  });

  on("loopclear", () => {
    loopStart = 0;
    loopEnd = 0;
    startInput.value = "";
    endInput.value = "";
  });

  document.querySelectorAll("[data-seek]").forEach((btn) => {
    btn.addEventListener("click", () => {
      video.currentTime = Math.max(
        0,
        video.currentTime + Number(btn.dataset.seek),
      );
    });
  });
}
