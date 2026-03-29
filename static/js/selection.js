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
    const selectModeBtn = document.getElementById('selectModeBtn');

    if (selectModeBtn) {
        selectModeBtn.textContent = selectionMode ? 'done selecting' : 'select mode';
        if (selectionMode) {
            selectModeBtn.classList.add('active');
        } else {
            selectModeBtn.classList.remove('active');
        }
    }

    const selectionActions = document.getElementById('selectionActions');
    const selectAllBtn = document.getElementById('selectAllBtn');

    if (selectionActions) {
        if (selectionMode) {
            selectionActions.classList.remove('hidden');
        } else {
            selectionActions.classList.add('hidden');
        }
    }

    if (selectAllBtn && selectionMode) {
        const visibleFiles = virtualScrollData.filteredFiles.filter(f => !f.is_dir);
        const allSelected = visibleFiles.length > 0 && visibleFiles.every(f => selectedFiles.has(f.path));
        selectAllBtn.textContent = allSelected ? 'deselect all' : 'select all';
    }

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

function clearSelectionItems() {
    selectedFiles.clear();

    document.querySelectorAll('.grid-item.selected').forEach(item => {
        item.classList.remove('selected');
        const overlay = item.querySelector('.selection-overlay');
        if (overlay) {
            overlay.remove();
        }
    });
}

function clearSelection() {
    clearSelectionItems();
    selectionMode = false;
    updateSelectionUI();
}

function toggleSelectionMode() {
    selectionMode = !selectionMode;
    if (!selectionMode) {
        clearSelectionItems();
    }
    updateSelectionUI();
}

function selectAll() {
    const visibleFiles = virtualScrollData.filteredFiles.filter(f => !f.is_dir);

    visibleFiles.forEach(file => {
        selectedFiles.add(file.path);
    });

    updateSelectionUI();
}

function deselectAll() {
    clearSelectionItems();
    updateSelectionUI();
}

function toggleSelectAll() {
    const visibleFiles = virtualScrollData.filteredFiles.filter(f => !f.is_dir);
    const allSelected = visibleFiles.length > 0 && visibleFiles.every(f => selectedFiles.has(f.path));

    if (allSelected) {
        deselectAll();
    } else {
        selectAll();
    }
}

function getSelectedEntries() {
    const selectedPaths = new Set(selectedFiles);

    return currentFiles.filter(file => selectedPaths.has(file.path));
}

function getRenamedPath(file, name, index, multiple) {
    const lastSlashIndex = file.path.lastIndexOf('/');
    const dirPath = file.path.substring(0, lastSlashIndex);

    if (!multiple) {
        return `${dirPath}/${name}`;
    }

    if (file.is_dir) {
        return `${dirPath}/${name}${index + 1}`;
    }

    const lastDotIndex = file.name.lastIndexOf('.');
    if (lastDotIndex > 0) {
        const extension = file.name.substring(lastDotIndex);
        return `${dirPath}/${name}${index + 1}${extension}`;
    }

    return `${dirPath}/${name}${index + 1}`;
}

async function renameSelected() {
    const selectedEntries = getSelectedEntries();
    if (selectedEntries.length === 0) {
        return;
    }

    const promptLabel = selectedEntries.length === 1 ? 'New name:' : 'Base name:';
    const name = prompt(promptLabel, selectedEntries.length === 1 ? selectedEntries[0].name : '');

    if (name === null) {
        return;
    }

    const trimmedName = name.trim();
    if (!trimmedName) {
        return;
    }

    try {
        for (const [index, file] of selectedEntries.entries()) {
            const renamedPath = getRenamedPath(file, trimmedName, index, selectedEntries.length > 1);
            const response = await fetch(
                `/api/move?from=${encodeURIComponent(file.path)}&to=${encodeURIComponent(renamedPath)}`,
                { method: 'POST' }
            );

            if (!response.ok) {
                throw new Error('failed to rename');
            }
        }

        clearSelection();
        navigateToDirectory(currentPath);
    } catch {
        alert('Some files failed to rename');
    }
}

function downloadSelected() {
    const selectedEntries = getSelectedEntries();
    if (selectedEntries.length === 0) return;

    const hasDirectory = selectedEntries.some(file => file.is_dir);

    if (selectedEntries.length === 1 && !hasDirectory) {
        const filePath = selectedEntries[0].path;
        triggerDownload(`/api/download?path=${encodeURIComponent(filePath)}`);
    } else {
        const paths = selectedEntries.map(file => encodeURIComponent(file.path)).join(',');
        triggerDownload(`/api/download-multiple?paths=${paths}`);
    }
}

function deleteSelected() {
    const selectedEntries = getSelectedEntries();
    if (selectedEntries.length === 0) return;

    const fileCount = selectedEntries.length;
    if (!confirm(`Delete ${fileCount} selected file(s)?`)) return;

    const deletePromises = selectedEntries.map(file =>
        fetch(`/api/delete?path=${encodeURIComponent(file.path)}`, { method: 'DELETE' })
    );

    Promise.all(deletePromises)
        .then(() => {
            clearSelection();
            navigateToDirectory(currentPath);
        })
        .catch(() => alert('Some files failed to delete'));
}
