// The renderer owns the initial DOM. This client only replaces the complete
// browser representation delivered by the native application.
const canvas = document.getElementById("chikachika-canvas");
let widgetNodes = new Map();
let lastRevision = null;

const UUID_V4_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const FONT_FAMILY_CSS = Object.freeze({
  "noto-sans": "'Noto Sans', sans-serif",
  "jetbrains-mono": "'JetBrains Mono', monospace",
});

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function hasOwn(object, property) {
  return Object.prototype.hasOwnProperty.call(object, property);
}

function isFiniteNumber(value) {
  return typeof value === "number" && Number.isFinite(value);
}

function isValidUuidV4(value) {
  return typeof value === "string" && UUID_V4_PATTERN.test(value);
}

function isValidCanvasDimension(value) {
  return (
    Number.isSafeInteger(value) &&
    value > 0 &&
    value <= 0xffffffff
  );
}

function isColorChannel(value) {
  return Number.isInteger(value) && value >= 0 && value <= 255;
}

function isValidColor(color) {
  return (
    isObject(color) &&
    hasOwn(color, "red") &&
    hasOwn(color, "green") &&
    hasOwn(color, "blue") &&
    hasOwn(color, "alpha") &&
    isColorChannel(color.red) &&
    isColorChannel(color.green) &&
    isColorChannel(color.blue) &&
    isColorChannel(color.alpha)
  );
}

function isValidWidget(widget, canvasSize) {
  return (
    isObject(widget) &&
    hasOwn(widget, "widget_id") &&
    isValidUuidV4(widget.widget_id) &&
    hasOwn(widget, "name") &&
    typeof widget.name === "string" &&
    widget.name.trim().length > 0 &&
    hasOwn(widget, "content") &&
    typeof widget.content === "string" &&
    hasOwn(widget, "font_family") &&
    Object.prototype.hasOwnProperty.call(FONT_FAMILY_CSS, widget.font_family) &&
    hasOwn(widget, "position") &&
    isObject(widget.position) &&
    hasOwn(widget.position, "x") &&
    hasOwn(widget.position, "y") &&
    isFiniteNumber(widget.position.x) &&
    isFiniteNumber(widget.position.y) &&
    widget.position.x >= 0 &&
    widget.position.x <= canvasSize.width &&
    widget.position.y >= 0 &&
    widget.position.y <= canvasSize.height &&
    hasOwn(widget, "font_size") &&
    isFiniteNumber(widget.font_size) &&
    widget.font_size > 0 &&
    hasOwn(widget, "color") &&
    isValidColor(widget.color) &&
    hasOwn(widget, "alignment") &&
    (widget.alignment === "left" ||
      widget.alignment === "center" ||
      widget.alignment === "right")
  );
}

function isValidSnapshot(snapshot) {
  if (
    !isObject(snapshot) ||
    !hasOwn(snapshot, "overlay_id") ||
    !isValidUuidV4(snapshot.overlay_id) ||
    !hasOwn(snapshot, "revision") ||
    !Number.isSafeInteger(snapshot.revision) ||
    snapshot.revision < 0 ||
    !hasOwn(snapshot, "canvas") ||
    !isObject(snapshot.canvas) ||
    !hasOwn(snapshot.canvas, "width") ||
    !hasOwn(snapshot.canvas, "height") ||
    !isValidCanvasDimension(snapshot.canvas.width) ||
    !isValidCanvasDimension(snapshot.canvas.height) ||
    !hasOwn(snapshot, "widgets") ||
    !Array.isArray(snapshot.widgets)
  ) {
    return false;
  }

  const widgetIds = new Set();
  for (const widget of snapshot.widgets) {
    if (!isValidWidget(widget, snapshot.canvas) || widgetIds.has(widget.widget_id)) {
      return false;
    }
    widgetIds.add(widget.widget_id);
  }
  return true;
}

function removeElement(element) {
  if (typeof element.remove === "function") {
    element.remove();
  } else if (element.parentNode) {
    element.parentNode.removeChild(element);
  }
}

function setWidgetProperties(element, widget) {
  element.className = "chikachika-text";
  element.setAttribute("data-widget-id", widget.widget_id);
  element.setAttribute("data-widget-name", widget.name);
  element.setAttribute("data-font-family", widget.font_family);
  element.textContent = widget.content;
  element.style.left = `${widget.position.x}px`;
  element.style.top = `${widget.position.y}px`;
  element.style.fontSize = `${widget.font_size}px`;
  element.style.fontFamily = FONT_FAMILY_CSS[widget.font_family];
  element.style.color = `rgba(${widget.color.red}, ${widget.color.green}, ${widget.color.blue}, ${widget.color.alpha / 255})`;
  element.style.textAlign = widget.alignment;
}

function applySnapshot(snapshot) {
  const nextNodes = new Map();
  for (const widget of snapshot.widgets) {
    const element = widgetNodes.get(widget.widget_id) ?? document.createElement("span");
    setWidgetProperties(element, widget);
    nextNodes.set(widget.widget_id, element);
  }

  for (const [widgetId, element] of widgetNodes) {
    if (!nextNodes.has(widgetId)) {
      removeElement(element);
    }
  }

  // Model index zero is frontmost. Appending in reverse model order means the
  // frontmost element is last and paints above the other widgets. appendChild
  // moves an existing node without changing its identity.
  for (let index = snapshot.widgets.length - 1; index >= 0; index -= 1) {
    canvas.appendChild(nextNodes.get(snapshot.widgets[index].widget_id));
  }

  widgetNodes = nextNodes;
  canvas.style.width = `${snapshot.canvas.width}px`;
  canvas.style.height = `${snapshot.canvas.height}px`;
  canvas.setAttribute("data-width", String(snapshot.canvas.width));
  canvas.setAttribute("data-height", String(snapshot.canvas.height));
}

if (canvas && typeof canvas.querySelectorAll === "function") {
  for (const element of canvas.querySelectorAll("[data-widget-id]")) {
    const widgetId = element.getAttribute("data-widget-id");
    if (widgetId !== null) {
      widgetNodes.set(widgetId, element);
    }
  }
}

function handleSnapshot(event) {
  let snapshot;
  try {
    snapshot = JSON.parse(event.data);
  } catch {
    return;
  }

  // Validation is deliberately complete and side-effect free. Do not advance
  // the revision or touch the DOM until every field and every array item passes.
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
