(function () {
  "use strict";

  function getInput() {
    return (
      document.getElementById("text") ||
      document.getElementById("note-input") ||
      document.getElementById("note") ||
      document.querySelector('input[type="text"]') ||
      document.querySelector("textarea") ||
      document.querySelector("input")
    );
  }

  function getErrorEl() {
    var el = document.getElementById("error");
    if (!el) {
      el = document.createElement("div");
      el.id = "error";
      el.setAttribute("role", "alert");
      el.hidden = true;
      document.body.appendChild(el);
    }
    return el;
  }

  function showError(message) {
    var el = getErrorEl();
    el.textContent = message || "Unbekannter Fehler";
    el.hidden = false;
  }

  function clearError() {
    var el = document.getElementById("error");
    if (el) {
      el.textContent = "";
      el.hidden = true;
    }
  }

  function core() {
    if (!window.__TAURI__ || !window.__TAURI__.core) {
      throw new Error("Tauri-API nicht verfügbar");
    }
    return window.__TAURI__.core;
  }

  async function invokeParkNote(text) {
    return core().invoke("park_note", { text: text });
  }

  async function closeWindow() {
    if (!window.__TAURI__ || !window.__TAURI__.window) {
      throw new Error("Tauri-API nicht verfügbar");
    }
    await window.__TAURI__.window.getCurrentWindow().close();
  }

  async function onEnter(event) {
    if (event.key !== "Enter") {
      return;
    }
    event.preventDefault();

    var input = event.target;
    var text = input.value.trim();
    if (!text) {
      return;
    }

    input.disabled = true;
    clearError();
    try {
      await invokeParkNote(text);
      await closeWindow();
    } catch (err) {
      input.disabled = false;
      input.value = "";
      showError(String(err));
      input.focus();
    }
  }

  function init() {
    var input = getInput();
    if (!input) {
      return;
    }
    input.addEventListener("keydown", onEnter);
    input.focus();
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
