function openMedia(file) {
    if (file.is_dir) {
        navigateToDirectory(file.path);
        return;
    }

    const container = document.querySelector('.gallery-container');
    if (container) {
        sessionStorage.setItem('galleryScrollPosition', container.scrollTop);
    }

    const pathWithoutData = file.path.replace('/data', '');
    const fileUrl = '/ui' + encodeURIPath(pathWithoutData);
    window.location.href = fileUrl;
}

function showCurrentMedia() {
    if (currentMediaIndex < 0 || currentMediaIndex >= currentFiles.length) return;

    const file = currentFiles[currentMediaIndex];
    selectedFile = file;
    const viewer = document.getElementById('viewer');
    const content = document.getElementById('viewerContent');
    const zoomControls = document.getElementById('zoomControls');
    const loopControls = document.getElementById('loopControls');
    const pathWithoutData = file.path.replace('/data', '') || '/';
    const servePath = encodeURIPath(pathWithoutData);

    if (file.file_type === 'image') {
        content.innerHTML = `<img src="${servePath}" alt="${escapeHtml(file.name)}" id="viewerImage">`;
        setupMediaZoom();
        updateMediaTransform();
        zoomControls.style.display = 'block';
        loopControls.style.display = 'none';
    } else if (file.file_type === 'video') {
        content.innerHTML = `<video src="${servePath}" controls></video>`;
        setupMediaZoom();
        updateMediaTransform();
        zoomControls.style.display = 'block';
        loopControls.style.display = 'block';
        setupVideoLoop();
    } else if (file.file_type === 'audio') {
        content.innerHTML = `<audio src="${servePath}" controls></audio>`;
        zoomControls.style.display = 'none';
        loopControls.style.display = 'none';
    } else {
        fetch(servePath)
            .then(response => response.text())
            .then(text => {
                content.innerHTML = `
                    <div class="text-editor-container">
                        <textarea class="text-editor" id="textEditor">${escapeHtml(text)}</textarea>
                        <div class="text-editor-controls">
                            <button class="viewer-btn" onclick="saveTextFile()">save</button>
                            <button class="viewer-btn" onclick="cancelTextEdit()">cancel</button>
                        </div>
                    </div>
                `;
            })
            .catch(() => {
                content.innerHTML = `<div class="text-viewer">Failed to load file content</div>`;
            });
        zoomControls.style.display = 'none';
        loopControls.style.display = 'none';
    }

    viewer.classList.add('active');
}

function nextMedia() {
    const mediaFiles = currentFiles.filter(f => !f.is_dir);
    if (mediaFiles.length === 0) return;

    const currentMediaFile = mediaFiles.find(f => f.path === selectedFile.path);
    const currentIndex = mediaFiles.indexOf(currentMediaFile);
    const nextIndex = currentIndex < mediaFiles.length - 1 ? currentIndex + 1 : 0;

    currentMediaIndex = currentFiles.findIndex(f => f.path === mediaFiles[nextIndex].path);
    clearLoop();
    showCurrentMedia();
}

function previousMedia() {
    const mediaFiles = currentFiles.filter(f => !f.is_dir);
    if (mediaFiles.length === 0) return;

    const currentMediaFile = mediaFiles.find(f => f.path === selectedFile.path);
    const currentIndex = mediaFiles.indexOf(currentMediaFile);
    const prevIndex = currentIndex > 0 ? currentIndex - 1 : mediaFiles.length - 1;

    currentMediaIndex = currentFiles.findIndex(f => f.path === mediaFiles[prevIndex].path);
    clearLoop();
    showCurrentMedia();
}

function closeViewer() {
    const video = document.querySelector('#viewerContent video');
    if (video) {
        video.pause();
        video.src = '';
        video.load();
    }

    const audio = document.querySelector('#viewerContent audio');
    if (audio) {
        audio.pause();
        audio.src = '';
        audio.load();
    }

    document.getElementById('zoomControls').style.display = 'none';
    document.getElementById('loopControls').style.display = 'none';

    document.getElementById('viewer').classList.remove('active');
    selectedFile = null;
    resetZoom();
    clearLoop();

    navigateToDirectory(currentPath);
}

function downloadCurrent() {
    if (selectedFile) {
        if (selectedFile.is_dir) {
            window.open(`/api/download-multiple?paths=${encodeURIComponent(selectedFile.path)}`);
        } else {
            window.open(`/api/download?path=${encodeURIComponent(selectedFile.path)}`);
        }
    }
}

function saveTextFile() {
    const textarea = document.getElementById('textEditor');
    if (!textarea || !selectedFile) return;

    if (!confirm(`Save changes to ${selectedFile.name}?`)) {
        return;
    }

    const content = textarea.value;

    fetch(`/api/save?path=${encodeURIComponent(selectedFile.path)}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'text/plain',
        },
        body: content
    })
        .then(response => {
            if (response.ok) {
                alert('File saved successfully');
            } else {
                alert('Failed to save file');
            }
        })
        .catch(err => {
            alert('Error saving file: ' + err.message);
        });
}

function cancelTextEdit() {
    closeViewer();
}
