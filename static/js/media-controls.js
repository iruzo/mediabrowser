function setupMediaZoom() {
    const img = document.getElementById('viewerImage');
    const video = document.querySelector('#viewerContent video');
    const element = img || video;

    if (!element) return;

    element.addEventListener('wheel', (e) => {
        e.preventDefault();
        const delta = e.deltaY > 0 ? 0.9 : 1.1;
        zoomLevel *= delta;
        updateMediaTransform();
    });

    element.addEventListener('mousedown', (e) => {
        if (zoomLevel > 1) {
            e.stopPropagation();
            isDragging = true;
            dragStart.x = e.clientX - imagePos.x;
            dragStart.y = e.clientY - imagePos.y;
            element.classList.add('zoomed');
        }
    });

    document.addEventListener('mousemove', (e) => {
        if (isDragging) {
            imagePos.x = e.clientX - dragStart.x;
            imagePos.y = e.clientY - dragStart.y;
            updateMediaTransform();
        }
    });

    document.addEventListener('mouseup', () => {
        isDragging = false;
        const currentElement = document.getElementById('viewerImage') || document.querySelector('#viewerContent video');
        if (currentElement && zoomLevel <= 1) {
            currentElement.classList.remove('zoomed');
        }
    });

    element.addEventListener('dblclick', (e) => {
        if (zoomLevel === 1) {
            e.stopPropagation();
            zoomLevel = 2;
            updateMediaTransform();
        } else {
            e.stopPropagation();
            resetZoom();
        }
    });
}

function zoomIn() {
    zoomLevel *= 1.05;
    updateMediaTransform();
}

function zoomOut() {
    zoomLevel *= 0.95;
    if (zoomLevel < 1) zoomLevel = 1;
    updateMediaTransform();
}

function resetZoom() {
    zoomLevel = 1;
    imagePos = { x: 0, y: 0 };
    isDragging = false;
    rotationAngle = 0;
    updateMediaTransform();
}

function moveUp() {
    if (zoomLevel > 1) {
        imagePos.y += 25;
        updateMediaTransform();
    }
}

function moveDown() {
    if (zoomLevel > 1) {
        imagePos.y -= 25;
        updateMediaTransform();
    }
}

function moveLeft() {
    if (zoomLevel > 1) {
        imagePos.x += 25;
        updateMediaTransform();
    }
}

function moveRight() {
    if (zoomLevel > 1) {
        imagePos.x -= 25;
        updateMediaTransform();
    }
}

function rotateLeft() {
    rotationAngle -= 45;
    updateMediaTransform();
}

function rotateRight() {
    rotationAngle += 45;
    updateMediaTransform();
}

function updateMediaTransform() {
    const img = document.getElementById('viewerImage');
    const video = document.querySelector('#viewerContent video');
    const element = img || video;

    if (!element) return;

    if (zoomLevel > 1) {
        element.classList.add('zoomed');
        element.style.transform = `rotate(${rotationAngle}deg) scale(${zoomLevel}) translate(${imagePos.x / zoomLevel}px, ${imagePos.y / zoomLevel}px)`;
    } else {
        element.classList.remove('zoomed');
        element.style.transform = `rotate(${rotationAngle}deg) scale(1) translate(0, 0)`;
        imagePos = { x: 0, y: 0 };
    }
}

function setupVideoLoop() {
    const video = document.querySelector('#viewerContent video');
    if (!video) return;

    video.addEventListener('timeupdate', () => {
        if (loopEnabled && loopEnd > loopStart && video.currentTime >= loopEnd) {
            video.currentTime = loopStart;
        }
    });

    updateLoopDisplay();
}

function setLoopStart() {
    const video = document.querySelector('#viewerContent video');
    if (!video) return;

    loopStart = video.currentTime;

    if (loopEnd > 0 && loopEnd <= loopStart) {
        loopEnd = 0;
    }

    if (loopEnd > loopStart) {
        loopEnabled = true;
    }

    updateLoopDisplay();
}

function setLoopEnd() {
    const video = document.querySelector('#viewerContent video');
    if (!video) return;

    loopEnd = video.currentTime;

    if (loopEnd <= loopStart) {
        alert('Loop end must be after loop start');
        loopEnd = 0;
        return;
    }

    loopEnabled = true;
    updateLoopDisplay();
}

function clearLoop() {
    loopEnabled = false;
    loopStart = 0;
    loopEnd = 0;
    updateLoopDisplay();
}

function seekBackward() {
    const video = document.querySelector('#viewerContent video');
    if (!video) return;

    video.currentTime = Math.max(0, video.currentTime - 1);
}

function seekForward() {
    const video = document.querySelector('#viewerContent video');
    if (!video) return;

    video.currentTime = Math.min(video.duration, video.currentTime + 1);
}

function updateLoopDisplay() {
    const startInput = document.getElementById('loopStartInput');
    const endInput = document.getElementById('loopEndInput');

    if (!startInput || !endInput) return;

    startInput.value = formatTime(loopStart);
    endInput.value = formatTime(loopEnd);
}

function onLoopTimeChange(isStart) {
    const video = document.querySelector('#viewerContent video');
    if (!video) return;

    const startInput = document.getElementById('loopStartInput');
    const endInput = document.getElementById('loopEndInput');

    if (isStart) {
        const time = parseTime(startInput.value);
        if (time === null) {
            alert('Invalid time format. Use MM:SS (e.g., 1:30)');
            startInput.value = formatTime(loopStart);
            return;
        }

        if (time > video.duration) {
            alert('Start time exceeds video duration');
            startInput.value = formatTime(loopStart);
            return;
        }

        loopStart = time;

        if (loopEnd > 0 && loopEnd <= loopStart) {
            loopEnd = 0;
            endInput.value = formatTime(0);
        }

        if (loopEnd > loopStart) {
            loopEnabled = true;
        }
    } else {
        const time = parseTime(endInput.value);
        if (time === null) {
            alert('Invalid time format. Use MM:SS (e.g., 1:30)');
            endInput.value = formatTime(loopEnd);
            return;
        }

        if (time > video.duration) {
            alert('End time exceeds video duration');
            endInput.value = formatTime(loopEnd);
            return;
        }

        if (time <= loopStart) {
            alert('Loop end must be after loop start');
            endInput.value = formatTime(loopEnd);
            return;
        }

        loopEnd = time;
        loopEnabled = true;
    }

    updateLoopDisplay();
}
