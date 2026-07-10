const searchInput = document.getElementById("searchInput");
const searchContainer = document.getElementById("searchContainer");
const viewerDropdown = document.getElementById("viewerDropdown");

function closeDropdownOnOutsideClick(dropdown, target) {
  if (dropdown && !dropdown.contains(target)) {
    dropdown.classList.remove("open");
  }
}

function updateSearchPosition() {
  if (!searchContainer) {
    return;
  }

  const visualViewport = window.visualViewport;
  if (!visualViewport) {
    searchContainer.style.bottom = "";
    return;
  }

  const layoutHeight = window.innerHeight;
  const visibleBottom = visualViewport.height + visualViewport.offsetTop;
  const keyboardInset = Math.max(0, layoutHeight - visibleBottom);

  searchContainer.style.bottom = `${keyboardInset}px`;
}

function init() {
  initializeGridSize();

  window.addEventListener("resize", resizeHandler);
  window.addEventListener("resize", updateSearchPosition);

  document.addEventListener("click", (e) => {
    closeDropdownOnOutsideClick(viewerDropdown, e.target);
    closeItemContextMenuOnOutsideClick(e.target);
  });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      if (closeItemContextMenu()) {
        return;
      }
      closeViewer();
    } else if (e.key === "ArrowLeft") {
      previousMedia();
    } else if (e.key === "ArrowRight") {
      nextMedia();
    }
  });

  if (window.visualViewport) {
    window.visualViewport.addEventListener("resize", updateSearchPosition);
    window.visualViewport.addEventListener("scroll", updateSearchPosition);
  }

  if (searchInput) {
    searchInput.addEventListener("focus", updateSearchPosition);
    searchInput.addEventListener("blur", updateSearchPosition);
  }

  updateSearchPosition();
  loadInitialDirectory();
}

init();
