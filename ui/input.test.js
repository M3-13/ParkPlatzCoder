// ui/input.test.js
// Dependency-freie Tests für ui/input.js (node --test ui/input.test.js).
// Kein Tauri, kein jsdom: die invoke-/close-Funktionen werden auf dem
// exportierten Objekt überschrieben und ein minimales Fake-DOM installiert.
"use strict";

const { test, afterEach } = require("node:test");
const assert = require("node:assert");

const ParkInput = require("./input.js");

function makeFakeElement(props = {}) {
  const el = {
    value: "",
    textContent: "",
    focused: false,
    listeners: {},
    tagName: "INPUT",
  };
  Object.assign(el, props);
  el.focus = function () {
    this.focused = true;
  };
  el.addEventListener = function (type, fn) {
    this.listeners[type] = fn;
  };
  el.closest = function () {
    return null;
  };
  return el;
}

function installFakeDom({ withInput = true } = {}) {
  const input = makeFakeElement({ id: "park-input" });
  const errorEl = makeFakeElement({ id: "park-error", tagName: "DIV" });
  global.document = {
    readyState: "complete",
    addEventListener() {},
    createElement(tag) {
      return makeFakeElement({ tagName: (tag || "div").toUpperCase() });
    },
    body: makeFakeElement({ tagName: "BODY" }),
    querySelector(selector) {
      if (typeof selector === "string" && selector.indexOf("error") !== -1) {
        return errorEl;
      }
      return withInput ? input : null;
    },
  };
  return { input, errorEl };
}

afterEach(() => {
  delete global.document;
  delete global.window;
});

test("success: invokes park_note with trimmed text and closes the window", async () => {
  let invokedWith = null;
  let closed = false;

  ParkInput.invokeParkNote = async (text) => {
    invokedWith = text;
  };
  ParkInput.closeWindow = async () => {
    closed = true;
  };

  const result = await ParkInput.handleSubmit("  mein zettel  ");

  assert.strictEqual(invokedWith, "mein zettel");
  assert.strictEqual(closed, true);
  assert.deepStrictEqual(result, { ok: true });
});

test("error: shows the message as plain text and clears the field", async () => {
  const { input, errorEl } = installFakeDom();
  input.value = "etwas text";

  ParkInput.invokeParkNote = async () => {
    throw "Speichern fehlgeschlagen";
  };
  ParkInput.closeWindow = async () => {};

  const result = await ParkInput.handleSubmit(input.value);

  assert.deepStrictEqual(result, {
    ok: false,
    error: "Speichern fehlgeschlagen",
  });
  assert.strictEqual(errorEl.textContent, "Speichern fehlgeschlagen");
  assert.strictEqual(input.value, "");
});

test("error object: extracts the message", async () => {
  const { errorEl } = installFakeDom();
  ParkInput.invokeParkNote = async () => {
    throw new Error("boom");
  };
  ParkInput.closeWindow = async () => {};

  const result = await ParkInput.handleSubmit("x");

  assert.strictEqual(result.error, "boom");
  assert.strictEqual(errorEl.textContent, "boom");
});

test("empty input: never invokes park_note", async () => {
  let called = false;
  ParkInput.invokeParkNote = async () => {
    called = true;
  };
  ParkInput.closeWindow = async () => {};

  const result = await ParkInput.handleSubmit("   ");

  assert.strictEqual(called, false);
  assert.deepStrictEqual(result, { ok: false, empty: true });
});

test("showError uses textContent, never innerHTML (no HTML injection)", () => {
  const { errorEl } = installFakeDom();
  const payload = '<img src=x onerror="alert(1)">';

  ParkInput.showError(payload);

  assert.strictEqual(errorEl.textContent, payload);
  assert.strictEqual(errorEl.innerHTML, undefined);
});

test("focusInput focuses the field", () => {
  const { input } = installFakeDom();

  const el = ParkInput.focusInput();

  assert.strictEqual(el, input);
  assert.strictEqual(input.focused, true);
});

test("Enter key on the field submits the value", async () => {
  const { input } = installFakeDom();
  input.value = "per hotkey";

  let invokedWith = null;
  ParkInput.invokeParkNote = async (text) => {
    invokedWith = text;
  };
  ParkInput.closeWindow = async () => {};

  ParkInput.init();
  const onKeydown = input.listeners.keydown;
  assert.strictEqual(typeof onKeydown, "function");

  onKeydown({
    key: "Enter",
    preventDefault() {
      this.defaultPrevented = true;
    },
  });

  await new Promise((resolve) => setImmediate(resolve));

  assert.strictEqual(invokedWith, "per hotkey");
});
