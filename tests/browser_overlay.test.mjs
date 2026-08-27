import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const script = readFileSync(new URL("../assets/browser/overlay.js", import.meta.url), "utf8");

class FakeElement {
  constructor(tagName) {
    this.tagName = tagName;
    this.children = [];
    this.parentNode = null;
    this.attributes = new Map();
    this.style = {};
    this.className = "";
    this._textContent = "";
  }

  get textContent() {
    return this._textContent;
  }

  set textContent(value) {
    this._textContent = String(value);
  }

  setAttribute(name, value) {
    this.attributes.set(name, String(value));
  }

  getAttribute(name) {
    return this.attributes.get(name) ?? null;
  }

  appendChild(child) {
    child.parentNode = this;
    this.children.push(child);
    return child;
  }

  removeChild(child) {
    const index = this.children.indexOf(child);
    if (index >= 0) {
      this.children.splice(index, 1);
      child.parentNode = null;
    }
    return child;
  }

  remove() {
    this.parentNode?.removeChild(this);
  }

  querySelector(selector) {
    if (selector !== "[data-widget-id]") {
      throw new Error(`unsupported selector: ${selector}`);
    }
    for (const child of this.children) {
      if (child.getAttribute("data-widget-id") !== null) {
        return child;
      }
      const descendant = child.querySelector(selector);
      if (descendant) {
        return descendant;
      }
    }
    return null;
  }
}

class FakeDocument {
  constructor(canvas) {
    this.canvas = canvas;
  }

  getElementById(id) {
    assert.equal(id, "chikachika-canvas");
    return this.canvas;
  }

  createElement(tagName) {
    return new FakeElement(tagName);
  }
}

class FakeEventSource {
  static instances = [];

  constructor(url) {
    this.url = url;
    this.listeners = new Map();
    FakeEventSource.instances.push(this);
  }

  addEventListener(name, callback) {
    this.listeners.set(name, callback);
  }

  dispatch(name, data) {
    this.listeners.get(name)?.({ data: JSON.stringify(data) });
  }
}

function snapshot(revision, widget = null) {
  return {
    overlay_id: "overlay-stable-id",
    revision,
    canvas: { width: 1280, height: 720 },
    text_widget: widget,
  };
}

function widget(widget_id, content = "hello") {
  return {
    widget_id,
    content,
    position: { x: 12.5, y: 34.25 },
    font_size: 42.5,
    color: { red: 10, green: 20, blue: 30, alpha: 128 },
    alignment: "center",
  };
}

function runScript() {
  FakeEventSource.instances = [];
  const canvas = new FakeElement("main");
  canvas.setAttribute("data-width", "640");
  canvas.setAttribute("data-height", "360");
  canvas.style.width = "640px";
  canvas.style.height = "360px";
  const initial = new FakeElement("span");
  initial.className = "chikachika-text";
  initial.setAttribute("data-widget-id", "initial-widget");
  initial.textContent = "server-rendered text";
  canvas.appendChild(initial);

  const window = {
    location: { pathname: "/overlay/overlay-stable-id/" },
  };
  const document = new FakeDocument(canvas);
  vm.runInNewContext(script, { window, document, EventSource: FakeEventSource });

  assert.equal(FakeEventSource.instances.length, 1);
  const source = FakeEventSource.instances[0];
  assert.equal(source.url, "/overlay/overlay-stable-id/events");
  assert.deepEqual([...source.listeners.keys()], ["snapshot"]);
  assert.equal(initial.textContent, "server-rendered text");
  assert.equal(canvas.children.length, 1);
  return { canvas, initial, source, window };
}

test("overlay script subscribes to same-origin named snapshots and applies complete updates safely", () => {
  const { canvas, initial, source } = runScript();

  source.dispatch("snapshot", snapshot(0, widget("widget-1", "<img src=x onerror=alert(1)>")));
  assert.equal(canvas.style.width, "1280px");
  assert.equal(canvas.style.height, "720px");
  assert.equal(canvas.getAttribute("data-width"), "1280");
  assert.equal(canvas.getAttribute("data-height"), "720");
  assert.strictEqual(canvas.children[0], initial);
  assert.equal(initial.getAttribute("data-widget-id"), "widget-1");
  assert.equal(initial.className, "chikachika-text");
  assert.equal(initial.textContent, "<img src=x onerror=alert(1)>");
  assert.equal(initial.style.left, "12.5px");
  assert.equal(initial.style.top, "34.25px");
  assert.equal(initial.style.fontSize, "42.5px");
  assert.equal(initial.style.color, "rgba(10, 20, 30, 0.5019607843137255)");
  assert.equal(initial.style.textAlign, "center");

  const updated = widget("widget-1", "updated");
  updated.position = { x: 1, y: 2 };
  updated.font_size = 18;
  updated.color = { red: 255, green: 0, blue: 128, alpha: 255 };
  updated.alignment = "right";
  source.dispatch("snapshot", snapshot(1, updated));
  assert.strictEqual(canvas.children[0], initial);
  assert.equal(initial.textContent, "updated");
  assert.equal(initial.style.left, "1px");
  assert.equal(initial.style.top, "2px");
  assert.equal(initial.style.fontSize, "18px");
  assert.equal(initial.style.color, "rgba(255, 0, 128, 1)");
  assert.equal(initial.style.textAlign, "right");

  source.dispatch("snapshot", snapshot(1, widget("widget-ignored", "duplicate")));
  source.dispatch("snapshot", snapshot(0, widget("widget-ignored", "stale")));
  assert.equal(initial.getAttribute("data-widget-id"), "widget-1");
  assert.equal(initial.textContent, "updated");

  source.dispatch("snapshot", snapshot(2, null));
  assert.equal(canvas.children.length, 0);

  source.dispatch("snapshot", snapshot(3, widget("widget-created", "created")));
  assert.equal(canvas.children.length, 1);
  const created = canvas.children[0];
  assert.notStrictEqual(created, initial);
  assert.equal(created.getAttribute("data-widget-id"), "widget-created");
  assert.equal(created.textContent, "created");
});

test("overlay script rejects malformed snapshots and never uses HTML injection or polling", () => {
  assert.equal(script.includes("innerHTML"), false);
  assert.equal(script.includes("insertAdjacentHTML"), false);
  assert.equal(script.includes("setInterval"), false);
  assert.equal(script.includes("setTimeout"), false);

  const { canvas, source } = runScript();
  source.dispatch("snapshot", snapshot(0, widget("valid", "valid")));
  const before = canvas.children[0].textContent;

  source.dispatch("snapshot", "not an object");
  source.dispatch("snapshot", { ...snapshot(1), revision: Number.MAX_SAFE_INTEGER + 1 });
  source.dispatch("snapshot", { ...snapshot(1), revision: -1 });
  source.dispatch("snapshot", { ...snapshot(1), canvas: { width: 0, height: 720 } });
  source.dispatch("snapshot", {
    ...snapshot(1),
    text_widget: { ...widget("bad"), alignment: "justify" },
  });
  source.listeners.get("snapshot")({ data: "not JSON" });

  assert.equal(canvas.children[0].textContent, before);
});
