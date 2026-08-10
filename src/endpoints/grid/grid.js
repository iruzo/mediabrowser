async function post(action, data) {
  const response = await fetch(action, {
    method: "POST",
    body: new URLSearchParams(data),
  });

  if (!response.ok) throw new Error(await response.text());
}

document.querySelectorAll("button.rm").forEach((button) => {
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
});

function pathAction(action) {
  document.querySelectorAll(`button.${action}`).forEach((button) => {
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
  });
}

pathAction("cp");
pathAction("mv");

const mkdir = document.querySelector("form.mkdir");
const showMkdir = document.querySelector("button.show-mkdir");

showMkdir.addEventListener("click", () => {
  mkdir.hidden = false;
  mkdir.elements.path.focus();
});

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
