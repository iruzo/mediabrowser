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
