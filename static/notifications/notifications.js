const notificationState = {
  items: new Map(),
  spinnerTimer: null,
};

const dotsSpinner = {
  frames: ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
  interval: 80,
};

function getNotificationContainer() {
  let container = document.getElementById("notificationContainer");
  if (!container) {
    container = document.createElement("div");
    container.id = "notificationContainer";
    container.className = "notification-container";
    document.body.appendChild(container);
  }

  return container;
}

function ensureNotification(id) {
  let entry = notificationState.items.get(id);
  if (entry) {
    return entry;
  }

  const element = document.createElement("div");
  element.className = "notification";
  element.dataset.notificationId = id;
  element.addEventListener("click", () => hideNotification(id));

  const spinner = document.createElement("span");
  spinner.className = "notification-spinner";

  const message = document.createElement("span");
  message.className = "notification-message";

  const progress = document.createElement("span");
  progress.className = "notification-progress";

  element.appendChild(spinner);
  element.appendChild(message);
  element.appendChild(progress);
  getNotificationContainer().appendChild(element);

  entry = {
    id,
    element,
    spinner,
    message,
    progress,
    spinnerFrame: 0,
    timerId: null,
    options: {},
  };

  notificationState.items.set(id, entry);
  return entry;
}

function renderNotification(entry) {
  const options = entry.options;
  const hasSpinner = Boolean(options.spinner);
  const hasProgress = options.progress && options.progress.total > 0;

  entry.spinner.textContent = hasSpinner
    ? dotsSpinner.frames[entry.spinnerFrame % dotsSpinner.frames.length]
    : "";
  entry.spinner.style.display = hasSpinner ? "inline-flex" : "none";

  entry.message.textContent = options.message || "";

  if (hasProgress) {
    const percentage = Math.round(
      (options.progress.current / options.progress.total) * 100,
    );
    entry.progress.textContent = `${options.progress.current}/${options.progress.total} (${percentage}%)`;
    entry.progress.style.display = "inline-flex";
  } else {
    entry.progress.textContent = "";
    entry.progress.style.display = "none";
  }

  entry.element.style.display = "flex";
}

function updateSpinnerState() {
  let hasAnimatedNotifications = false;

  for (const entry of notificationState.items.values()) {
    if (!entry.options.spinner) {
      continue;
    }

    entry.spinnerFrame = (entry.spinnerFrame + 1) % dotsSpinner.frames.length;
    renderNotification(entry);
    hasAnimatedNotifications = true;
  }

  if (!hasAnimatedNotifications && notificationState.spinnerTimer) {
    clearInterval(notificationState.spinnerTimer);
    notificationState.spinnerTimer = null;
  }
}

function ensureSpinnerTimer() {
  if (!notificationState.spinnerTimer) {
    notificationState.spinnerTimer = setInterval(
      updateSpinnerState,
      dotsSpinner.interval,
    );
  }
}

function showNotification(message, options = {}) {
  const id = options.id || "default";
  const entry = ensureNotification(id);

  if (entry.timerId) {
    clearTimeout(entry.timerId);
    entry.timerId = null;
  }

  entry.options = {
    timeout: 2000,
    persistent: false,
    spinner: false,
    progress: null,
    ...options,
    message,
  };

  renderNotification(entry);

  if (entry.options.spinner) {
    ensureSpinnerTimer();
  }

  if (!entry.options.persistent) {
    entry.timerId = setTimeout(() => {
      hideNotification(id);
    }, entry.options.timeout);
  }
}

function hideNotification(id = "default") {
  const entry = notificationState.items.get(id);
  if (!entry) {
    return;
  }

  if (entry.timerId) {
    clearTimeout(entry.timerId);
  }

  entry.element.remove();
  notificationState.items.delete(id);

  if (notificationState.items.size === 0) {
    const container = document.getElementById("notificationContainer");
    if (container) {
      container.remove();
    }
  }

  updateSpinnerState();
}

function showUploadPickerNotification() {
  showNotification("Loading...", {
    id: "upload-picker",
    spinner: true,
    persistent: true,
  });
}

function hideUploadPickerNotification() {
  hideNotification("upload-picker");
}

function showUploadProgress(current, total) {
  showNotification("Uploading", {
    id: "upload",
    spinner: true,
    persistent: true,
    progress: { current, total },
  });
}

function hideUploadProgress() {
  hideNotification("upload");
}
