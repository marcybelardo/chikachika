import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const script = readFileSync(new URL("../assets/browser/overlay.js", import.meta.url), "utf8");

const OVERLAY_ID = "11111111-1111-4111-8111-111111111111";
const FRONT_ID = "22222222-2222-4222-8222-222222222222";
const MIDDLE_ID = "33333333-3333-4333-8333-333333333333";
const BACK_ID = "44444444-4444-4444-8444-444444444444";
const COPY_ID = "55555555-5555-4555-8555-555555555555";
const NEW_ID = "66666666-6666-4666-8666-666666666666";

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
    if (child.parentNode) {
      child.parentNode.removeChild(child);
    }
    child.parentNode = this;
    this.children.push(child);
    return child;
  }

  removeChild(child) {
    const index = this.children.indexOf(child);
    if (index < 0) {
      throw new Error("child is not present");
    }
    this.children.splice(index, 1);
    child.parentNode = null;
    return child;
  }

  remove() {
    this.parentNode?.removeChild(this);
  }

  matches(selector) {
    if (selector !== "[data-widget-id]") {
      throw new Error(`unsupported selector: ${selector}`);
    }
    return this.getAttribute("data-widget-id") !== null;
  }

  querySelector(selector) {
    return this.querySelectorAll(selector)[0] ?? null;
  }

  querySelectorAll(selector) {
    const matches = [];
    const visit = (element) => {
      for (const child of element.children) {
        if (child.matches(selector)) {
          matches.push(child);
        }
        visit(child);
      }
    };
    visit(this);
    return matches;
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
    this.readyState = 1;
    FakeEventSource.instances.push(this);
  }

  addEventListener(name, callback) {
    this.listeners.set(name, callback);
  }

  dispatch(name, data) {
    this.dispatchRaw(name, JSON.stringify(data));
  }

  dispatchRaw(name, data) {
    this.listeners.get(name)?.({ data });
  }
}

function snapshot(revision, widgets = [], overrides = {}) {
  return {
    overlay_id: OVERLAY_ID,
    revision,
    canvas: { width: 1280, height: 720 },
    widgets,
    ...overrides,
  };
}

function widget(widget_id, content = "hello", overrides = {}) {
  return {
    widget_id,
    name: `Widget ${widget_id.slice(0, 4)}`,
    content,
    font_family: "noto-sans",
    position: { x: 12.5, y: 34.25 },
    font_size: 42.5,
    color: { red: 10, green: 20, blue: 30, alpha: 128 },
    alignment: "center",
    ...overrides,
  };
}

function childIds(canvas) {
  return canvas.children.map((child) => child.getAttribute("data-widget-id"));
}

function runScript(initialIds = ["77777777-7777-4777-8777-777777777777"]) {
  FakeEventSource.instances = [];
  const canvas = new FakeElement("main");
  canvas.setAttribute("data-width", "640");
  canvas.setAttribute("data-height", "360");
  canvas.style.width = "640px";
  canvas.style.height = "360px";

  const initialNodes = new Map();
  for (const id of initialIds) {
    const initial = new FakeElement("span");
    initial.className = "chikachika-text";
    initial.setAttribute("data-widget-id", id);
    initial.textContent = "server-rendered text";
    canvas.appendChild(initial);
    initialNodes.set(id, initial);
  }

  const window = {
    location: { pathname: `/overlay/${OVERLAY_ID}/` },
  };
  const document = new FakeDocument(canvas);
  vm.runInNewContext(script, { window, document, EventSource: FakeEventSource });

  assert.equal(FakeEventSource.instances.length, 1);
  const source = FakeEventSource.instances[0];
  assert.equal(source.url, `/overlay/${OVERLAY_ID}/events`);
  assert.deepEqual([...source.listeners.keys()], ["snapshot"]);
  return { canvas, initialNodes, source, window };
}

