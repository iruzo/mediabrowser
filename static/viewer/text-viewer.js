const LARGE_TEXT_FILE_SIZE = 512 * 1024;
const TEXT_CHUNK_SIZE = 64 * 1024;

let textStreamState = null;

function showTextViewer(file, servePath, content, zoomControls, loopControls, saveTextBtn) {
  if (file.size > LARGE_TEXT_FILE_SIZE) {
    showLargeTextViewer(file, servePath, content);
  } else {
    showSmallTextEditor(servePath, content);
  }

  zoomControls.style.display = "none";
  loopControls.style.display = "none";
  saveTextBtn.style.display = file.size > LARGE_TEXT_FILE_SIZE ? "none" : "block";
}

function showSmallTextEditor(servePath, content) {
  fetch(servePath)
    .then((response) => response.text())
    .then((text) => {
      const container = document.createElement("div");
      container.className = "text-editor-container";

      const textarea = document.createElement("textarea");
      textarea.className = "text-editor";
      textarea.id = "textEditor";
      textarea.value = text;

      container.appendChild(textarea);
      content.innerHTML = "";
      content.appendChild(container);
    })
    .catch(() => {
      content.innerHTML = `<div class="text-viewer">Failed to load file content</div>`;
    });
}

function showLargeTextViewer(file, servePath, content) {
  const viewer = document.createElement("div");
  viewer.className = "text-stream-viewer";

  const body = document.createElement("pre");
  body.className = "text-stream-content";

  const status = document.createElement("div");
  status.className = "text-stream-status";
  status.textContent = `0/${file.size} bytes`;

  viewer.appendChild(body);
  viewer.appendChild(status);
  content.innerHTML = "";
  content.appendChild(viewer);

  textStreamState = {
    filePath: file.path,
    servePath,
    viewer,
    body,
    status,
    offset: 0,
    size: file.size,
    loading: false,
    done: false,
    decoder: new TextDecoder(),
    onScroll: null,
  };

  textStreamState.onScroll = () => {
    if (
      !textStreamState ||
      textStreamState.loading ||
      textStreamState.done ||
      viewer.scrollTop + viewer.clientHeight < viewer.scrollHeight - 400
    ) {
      return;
    }

    loadNextTextChunk();
  };

  viewer.addEventListener("scroll", textStreamState.onScroll);
  loadNextTextChunk();
}

async function loadNextTextChunk() {
  if (!textStreamState || textStreamState.loading || textStreamState.done) {
    return;
  }

  const start = textStreamState.offset;
  const end = Math.min(textStreamState.size - 1, start + TEXT_CHUNK_SIZE - 1);

  textStreamState.loading = true;
  textStreamState.status.textContent = `${start}/${textStreamState.size} bytes`;

  try {
    const response = await fetch(textStreamState.servePath, {
      headers: {
        Range: `bytes=${start}-${end}`,
      },
    });

    if (!response.ok && response.status !== 206) {
      throw new Error("failed to load text chunk");
    }

    const chunk = new Uint8Array(await response.arrayBuffer());
    const text = textStreamState.decoder.decode(chunk, {
      stream: end + 1 < textStreamState.size,
    });

    textStreamState.body.textContent += text;
    textStreamState.offset = end + 1;
    textStreamState.done = textStreamState.offset >= textStreamState.size;

    if (textStreamState.done) {
      textStreamState.body.textContent += textStreamState.decoder.decode();
      textStreamState.status.textContent = `${textStreamState.size}/${textStreamState.size} bytes`;
    } else {
      textStreamState.status.textContent = `${textStreamState.offset}/${textStreamState.size} bytes`;
    }
  } catch {
    if (textStreamState) {
      textStreamState.status.textContent = "Failed to load file content";
    }
  } finally {
    if (textStreamState) {
      textStreamState.loading = false;
    }
  }
}

function cleanupTextStreamViewer() {
  if (!textStreamState) {
    return;
  }

  if (textStreamState.viewer && textStreamState.onScroll) {
    textStreamState.viewer.removeEventListener(
      "scroll",
      textStreamState.onScroll,
    );
  }

  textStreamState = null;
}

function saveTextFile() {
  const textarea = document.getElementById("textEditor");
  if (!textarea || !selectedFile) return;

  if (!confirm(`Save changes to ${selectedFile.name}?`)) {
    return;
  }

  const content = textarea.value;

  fetch(`/api/save?path=${encodeURIComponent(selectedFile.path)}`, {
    method: "POST",
    headers: {
      "Content-Type": "text/plain",
    },
    body: content,
  })
    .then((response) => {
      if (response.ok) {
        alert("File saved successfully");
      } else {
        alert("Failed to save file");
      }
    })
    .catch((err) => {
      alert("Error saving file: " + err.message);
    });
}
