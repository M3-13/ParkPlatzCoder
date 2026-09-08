"use strict";

const { test } = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const source = fs.readFileSync(path.join(__dirname, "input.js"), "utf8");

function makeInput() {
  return {
    value: "",
    disabled: false,
    focused: false,
    listeners: {},
    addEventListener(type, fn) {
      this.listeners[type] = fn;
    },
    dispatch(type, event) {
      const fn = this.listeners[type];
      if (!fn) return undefined;
      event = event || {};
      if (event.target === undefined) event.target = this;
      return fn(event);
    },
    focus() {
      this.focused = true;
    },
  };
}

function setup(invokeImpl) {
  const input = makeInput();
  let createdError = null;

  const document = {
    readyState: "complete",
    addEventListener() {},
    getElementById(id) {
      if (id === "note-input") return input;
      if (id === "error") return createdError;
      return null;
    },
    createElement(tag) {
      return {
        tagName: tag,
        id: "",
        hidden: false,
        textContent: "",
        setAttribute() {},
      };
    },
    body: {
      appendChild(el) {
        if (el.id === "error") createdError = el;
      },
    },
  };

  const invokeCalls = [];
  const closeCalls = [];

  const window = {
    __TAURI__: {
      core: {
        invoke: async (cmd, args) => {
          invokeCalls.push({ cmd, args });
          return invokeImpl ? invokeImpl(cmd, args) : {};
        },
      },
      window: {
        getCurrentWindow: () => ({
          close: async () => {
            closeCalls.push(true);
          },
        }),
      },
    },
  };

  new Function("window", "document", source)(window, document);

  return { document, window, input, invokeCalls, closeCalls, getError: () => createdError };
}

test("Enter invokes park_note with the trimmed text and closes the window on success", async () => {
  const ctx = setup(async (cmd, args) => {
    assert.equal(cmd, "park_note");
    assert.equal(typeof args.text, "string");
    return { id: 1 };
  });

  ctx.input.value = "  erledigt: bug behoben  ";
  await ctx.input.dispatch("keydown", { key: "Enter", preventDefault() {} });

  assert.equal(ctx.invokeCalls.length, 1);
  assert.equal(ctx.invokeCalls[0].cmd, "park_note");
  assert.equal(ctx.invokeCalls[0].args.text, "erledigt: bug behoben");
  assert.equal(ctx.closeCalls.length, 1);
  assert.equal(ctx.getError(), null);
});

test("Enter clears the field and shows the error message when park_note fails", async () => {
  const ctx = setup(async () => {
    throw new Error("no git repository detected");
  });

  ctx.input.value = "hello";
  await ctx.input.dispatch("keydown", { key: "Enter", preventDefault() {} });

  assert.equal(ctx.input.value, "");
  assert.equal(ctx.input.disabled, false);
  assert.equal(ctx.closeCalls.length, 0);

  const err = ctx.getError();
  assert.ok(err);
  assert.equal(err.hidden, false);
  assert.match(err.textContent, /no git repository detected/);
});

test("Enter with empty or whitespace-only text does nothing", async () => {
  const ctx = setup(async () => ({ id: 1 }));

  ctx.input.value = "   ";
  await ctx.input.dispatch("keydown", { key: "Enter", preventDefault() {} });

  assert.equal(ctx.invokeCalls.length, 0);
  assert.equal(ctx.closeCalls.length, 0);
});
