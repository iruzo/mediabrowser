async function post(form) {
  const response = await fetch(form.action, {
    method: form.method,
    body: new URLSearchParams(new FormData(form)),
  });

  if (!response.ok) throw new Error(await response.text());
}

document.querySelectorAll("button.rm").forEach((button) => {
  button.addEventListener("click", async () => {
    button.disabled = true;

    try {
      await post(button.form);
      button.closest(".box").remove();
    } catch (error) {
      alert(error.message || "Failed to remove item");
      button.disabled = false;
    }
  });
});

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
    await post(mkdir);
    location.reload();
  } catch (error) {
    alert(error.message || "Failed to create folder");
    button.disabled = false;
  }
});
