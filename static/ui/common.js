const BATCH_SIZE = 60;

function on(id, handler) {
  const element = document.getElementById(id);
  if (element) element.addEventListener("click", handler);
}

function post(action, data, signal) {
  return request(action, {
    method: "POST",
    body: new URLSearchParams(data),
    signal,
  });
}

async function catUrl(path, signal) {
  const response = await post("/api/cat", { path }, signal);
  return URL.createObjectURL(await response.blob());
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
    return path.split("/").filter(Boolean).map(decodeURIComponent).join("/");
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
