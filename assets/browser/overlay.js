// The renderer owns the initial DOM. This client only replaces the complete
// browser representation delivered by the native application.
const canvas = document.getElementById("chikachika-canvas");
let textElement = canvas?.querySelector("[data-widget-id]") ?? null;
let lastRevision = null;

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isFiniteNumber(value) {
  return typeof value === "number" && Number.isFinite(value);
}

function isColorChannel(value) {
  return Number.isInteger(value) && value >= 0 && value <= 255;
}

function isValidWidget(widget) {
  return (
    isObject(widget) &&
    typeof widget.widget_id === "string" &&
    widget.widget_id.length > 0 &&
    typeof widget.content === "string" &&
    isObject(widget.position) &&
    isFiniteNumber(widget.position.x) &&
    isFiniteNumber(widget.position.y) &&
    isFiniteNumber(widget.font_size) &&
    widget.font_size > 0 &&
    isObject(widget.color) &&
    isColorChannel(widget.color.red) &&
    isColorChannel(widget.color.green) &&
    isColorChannel(widget.color.blue) &&
    isColorChannel(widget.color.alpha) &&
    (widget.alignment === "left" ||
      widget.alignment === "center" ||
      widget.alignment === "right")
  );
}

function isValidSnapshot(snapshot) {
  return (
    isObject(snapshot) &&
    typeof snapshot.overlay_id === "string" &&
    snapshot.overlay_id.length > 0 &&
    Number.isSafeInteger(snapshot.revision) &&
    snapshot.revision >= 0 &&
    isObject(snapshot.canvas) &&
    Number.isInteger(snapshot.canvas.width) &&
    snapshot.canvas.width > 0 &&
    Number.isInteger(snapshot.canvas.height) &&
    snapshot.canvas.height > 0 &&
    (snapshot.text_widget === null || isValidWidget(snapshot.text_widget))
  );
}

function removeTextElement() {
  if (!textElement) {
    return;
  }

  if (typeof textElement.remove === "function") {
    textElement.remove();
  } else if (textElement.parentNode) {
    textElement.parentNode.removeChild(textElement);
  }
  textElement = null;
}

function applyWidget(widget) {
  if (!textElement) {
    textElement = document.createElement("span");
    textElement.className = "chikachika-text";
    canvas.appendChild(textElement);
  }

  textElement.className = "chikachika-text";
  textElement.setAttribute("data-widget-id", widget.widget_id);
  textElement.textContent = widget.content;
  textElement.style.left = `${widget.position.x}px`;
  textElement.style.top = `${widget.position.y}px`;
  textElement.style.fontSize = `${widget.font_size}px`;
  textElement.style.color = `rgba(${widget.color.red}, ${widget.color.green}, ${widget.color.blue}, ${widget.color.alpha / 255})`;
  textElement.style.textAlign = widget.alignment;
}

function applySnapshot(snapshot) {
  canvas.style.width = `${snapshot.canvas.width}px`;
  canvas.style.height = `${snapshot.canvas.height}px`;
  canvas.setAttribute("data-width", String(snapshot.canvas.width));
  canvas.setAttribute("data-height", String(snapshot.canvas.height));

  if (snapshot.text_widget === null) {
    removeTextElement();
  } else {
    applyWidget(snapshot.text_widget);
  }
}

function handleSnapshot(event) {
  let snapshot;
  try {
    snapshot = JSON.parse(event.data);
  } catch {
    return;
  }

  if (!isValidSnapshot(snapshot)) {
    return;
  }
  if (lastRevision !== null && snapshot.revision <= lastRevision) {
    return;
  }

  applySnapshot(snapshot);
  lastRevision = snapshot.revision;
}

const overlayPath = window.location.pathname.replace(/\/$/, "");
const eventSource = new EventSource(`${overlayPath}/events`);
eventSource.addEventListener("snapshot", handleSnapshot);

window.ChikachikaOverlay = Object.freeze({
  canvas,
});
