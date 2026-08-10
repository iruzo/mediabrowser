document.querySelectorAll("button.rm").forEach((button) => {
  button.addEventListener("click", async () => {
    const form = button.form;

    button.disabled = true;

    try {
      const response = await fetch("/api/rm", {
        method: "POST",
        body: new URLSearchParams(new FormData(form)),
      });

      if (!response.ok) throw new Error(await response.text());

      form.closest(".box").remove();
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

  const button = mkdir.querySelector("button");

  button.disabled = true;

  try {
    const response = await fetch("/api/mkdir", {
      method: "POST",
      body: new URLSearchParams(new FormData(mkdir)),
    });

    if (!response.ok) throw new Error(await response.text());

    location.reload();
  } catch (error) {
    alert(error.message || "Failed to create folder");
    button.disabled = false;
  }
});
