async function post(action, data) {
  const response = await fetch(action, {
    method: "POST",
    body: new URLSearchParams(data),
  });

  if (!response.ok) throw new Error(await response.text());
}

function bindRm(button) {
  button.addEventListener("click", async () => {
    button.disabled = true;

    try {
      await post(button.form.action, new FormData(button.form));
      button.closest(".box").remove();
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

document.querySelectorAll(".box").forEach(bindBox);

const grid = document.querySelector(".grid");
const search = document.querySelector("form.search");

search.addEventListener("submit", async (event) => {
  event.preventDefault();

  if (!search.elements.query.value.trim()) {
    location.reload();
    return;
  }

  const button = event.submitter;
  button.disabled = true;

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
    grid.replaceChildren(...page.querySelector(".grid").children);
    grid.querySelectorAll(".box").forEach(bindBox);
  } catch (error) {
    alert(error.message || "Failed to search files");
  } finally {
    button.disabled = false;
  }
});

const mkdir = document.querySelector("form.mkdir");

mkdir.addEventListener("submit", async (event) => {
  event.preventDefault();

  const button = event.submitter;

  button.disabled = true;

  try {
    await post(mkdir.action, new FormData(mkdir));
    location.reload();
  } catch (error) {
    alert(error.message || "Failed to create folder");
    button.disabled = false;
  }
});

const upload = document.querySelector("form.upload");
const showUpload = document.querySelector("button.show-upload");
const files = upload.elements.file;

showUpload.addEventListener("click", () => {
  files.click();
});

files.addEventListener("change", () => {
  if (files.files.length) upload.requestSubmit();
});

upload.addEventListener("submit", async (event) => {
  event.preventDefault();
  showUpload.disabled = true;

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
    files.value = "";
  }
});