test("multi_widget_snapshot_reconciliation", () => {
  const { canvas, initialNodes, source } = runScript([BACK_ID]);
  const back = initialNodes.get(BACK_ID);
  const front = widget(FRONT_ID, "front & safe", {
    name: "Duplicate name",
    font_family: "jetbrains-mono",
    position: { x: 0, y: 0 },
    font_size: 16,
    color: { red: 255, green: 0, blue: 128, alpha: 255 },
    alignment: "right",
  });
  const middle = widget(MIDDLE_ID, "middle");
  const backData = widget(BACK_ID, "back");

  // Add three widgets in frontmost-first model order; DOM order is back-to-front.
  source.dispatch("snapshot", snapshot(0, [front, middle, backData]));
  assert.deepEqual(childIds(canvas), [BACK_ID, MIDDLE_ID, FRONT_ID]);
  assert.strictEqual(canvas.children[0], back);
  assert.equal(back.textContent, "back");
  assert.equal(canvas.children[2].textContent, "front & safe");
  assert.equal(canvas.children[2].style.fontFamily, "'JetBrains Mono', monospace");
  assert.equal(canvas.children[2].style.color, "rgba(255, 0, 128, 1)");
  assert.equal(canvas.children[2].style.textAlign, "right");

  // Edit in place and retain identity while allowing duplicate names.
  const middleNode = canvas.children[1];
  const frontNode = canvas.children[2];
  source.dispatch(
    "snapshot",
    snapshot(1, [
      widget(FRONT_ID, "front & safe", { name: "Duplicate name" }),
      widget(MIDDLE_ID, "edited middle", { name: "Duplicate name", font_family: "jetbrains-mono" }),
      backData,
    ]),
  );
  assert.deepEqual(childIds(canvas), [BACK_ID, MIDDLE_ID, FRONT_ID]);
  assert.strictEqual(canvas.children[1], middleNode);
  assert.equal(middleNode.textContent, "edited middle");
  assert.equal(middleNode.getAttribute("data-widget-name"), "Duplicate name");
  assert.equal(middleNode.style.fontFamily, "'JetBrains Mono', monospace");

  // Duplication inserts at model index zero, so the copy is painted last.
  const copy = widget(COPY_ID, "edited middle", {
    name: "Duplicate name",
    font_family: "jetbrains-mono",
    position: { x: 12.5, y: 34.25 },
    font_size: 42.5,
    color: { red: 10, green: 20, blue: 30, alpha: 128 },
    alignment: "center",
  });
  source.dispatch("snapshot", snapshot(2, [copy, front, middle, backData]));
  assert.deepEqual(childIds(canvas), [BACK_ID, MIDDLE_ID, FRONT_ID, COPY_ID]);
  assert.strictEqual(canvas.children[0], back);
  assert.strictEqual(canvas.children[1], middleNode);
  const copyNode = canvas.children[3];
  assert.equal(copyNode.textContent, "edited middle");
  assert.equal(copyNode.getAttribute("data-widget-name"), "Duplicate name");

  // A one-step reorder changes only DOM order and keeps every existing node.
  source.dispatch("snapshot", snapshot(3, [copy, middle, front, backData]));
  assert.deepEqual(childIds(canvas), [BACK_ID, FRONT_ID, MIDDLE_ID, COPY_ID]);
  assert.strictEqual(canvas.children[0], back);
  assert.strictEqual(canvas.children[1], frontNode);
  assert.strictEqual(canvas.children[2], middleNode);
  assert.strictEqual(canvas.children[3], copyNode);

  // Removal drops only the absent node; the remaining identities stay stable.
  source.dispatch("snapshot", snapshot(4, [middle, front, backData]));
  assert.deepEqual(childIds(canvas), [BACK_ID, FRONT_ID, MIDDLE_ID]);
  assert.strictEqual(canvas.children[0], back);
  assert.strictEqual(canvas.children[2], middleNode);
  assert.equal(copyNode.parentNode, null);

  // Empty documents clear all nodes, including the initial server-rendered ones.
  source.dispatch("snapshot", snapshot(5, []));
  assert.deepEqual(childIds(canvas), []);
  assert.equal(canvas.children.length, 0);

  // A later add creates a fresh node after the old node was removed.
  source.dispatch("snapshot", snapshot(6, [widget(NEW_ID, "new")]));
  assert.deepEqual(childIds(canvas), [NEW_ID]);
  assert.equal(canvas.children[0].textContent, "new");
  assert.notStrictEqual(canvas.children[0], back);
});

