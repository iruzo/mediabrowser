function renderGallery(files) {
  if (intersectionObserver) {
    intersectionObserver.disconnect();
  }

  let filteredFiles = filterFiles(files);
  filteredFiles = filterBySearchTerm(filteredFiles);
  virtualScrollData.filteredFiles = sortFiles(filteredFiles);

  initializeVirtualScroll(galleryGrid);

  const savedScroll = sessionStorage.getItem("galleryScrollPosition");
  if (savedScroll && galleryContainer) {
    requestAnimationFrame(() => {
      galleryContainer.scrollTop = parseInt(savedScroll, 10);
      sessionStorage.removeItem("galleryScrollPosition");
    });
  }
}

function initializeVirtualScroll(grid) {
  grid.innerHTML = "";
  virtualScrollData.renderedItems.clear();

  if (virtualScrollData.scrollContainer && virtualScrollData.scrollHandler) {
    virtualScrollData.scrollContainer.removeEventListener(
      "scroll",
      virtualScrollData.scrollHandler,
    );
  }

  const container = grid.parentElement;
  virtualScrollData.scrollContainer = container;
  virtualScrollData.containerHeight = container.clientHeight;

  virtualScrollData.lastContainerDimensions = {
    width: container.clientWidth,
    height: container.clientHeight,
  };

  const containerWidth = container.clientWidth;
  const gridSizeVh =
    parseInt(
      getComputedStyle(document.documentElement).getPropertyValue(
        "--grid-size",
      ),
    ) || 30;
  const baseGridSizePx = (gridSizeVh / 200) * window.innerHeight;

  const minColumns = Math.max(1, Math.floor(containerWidth / baseGridSizePx));
  const actualGridSizePx =
    minColumns === 1
      ? containerWidth
      : (containerWidth - (minColumns - 1) * 2) / minColumns;

  virtualScrollData.itemHeight = actualGridSizePx + 2;
  virtualScrollData.actualItemSize = actualGridSizePx;
  virtualScrollData.columns = minColumns;
  virtualScrollData.rows = Math.ceil(
    virtualScrollData.filteredFiles.length / minColumns,
  );

  const spacer = document.createElement("div");
  spacer.id = "virtual-spacer";
  spacer.style.height = `${virtualScrollData.rows * virtualScrollData.itemHeight}px`;
  spacer.style.position = "relative";
  grid.appendChild(spacer);
  virtualScrollData.spacer = spacer;

  virtualScrollData.scrollHandler = performanceUtils.throttle(
    handleVirtualScroll,
    16,
  );
  container.addEventListener("scroll", virtualScrollData.scrollHandler);

  renderVisibleItems();
}

function handleVirtualScroll() {
  virtualScrollData.scrollPosition =
    virtualScrollData.scrollContainer.scrollTop;
  renderVisibleItems();
}

function renderVisibleItems() {
  const containerHeight = virtualScrollData.scrollContainer.clientHeight;
  const scrollTop = virtualScrollData.scrollPosition;
  const itemHeight = virtualScrollData.itemHeight;
  const columns = virtualScrollData.columns;

  if (columns <= 0) return;

  const startRow = Math.max(0, Math.floor(scrollTop / itemHeight) - 1);
  const endRow = Math.min(
    virtualScrollData.rows - 1,
    Math.ceil((scrollTop + containerHeight) / itemHeight) + 1,
  );

  const startIndex = startRow * columns;
  const endIndex = Math.min(
    virtualScrollData.filteredFiles.length - 1,
    (endRow + 1) * columns - 1,
  );

  virtualScrollData.visibleRange = { start: startIndex, end: endIndex };

  const spacer = virtualScrollData.spacer;
  const newLazyItems = [];

  for (const [index, item] of virtualScrollData.renderedItems) {
    if (index < startIndex || index > endIndex) {
      item.onclick = null;

      const img = item.querySelector("img");
      const video = item.querySelector("video");
      if (img) {
        img.onload = null;
        img.onerror = null;
        img.src = "";
      }
      if (video) {
        video.src = "";
        video.load();
      }

      if (intersectionObserver) {
        intersectionObserver.unobserve(item);
      }
      item.remove();
      virtualScrollData.renderedItems.delete(index);
    }
  }

  for (let i = startIndex; i <= endIndex; i++) {
    if (
      !virtualScrollData.renderedItems.has(i) &&
      i < virtualScrollData.filteredFiles.length
    ) {
      const file = virtualScrollData.filteredFiles[i];
      const item = createGridItem(file, i);
      virtualScrollData.renderedItems.set(i, item);
      spacer.appendChild(item);
      if (item.classList.contains("lazy-load")) {
        newLazyItems.push(item);
      }
    }
  }

  if (newLazyItems.length > 0) {
    observeLazyItems(newLazyItems);
  }
}

