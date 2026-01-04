const performanceUtils = {
    throttle(func, delay) {
        let timeoutId;
        let lastExecTime = 0;
        return function (...args) {
            const currentTime = Date.now();
            if (currentTime - lastExecTime > delay) {
                func.apply(this, args);
                lastExecTime = currentTime;
            } else {
                clearTimeout(timeoutId);
                timeoutId = setTimeout(() => {
                    func.apply(this, args);
                    lastExecTime = Date.now();
                }, delay - (currentTime - lastExecTime));
            }
        };
    },

    debounce(func, delay) {
        let timeoutId;
        return function (...args) {
            clearTimeout(timeoutId);
            timeoutId = setTimeout(() => func.apply(this, args), delay);
        };
    }
};

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function encodeURIPath(path) {
    return path.split('/').map(segment => encodeURIComponent(segment)).join('/');
}

function determineFileType(filename) {
    const ext = filename.split('.').pop().toLowerCase();

    const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'bmp', 'webp', 'svg', 'ico'];
    const videoExts = ['mp4', 'avi', 'mkv', 'mov', 'wmv', 'flv', 'webm', 'm4v', 'ogv'];
    const audioExts = ['mp3', 'wav', 'flac', 'aac', 'ogg', 'wma', 'm4a'];

    if (imageExts.includes(ext)) return 'image';
    if (videoExts.includes(ext)) return 'video';
    if (audioExts.includes(ext)) return 'audio';

    return 'text';
}

function formatTime(seconds) {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs.toString().padStart(2, '0')}`;
}

function parseTime(timeStr) {
    const parts = timeStr.trim().split(':');
    if (parts.length !== 2) return null;

    const mins = parseInt(parts[0], 10);
    const secs = parseInt(parts[1], 10);

    if (isNaN(mins) || isNaN(secs) || mins < 0 || secs < 0 || secs >= 60) {
        return null;
    }

    return mins * 60 + secs;
}

function showNotification(message) {
    let notification = document.getElementById('uploadNotification');
    if (!notification) {
        notification = document.createElement('div');
        notification.id = 'uploadNotification';
        notification.style.cssText = 'position:fixed;bottom:20px;right:20px;background:#333;color:#fff;padding:20px 30px;border-radius:8px;font-size:16px;z-index:10000;box-shadow:0 4px 6px rgba(0,0,0,0.3);';
        document.body.appendChild(notification);
    }
    notification.textContent = message;
    notification.style.display = 'block';
}

function hideNotification() {
    const notification = document.getElementById('uploadNotification');
    if (notification) {
        notification.style.display = 'none';
    }
}

function hideContextMenu() {
    document.getElementById('contextMenu').style.display = 'none';
}