test("invalid_snapshot_is_atomic", () => {
  const { canvas, source } = runScript();
  const first = widget(FRONT_ID, "before");
  const second = widget(BACK_ID, "second");
  source.dispatch("snapshot", snapshot(10, [first, second]));

  const beforeChildren = [...canvas.children];
  const beforeIds = childIds(canvas);
  const beforeText = beforeChildren.map((child) => child.textContent);
  const beforeStyles = beforeChildren.map((child) => ({ ...child.style }));
  const beforeCanvasStyle = { ...canvas.style };
  const beforeCanvasAttributes = {
    width: canvas.getAttribute("data-width"),
    height: canvas.getAttribute("data-height"),
  };
  const changedFirst = widget(FRONT_ID, "must not be applied", { position: { x: 100, y: 200 } });
  const invalid = widget(BACK_ID, "invalid", { font_family: "comic-sans" });

  const assertUnchanged = () => {
    assert.deepEqual(childIds(canvas), beforeIds);
    assert.deepEqual(canvas.children, beforeChildren);
    assert.deepEqual(beforeChildren.map((child) => child.textContent), beforeText);
    assert.deepEqual(beforeChildren.map((child) => ({ ...child.style })), beforeStyles);
    assert.deepEqual({ ...canvas.style }, beforeCanvasStyle);
    assert.deepEqual(
      {
        width: canvas.getAttribute("data-width"),
        height: canvas.getAttribute("data-height"),
      },
      beforeCanvasAttributes,
    );
  };

  // The invalid later item must prevent the earlier valid item from changing.
  source.dispatch("snapshot", snapshot(11, [changedFirst, invalid]));
  assertUnchanged();

  // The same malformed item before a valid item must also be atomic.
  source.dispatch("snapshot", snapshot(12, [invalid, changedFirst]));
  assertUnchanged();

  // Duplicate IDs are not a valid ordered collection.
  source.dispatch("snapshot", snapshot(13, [changedFirst, { ...changedFirst }]));
  assertUnchanged();

  // Validate all required fields and model bounds, not merely their types.
  source.dispatch("snapshot", snapshot(14, [widget(FRONT_ID, "bad", { name: "   " })]));
  source.dispatch("snapshot", snapshot(15, [widget(FRONT_ID, "bad", { position: { x: -1, y: 0 } })]));
  source.dispatch("snapshot", snapshot(16, [widget(FRONT_ID, "bad", { position: { x: 1280.1, y: 0 } })]));
  source.dispatch("snapshot", snapshot(17, [widget(FRONT_ID, "bad", { font_size: 0 })]));
  source.dispatch("snapshot", snapshot(18, [widget(FRONT_ID, "bad", { color: { red: 256, green: 0, blue: 0, alpha: 0 } })]));
  source.dispatch("snapshot", snapshot(19, [widget(FRONT_ID, "bad", { alignment: "justify" })]));
  source.dispatch("snapshot", snapshot(20, [widget("not-a-uuid", "bad")]));
  source.dispatch("snapshot", snapshot(21, [widget(FRONT_ID, "bad", { content: 42 })]));
  source.dispatch("snapshot", snapshot(22, [widget(FRONT_ID, "bad", { position: { x: NaN, y: 0 } })]));
  source.dispatch("snapshot", snapshot(23, [widget(FRONT_ID, "bad", { name: "valid" })], { canvas: { width: 0, height: 720 } }));
  source.dispatch("snapshot", snapshot(24, [widget(FRONT_ID, "bad", { name: "valid" })], { canvas: { width: 1280, height: 720.5 } }));
  source.dispatch("snapshot", { ...snapshot(25, [changedFirst]), overlay_id: "not-a-uuid" });
  source.dispatch("snapshot", { ...snapshot(26, [changedFirst]), revision: Number.MAX_SAFE_INTEGER + 1 });
  assertUnchanged();

  // Since all malformed revisions were rejected, the next valid revision applies.
  source.dispatch("snapshot", snapshot(11, [changedFirst, second]));
  assert.equal(canvas.children[1].textContent, "must not be applied");
  assert.equal(canvas.children[1].style.left, "100px");
});

test("stale_snapshot_preserves_dom", () => {
  const { canvas, source } = runScript();
  const front = widget(FRONT_ID, "current");
  const back = widget(BACK_ID, "back");
  source.dispatch("snapshot", snapshot(2, [front, back]));

  const beforeChildren = [...canvas.children];
  const beforeIds = childIds(canvas);
  const beforeContent = beforeChildren.map((child) => child.textContent);
  const beforeStyles = beforeChildren.map((child) => ({ ...child.style }));

  source.dispatch("snapshot", snapshot(1, [widget(FRONT_ID, "older"), widget(BACK_ID, "older back")]));
  source.dispatch("snapshot", snapshot(2, [widget(FRONT_ID, "duplicate revision")]));
  assert.deepEqual(childIds(canvas), beforeIds);
  assert.deepEqual(canvas.children, beforeChildren);
  assert.deepEqual(beforeChildren.map((child) => child.textContent), beforeContent);
  assert.deepEqual(beforeChildren.map((child) => ({ ...child.style })), beforeStyles);

  // MAX_SAFE_INTEGER is accepted, but the next integer is not representable as
  // a safe monotonic revision and must not mutate the already-current DOM.
  source.dispatch("snapshot", snapshot(Number.MAX_SAFE_INTEGER, [front, back]));
  assert.equal(canvas.children[1].textContent, "current");
  const maxChildren = [...canvas.children];
  source.dispatch(
    "snapshot",
    snapshot(Number.MAX_SAFE_INTEGER + 1, [widget(FRONT_ID, "overflow"), widget(BACK_ID, "overflow back")]),
  );
  assert.deepEqual(canvas.children, maxChildren);
  assert.deepEqual(childIds(canvas), beforeIds);
  assert.deepEqual(canvas.children.map((child) => child.textContent), beforeContent);
});

test("overlay script uses safe DOM APIs and keeps SSE lifecycle native", () => {
  assert.equal(script.includes("innerHTML"), false);
  assert.equal(script.includes("insertAdjacentHTML"), false);
  assert.equal(script.includes("setInterval"), false);
  assert.equal(script.includes("setTimeout"), false);
  assert.equal(script.includes("textElement"), false);

  const { source, window } = runScript([]);
  assert.equal(typeof source.listeners.get("snapshot"), "function");
  assert.equal(window.ChikachikaOverlay.canvas.tagName, "main");
  source.dispatchRaw("snapshot", "not JSON");
});
