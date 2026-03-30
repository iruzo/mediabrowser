function showNotification(message) {
  let notification = document.getElementById("uploadNotification");
  if (!notification) {
    notification = document.createElement("div");
    notification.id = "uploadNotification";
    notification.className = "notification";
    document.body.appendChild(notification);
  }

  notification.textContent = message;
  notification.style.display = "block";
}

function hideNotification() {
  const notification = document.getElementById("uploadNotification");
  if (notification) {
    notification.style.display = "none";
  }
}