function createGridItem(file, index) {
  const item = document.createElement("div");
  item.className = "grid-item";
  item.dataset.filePath = file.path;
  item.dataset.fileType = file.file_type;
  item.dataset.fileName = file.name;
  item.onclick = (e) => handleFileClick(e, file);

  const columns = virtualScrollData.columns;
  const row = Math.floor(index / columns);
  const col = index % columns;

  const gridSizePx = virtualScrollData.actualItemSize;

  item.style.position = "absolute";
  item.style.top = `${row * virtualScrollData.itemHeight}px`;
  item.style.left = `${col * (gridSizePx + 2)}px`;
  item.style.width = `${gridSizePx}px`;
  item.style.height = `${gridSizePx}px`;

  if (selectedFiles.has(file.path)) {
    item.classList.add("selected");
  }

  const hasPreview = file.file_type === "image";
  const label = file.is_dir ? `${file.name}/` : file.name;

  item.innerHTML = `<div class="file-name">${escapeHtml(label)}</div>`;

  if (!hasPreview || file.is_dir) {
    item.classList.add("always-show-name");
  }

  if (file.is_dir) {
    item.classList.add("directory");
  } else if (hasPreview) {
    item.classList.add("lazy-load");
  } else {
    item.classList.add("file");
  }

  if (selectedFiles.has(file.path)) {
    const overlay = document.createElement("div");
    overlay.className = "selection-overlay";
    item.appendChild(overlay);
  }

  return item;
}

function ensureLazyLoadObserver() {
  if (intersectionObserver) {
    return;
  }

  intersectionObserver = new IntersectionObserver(
    (entries) => {
      requestAnimationFrame(() => {
        entries.forEach((entry) => {
          if (!entry.isIntersecting) {
            return;
          }

          const item = entry.target;
          if (!item.classList.contains("lazy-load")) {
            return;
          }

          intersectionObserver.unobserve(item);
          delete item.dataset.lazyObserved;
          loadItemContent(item);
        });
      });
    },
    {
      root: null,
      rootMargin: "25px",
      threshold: 0.2,
    },
  );
}

function observeLazyItems(items) {
  ensureLazyLoadObserver();

  items.forEach((item) => {
    if (item.dataset.lazyObserved === "true") {
      return;
    }

    item.dataset.lazyObserved = "true";
    intersectionObserver.observe(item);
  });
}

function loadItemContent(item) {
  const filePath = item.dataset.filePath;
  const fileType = item.dataset.fileType;
  const fileName = item.dataset.fileName;
  const label = item.querySelector(".file-name");

  item.classList.remove("lazy-load");

  if (fileType === "image") {
    const img = document.createElement("img");
    img.loading = "lazy";
    img.alt = escapeHtml(fileName);
    img.style.opacity = "0";
    img.style.transition = "opacity 0.2s";

    img.onload = () => {
      img.style.opacity = "1";
    };

    img.onerror = () => {
      img.style.opacity = "0.5";
      img.alt = "Failed to load";
    };

    const pathWithoutData = filePath.replace("/data", "") || "/";
    const servePath = encodeURIPath(pathWithoutData);
    img.src = servePath;
    item.innerHTML = "";
    item.appendChild(img);
    if (label) {
      item.appendChild(label);
    }
  } else if (fileType === "video") {
    item.classList.add("file");
    return;
  }

  if (selectedFiles.has(filePath)) {
    const overlay = document.createElement("div");
    overlay.className = "selection-overlay";
    item.appendChild(overlay);
  }
}

function filterFiles(files) {
  if (currentFilter === "all") return files;
  if (currentFilter === "image")
    return files.filter((f) => f.is_dir || f.file_type === "image");
  if (currentFilter === "video")
    return files.filter((f) => f.is_dir || f.file_type === "video");
  if (currentFilter === "audio")
    return files.filter((f) => f.is_dir || f.file_type === "audio");
  if (currentFilter === "text")
    return files.filter((f) => f.is_dir || f.file_type === "text");
  return files;
}

