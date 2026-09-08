// ui/input.js
// Verdrahtet den Schnell-Park-Eingabevorgang:
//   - Fokus auf das Eingabefeld beim Öffnen des Fensters
//   - Enter ruft invoke('park_note', { text }) auf
//   - Erfolg: Fenster schließen
//   - Fehler: Fehlermeldung als reinen Text anzeigen und Feld leeren
// Der Text wird als reiner String übergeben, nie als HTML.
//
// Läuft im Browser als plain <script> (hängt window.ParkInput an) ebenso wie
// als ES-Modul; in Node (CommonJS require) für die Tests ohne Tauri- und
// DOM-Abhängigkeit beim Laden.

(function (root, factory) {
  if (typeof module === "object" && module.exports) {
    module.exports = factory();
  } else {
    root.ParkInput = factory();
  }
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  // Eingabefeld: bekannter ID folgen, sonst auf das erste plausible Feld fallen.
  const INPUT_SELECTOR = [
    "#park-input",
    "#note-input",
    'input[type="text"]',
    "textarea",
    'input:not([type="submit"]):not([type="button"]):not([type="hidden"])',
    "input",
  ].join(", ");

  const ERROR_SELECTOR = "#park-error, #error";

  const api = {};

  function getTauri() {
    if (typeof window !== "undefined" && window.__TAURI__) {
      return window.__TAURI__;
    }
    return undefined;
  }

  function getInputElement() {
    if (typeof document === "undefined") return null;
    return document.querySelector(INPUT_SELECTOR);
  }

  function getErrorElement() {
    if (typeof document === "undefined") return null;
    const existing = document.querySelector(ERROR_SELECTOR);
    if (existing) return existing;
    if (typeof document.createElement !== "function") return null;
    const created = document.createElement("div");
    created.id = "park-error";
    created.setAttribute("role", "alert");
    const parent = document.body || document.documentElement;
    if (parent) parent.appendChild(created);
    return created;
  }

  api.invokeParkNote = function (text) {
    const tauri = getTauri();
    if (!tauri || !tauri.core || typeof tauri.core.invoke !== "function") {
      return Promise.reject(new Error("Tauri runtime ist nicht verfügbar"));
    }
    return tauri.core.invoke("park_note", { text });
  };

  api.closeWindow = function () {
    const tauri = getTauri();
    if (
      tauri &&
      tauri.window &&
      typeof tauri.window.getCurrentWindow === "function"
    ) {
      return tauri.window.getCurrentWindow().close();
    }
    return Promise.resolve();
  };

  api.focusInput = function () {
    const el = getInputElement();
    if (el && typeof el.focus === "function") {
      el.focus();
    }
    return el;
  };

  api.clearField = function () {
    const el = getInputElement();
    if (el) el.value = "";
  };

  api.showError = function (message) {
    const el = getErrorElement();
    if (el) el.textContent = String(message);
  };

  api.clearError = function () {
    const el = getErrorElement();
    if (el) el.textContent = "";
  };

  api.errorMessage = function (err) {
    if (typeof err === "string") return err;
    if (err && typeof err.message === "string") return err.message;
    return String(err);
  };

  api.handleSubmit = function (text) {
    const value = typeof text === "string" ? text.trim() : "";
    if (!value) {
      return Promise.resolve({ ok: false, empty: true });
    }
    return api
      .invokeParkNote(value)
      .then(function () {
        return api.closeWindow().then(function () {
          return { ok: true };
        });
      })
      .catch(function (err) {
        const message = api.errorMessage(err);
        api.showError(message);
        api.clearField();
        return { ok: false, error: message };
      });
  };

  function onEnter(event) {
    if (!event) return;
    if (event.key !== "Enter" && event.keyCode !== 13) return;
    if (event.isComposing) return;
    if (typeof event.preventDefault === "function") {
      event.preventDefault();
    }
    const el = getInputElement();
    api.handleSubmit(el ? el.value : "");
  }

  function wire() {
    const el = getInputElement();
    if (!el) return;
    if (typeof el.addEventListener === "function") {
      el.addEventListener("keydown", onEnter);
    }
    const form = typeof el.closest === "function" ? el.closest("form") : null;
    if (form && typeof form.addEventListener === "function") {
      form.addEventListener("submit", function (event) {
        if (event && typeof event.preventDefault === "function") {
          event.preventDefault();
        }
        api.handleSubmit(el.value);
      });
    }
  }

  function onReady() {
    api.focusInput();
    wire();
  }

  api.init = function () {
    if (typeof document === "undefined") return;
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", onReady);
    } else {
      onReady();
    }
  };

  api.init();

  return api;
});
