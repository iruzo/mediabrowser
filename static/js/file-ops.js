function triggerUpload() {
    showNotification('Loading...');
    document.getElementById('fileInput').click();
}

function handleFileSelect(files) {
    if (files.length === 0) {
        hideNotification();
        return;
    }

    showNotification(`Uploading ${files.length} file(s)...`);

    setTimeout(() => uploadFiles(files), 0);
}

async function uploadFiles(files) {
    const CONCURRENT_UPLOADS = 3;
    const totalFiles = files.length;
    let completedCount = 0;
    let failedFiles = [];

    async function uploadFile(file) {
        const formData = new FormData();
        formData.append('file', file);

        try {
            const response = await fetch(`/api/upload?path=${encodeURIComponent(currentPath)}`, {
                method: 'POST',
                body: formData
            });

            if (!response.ok) {
                const text = await response.text();
                failedFiles.push(`${file.name}: ${text}`);
            } else {
                await response.json();
            }
        } catch (err) {
            failedFiles.push(`${file.name}: ${err.message}`);
        } finally {
            completedCount++;
            showNotification(`Uploading: ${completedCount}/${totalFiles}`);
        }
    }

    const fileArray = Array.from(files);
    let currentIndex = 0;

    async function processQueue() {
        while (currentIndex < fileArray.length) {
            const file = fileArray[currentIndex++];
            await uploadFile(file);
        }
    }

    const workers = [];
    for (let i = 0; i < Math.min(CONCURRENT_UPLOADS, fileArray.length); i++) {
        workers.push(processQueue());
    }

    await Promise.all(workers);

    hideNotification();

    const successCount = totalFiles - failedFiles.length;
    if (failedFiles.length > 0) {
        alert(`Upload completed: ${successCount} succeeded, ${failedFiles.length} failed\n\nFailed files:\n${failedFiles.join('\n')}`);
    } else if (totalFiles > 0) {
        alert(`Successfully uploaded ${successCount} file(s)`);
    }

    navigateToDirectory(currentPath);
}

function createFolder() {
    const name = prompt('folder name:');
    if (name) {
        const folderPath = `${currentPath}/${name}`;
        fetch(`/api/mkdir?path=${encodeURIComponent(folderPath)}`, { method: 'POST' })
            .then(() => navigateToDirectory(currentPath))
            .catch(() => alert('failed to create folder'));
    }
}
