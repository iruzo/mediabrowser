function toggleFileSelection(file, clickedElement) {
    const isSelected = selectedFiles.has(file.path);

    if (isSelected) {
        selectedFiles.delete(file.path);
    } else {
        selectedFiles.add(file.path);
    }

    if (clickedElement) {
        const existingOverlay = clickedElement.querySelector('.selection-overlay');

        if (!isSelected && !existingOverlay) {
            clickedElement.classList.add('selected');
            const overlay = document.createElement('div');
            overlay.className = 'selection-overlay';
            clickedElement.appendChild(overlay);
        } else if (isSelected && existingOverlay) {
            clickedElement.classList.remove('selected');
            existingOverlay.remove();
        }
    }

    updateSelectionUI();
}

function updateSelectionUI() {
    const hasSelection = selectedFiles.size > 0;
    const selectBtn = document.getElementById('selectBtn');

    selectBtn.textContent = selectionMode ? 'done' : 'select';
    if (selectionMode) {
        selectBtn.classList.add('active');
    } else {
        selectBtn.classList.remove('active');
    }

    document.getElementById('downloadBtn').style.display = hasSelection ? 'inline-block' : 'none';
    document.getElementById('deleteBtn').style.display = hasSelection ? 'inline-block' : 'none';
    document.getElementById('clearBtn').style.display = hasSelection ? 'inline-block' : 'none';

    updateVirtualItemsSelection();
}

function updateVirtualItemsSelection() {
    const currentSelected = new Set(selectedFiles);

    for (const [index, item] of virtualScrollData.renderedItems) {
        const file = virtualScrollData.filteredFiles[index];
        if (file) {
            const isSelected = currentSelected.has(file.path);
            const wasSelected = virtualScrollData.selectedItemsCache.has(file.path);

            if (isSelected !== wasSelected) {
                item.classList.toggle('selected', isSelected);

                const existingOverlay = item.querySelector('.selection-overlay');
                if (isSelected && !existingOverlay) {
                    const overlay = document.createElement('div');
                    overlay.className = 'selection-overlay';
                    item.appendChild(overlay);
                } else if (!isSelected && existingOverlay) {
                    existingOverlay.remove();
                }
            }
        }
    }

    virtualScrollData.selectedItemsCache = currentSelected;
}

function clearSelection() {
    selectedFiles.clear();
    selectionMode = false;

    document.querySelectorAll('.grid-item.selected').forEach(item => {
        item.classList.remove('selected');
        const overlay = item.querySelector('.selection-overlay');
        if (overlay) {
            overlay.remove();
        }
    });

    updateSelectionUI();
}

function toggleSelectionMode() {
    selectionMode = !selectionMode;
    if (!selectionMode) {
        selectedFiles.clear();

        document.querySelectorAll('.grid-item.selected').forEach(item => {
            item.classList.remove('selected');
            const overlay = item.querySelector('.selection-overlay');
            if (overlay) {
                overlay.remove();
            }
        });
    }
    updateSelectionUI();
}

function downloadSelected() {
    if (selectedFiles.size === 0) return;

    const selectedPaths = Array.from(selectedFiles);

    const hasDirectory = selectedPaths.some(path => {
        const file = currentFiles.find(f => f.path === path);
        return file && file.is_dir;
    });

    if (selectedFiles.size === 1 && !hasDirectory) {
        const filePath = selectedPaths[0];
        window.open(`/api/download?path=${encodeURIComponent(filePath)}`);
    } else {
        const paths = selectedPaths.map(path => encodeURIComponent(path)).join(',');
        window.open(`/api/download-multiple?paths=${paths}`);
    }
}

function deleteSelected() {
    if (selectedFiles.size === 0) return;

    const fileCount = selectedFiles.size;
    if (!confirm(`Delete ${fileCount} selected file(s)?`)) return;

    const deletePromises = Array.from(selectedFiles).map(filePath =>
        fetch(`/api/delete?path=${encodeURIComponent(filePath)}`, { method: 'DELETE' })
    );

    Promise.all(deletePromises)
        .then(() => {
            clearSelection();
            navigateToDirectory(currentPath);
        })
        .catch(() => alert('Some files failed to delete'));
}

function showContextMenu(e, file) {
    e.preventDefault();
    selectedFile = file;
    const menu = document.getElementById('contextMenu');
    menu.style.display = 'block';
    menu.style.left = e.pageX + 'px';
    menu.style.top = e.pageY + 'px';
}

function downloadFile() {
    if (selectedFile) {
        if (selectedFile.is_dir) {
            window.open(`/api/download-multiple?paths=${encodeURIComponent(selectedFile.path)}`);
        } else {
            window.open(`/api/download?path=${encodeURIComponent(selectedFile.path)}`);
        }
    }
    hideContextMenu();
}

function deleteFile() {
    if (selectedFile && confirm(`delete ${selectedFile.name}?`)) {
        fetch(`/api/delete?path=${encodeURIComponent(selectedFile.path)}`, { method: 'DELETE' })
            .then(() => navigateToDirectory(currentPath))
            .catch(() => alert('delete failed'));
    }
    hideContextMenu();
}
