function navigateToDirectory(path) {
    const urlPath = path === '/data' ? '/ui/' : '/ui' + path.replace('/data', '') + '/';
    window.location.href = urlPath;
}

function loadInitialDirectory() {
    const pathname = decodeURIComponent(window.location.pathname);

    const isFileUrl = pathname !== '/ui' && pathname !== '/ui/' && !pathname.endsWith('/');

    if (isFileUrl) {
        const pathWithoutUI = pathname.substring(3);
        const lastSlashIndex = pathWithoutUI.lastIndexOf('/');
        const dirPathSuffix = pathWithoutUI.substring(0, lastSlashIndex);
        const dataPath = dirPathSuffix ? '/data' + dirPathSuffix : '/data';
        const filePath = '/data' + pathWithoutUI;

        currentPath = dataPath;

        const servePath = dataPath === '/data' ? '/' : dataPath.replace('/data', '') + '/';
        fetch(servePath)
            .then(response => response.text())
            .then(html => {
                const files = parseServerHtml(html, dataPath);
                currentFiles = files;

                const file = currentFiles.find(f => f.path === filePath);
                if (file) {
                    selectedFile = file;
                    currentMediaIndex = currentFiles.findIndex(f => f.path === file.path);
                    resetZoom();
                    showCurrentMedia();
                }
            })
            .catch(() => {
                document.getElementById('galleryGrid').innerHTML = '<div>error</div>';
            });
    } else {
        let dataPath = '/data';
        if (pathname.startsWith('/ui/') && pathname !== '/ui/') {
            const pathWithoutUI = pathname.substring(3);
            dataPath = '/data' + pathWithoutUI.slice(0, -1);
        }

        currentPath = dataPath;

        const servePath = dataPath === '/data' ? '/' : dataPath.replace('/data', '') + '/';
        fetch(servePath)
            .then(response => response.text())
            .then(html => {
                const files = parseServerHtml(html, dataPath);
                currentFiles = files;
                renderGallery(files);
            })
            .catch(() => {
                document.getElementById('galleryGrid').innerHTML = '<div>error</div>';
            });
    }
}

function parseServerHtml(html, currentPath) {
    const parser = new DOMParser();
    const doc = parser.parseFromString(html, 'text/html');
    const links = doc.querySelectorAll('ul li a');
    const files = [];

    links.forEach(link => {
        const href = link.getAttribute('href');
        const text = link.textContent.trim();

        if (!href || href === '../') return;

        const isDir = href.endsWith('/');
        const name = isDir ? text.replace(/\/$/, '') : text;
        const filePath = currentPath === '/data'
            ? `/data/${name}`
            : `${currentPath}/${name}`;

        const fileType = isDir ? 'directory' : determineFileType(name);

        files.push({
            name: name,
            path: filePath,
            is_dir: isDir,
            file_type: fileType,
            size: 0,
            modified: ''
        });
    });

    return files;
}

function refreshView() {
    navigateToDirectory(currentPath);
}
