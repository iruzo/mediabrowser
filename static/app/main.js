const searchInput = document.getElementById("searchInput");
const toolbarDropdown = document.getElementById("toolbarDropdown");
const viewerDropdown = document.getElementById("viewerDropdown");
const toolbarToggle = document.querySelector(".toolbar-dropdown-toggle");

function closeDropdownOnOutsideClick(dropdown, target) {
  if (dropdown && !dropdown.contains(target)) {
    dropdown.classList.remove("open");
  }
}

function updateToolbarDropdownPosition() {
  if (!toolbarDropdown) {
    return;
  }

  const visualViewport = window.visualViewport;
  if (!visualViewport) {
    toolbarDropdown.style.bottom = "";
    return;
  }

  const layoutHeight = window.innerHeight;
  const visibleBottom = visualViewport.height + visualViewport.offsetTop;
  const keyboardInset = Math.max(0, layoutHeight - visibleBottom);

  toolbarDropdown.style.bottom = `${keyboardInset}px`;
}

function init() {
  initializeGridSize();

  window.addEventListener("resize", resizeHandler);
  window.addEventListener("resize", updateToolbarDropdownPosition);

  document.addEventListener("click", (e) => {
    closeDropdownOnOutsideClick(viewerDropdown, e.target);
    closeDropdownOnOutsideClick(toolbarDropdown, e.target);
  });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      closeViewer();
    } else if (e.key === "ArrowLeft") {
      previousMedia();
    } else if (e.key === "ArrowRight") {
      nextMedia();
    }
  });

  if (window.visualViewport) {
    window.visualViewport.addEventListener(
      "resize",
      updateToolbarDropdownPosition,
    );
    window.visualViewport.addEventListener(
      "scroll",
      updateToolbarDropdownPosition,
    );
  }

  if (searchInput) {
    searchInput.addEventListener("focus", updateToolbarDropdownPosition);
    searchInput.addEventListener("blur", updateToolbarDropdownPosition);
  }

  if (toolbarToggle) {
    toolbarToggle.addEventListener("mousedown", (e) => {
      e.preventDefault();
    });
  }

  updateToolbarDropdownPosition();
  loadInitialDirectory();
}

init();