function changeFilter(filterType) {
  currentFilter = filterType;
  renderGallery(currentFiles);
}

function changeSort(sortType) {
  currentSort = sortType;
  renderGallery(currentFiles);
}

const debouncedSearch = performanceUtils.debounce((term, requestId) => {
  runSearch(term, requestId);
}, 300);

function filterBySearch(term) {
  searchTerm = term.trim();
  const requestId = ++activeSearchRequest;

  if (!searchTerm) {
    currentFiles = currentDirectoryFiles;
    renderGallery(currentDirectoryFiles);
    return;
  }

  debouncedSearch(searchTerm, requestId);
}

function filterBySearchTerm(files) {
  return files;
}

async function runSearch(term, requestId) {
  try {
    const response = await fetch(
      `/api/search?path=${encodeURIComponent(currentPath)}&query=${encodeURIComponent(term)}`,
    );

    if (!response.ok) {
      throw new Error("failed to search");
    }

    if (requestId !== activeSearchRequest) {
      return;
    }

    const files = await response.json();
    currentFiles = files.map((file) => ({
      ...file,
      file_type: file.is_dir ? "directory" : determineFileType(file.name),
    }));
    renderGallery(currentFiles);
  } catch {
    if (requestId !== activeSearchRequest) {
      return;
    }

    currentFiles = [];
    renderGallery([]);
  }
}

function sortFiles(files) {
  const sorted = [...files];

  sorted.sort((a, b) => {
    if (a.name === "..") return -1;
    if (b.name === "..") return 1;

    if (a.is_dir !== b.is_dir) {
      return a.is_dir ? -1 : 1;
    }

    switch (currentSort) {
      case "name":
        return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
      case "date":
        return b.modified - a.modified;
      case "size":
        return b.size - a.size;
      case "type":
        if (a.file_type !== b.file_type) {
          return a.file_type.localeCompare(b.file_type);
        }
        return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
      default:
        return 0;
    }
  });

  return sorted;
}

function handleFileClick(e, file) {
  if (selectionMode || e.ctrlKey || e.metaKey) {
    e.preventDefault();
    toggleFileSelection(file, e.currentTarget);
  } else if (selectedFiles.size > 0) {
    clearSelection();
    openMedia(file);
  } else {
    openMedia(file);
  }
}

function increaseGridSize() {
  gridSize = Math.min(gridSize + 5, 35);
  updateGridSize();
}

function decreaseGridSize() {
  gridSize = Math.max(gridSize - 5, 10);
  updateGridSize();
}

function updateGridSize() {
  document.documentElement.style.setProperty("--grid-size", gridSize + "vh");
  localStorage.setItem("gridSize", gridSize);

  if (virtualScrollData.filteredFiles.length > 0) {
    const currentScroll = galleryContainer ? galleryContainer.scrollTop : 0;
    initializeVirtualScroll(galleryGrid);

    if (galleryContainer) {
      requestAnimationFrame(() => {
        galleryContainer.scrollTop = currentScroll;
      });
    }
  }
}

function initializeGridSize() {
  const savedSize = localStorage.getItem("gridSize");
  if (savedSize) {
    gridSize = parseInt(savedSize);
  }
  updateGridSize();
}

const resizeHandler = performanceUtils.debounce(() => {
  if (virtualScrollData.filteredFiles.length > 0) {
    const container = virtualScrollData.scrollContainer;
    if (container) {
      const currentWidth = container.clientWidth;
      const currentHeight = container.clientHeight;

      if (
        Math.abs(
          currentWidth - virtualScrollData.lastContainerDimensions.width,
        ) > 10 ||
        Math.abs(
          currentHeight - virtualScrollData.lastContainerDimensions.height,
        ) > 10
      ) {
        const scrollPercentage =
          container.scrollTop /
          (container.scrollHeight - container.clientHeight || 1);

        initializeVirtualScroll(galleryGrid);

        requestAnimationFrame(() => {
          const newScrollTop =
            scrollPercentage *
            (container.scrollHeight - container.clientHeight);
          container.scrollTop = newScrollTop;
        });

        virtualScrollData.lastContainerDimensions = {
          width: currentWidth,
          height: currentHeight,
        };
      }
    }
  }
}, 250);
