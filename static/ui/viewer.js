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
  const current = viewer;
  current.controller.abort();
  if (viewerMenu.matches(":popover-open")) viewerMenu.hidePopover();
  stopDragging();
  resetLoop();
  zoom = 1;
  angle = 0;
  position.x = 0;
  position.y = 0;

  const media = current.media;
  if (current.type === "image") {
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
  if (current.type !== "image") media.load();
  if (current.url) URL.revokeObjectURL(current.url);
  media.hidden = true;
  media.classList.remove("zoomed");
  media.style.removeProperty("transform");
  viewer = null;
}

async function loadViewerFile(current) {
  try {
    const url = await catUrl(current.path, current.controller.signal);
    if (viewer !== current) {
      URL.revokeObjectURL(url);
      return;
    }
    current.url = url;
    current.media.src = url;
  } catch (error) {
    if (viewer === current && error.name !== "AbortError") {
      alert(error.message || "Failed to load file");
      closeViewer();
    }
  }
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
    submitCat(path);
    return;
  }
  if (viewer) resetViewer();

  const paths = viewerFiles(state);
  const index = Math.max(0, paths.indexOf(path));
  const previous = paths.length
    ? paths[(index + paths.length - 1) % paths.length]
    : path;
  const next = paths.length ? paths[(index + 1) % paths.length] : path;

  const current = {
    path,
    type,
    state,
    media,
    previous,
    next,
    controller: new AbortController(),
    url: null,
  };
  viewer = current;
  media.hidden = false;
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
  loadViewerFile(current);
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
    if (loopEnd > loopStart && viewerMedia.video.currentTime >= loopEnd) {
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
