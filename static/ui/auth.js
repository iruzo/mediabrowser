const authDialog = document.getElementById("auth-dialog");
const authForm = document.getElementById("auth-form");
const authTitle = document.getElementById("auth-title");
const authError = document.getElementById("auth-error");
const authSubmit = document.getElementById("auth-submit");
const authCancel = document.getElementById("auth-cancel");
let authMode = "login";

function showAuth(mode) {
  if (authDialog.open && authMode === mode) return;
  authMode = mode;
  const login = mode === "login";
  authForm.reset();
  authForm.action = login ? "/api/login" : "/api/password";
  authForm.elements.username.hidden = !login;
  authForm.elements.username.disabled = !login;
  authForm.elements.new_password.hidden = login;
  authForm.elements.new_password.disabled = login;
  authForm.elements.password.placeholder = login
    ? "password"
    : "current password";
  authForm.elements.password.setAttribute(
    "aria-label",
    login ? "password" : "current password",
  );
  authTitle.textContent = login ? "login" : "change password";
  authSubmit.textContent = authTitle.textContent;
  authCancel.hidden = login;
  authError.hidden = true;
  document
    .querySelectorAll(":popover-open")
    .forEach((menu) => menu.hidePopover());
  if (!authDialog.open) authDialog.showModal();
  authForm.elements[login ? "username" : "password"].focus();
}

authDialog.addEventListener("cancel", (event) => {
  if (authMode === "login" || authSubmit.disabled) event.preventDefault();
});
authDialog.addEventListener("keydown", (event) => event.stopPropagation());
authDialog.addEventListener("close", () => authForm.reset());
authCancel.addEventListener("click", () => authDialog.close());

authForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (authSubmit.disabled) return;
  const mode = authMode;
  authSubmit.disabled = true;
  authCancel.disabled = true;
  authError.hidden = true;
  try {
    const response = await fetch(authForm.action, {
      method: "POST",
      body: new URLSearchParams(new FormData(authForm)),
    });
    if (mode !== authMode) return;
    if (!response.ok) {
      const message = await response.text();
      if (
        mode === "password" &&
        response.status === 401 &&
        message === "Login required"
      ) {
        showAuth("login");
      }
      throw new Error(message || "Authentication failed");
    }
    authForm.reset();
    if (mode === "login") location.reload();
    else authDialog.close();
  } catch (error) {
    authError.textContent = error.message || "Authentication failed";
    authError.hidden = false;
  } finally {
    authSubmit.disabled = false;
    authCancel.disabled = false;
  }
});

for (const id of ["menu", "viewer-menu"]) {
  const menu = document.getElementById(id);
  const password = document.createElement("button");
  password.type = "button";
  password.textContent = "change password";
  password.addEventListener("click", () => showAuth("password"));
  const logout = document.createElement("button");
  logout.type = "button";
  logout.textContent = "logout";
  logout.addEventListener("click", async () => {
    logout.disabled = true;
    try {
      await request("/api/logout");
      location.reload();
    } catch (error) {
      alert(error.message || "Logout failed");
      logout.disabled = false;
    }
  });
  menu.append(password, logout);
}
