"use strict";

(function () {
  // Resolve the Tauri `invoke` function. The window is configured with
  // `withGlobalTauri`, so the API is exposed on `window.__TAURI__`.
  function getInvoke() {
    if (
      typeof window !== "undefined" &&
      window.__TAURI__ &&
      window.__TAURI__.core &&
      typeof window.__TAURI__.core.invoke === "function"
    ) {
      return window.__TAURI__.core.invoke;
    }
    return null;
  }

  // Build a DOM element and set its text content as PURE TEXT. User-supplied
  // strings are never parsed as HTML or JavaScript (AC-10).
  function el(tag, className, text) {
    var node = document.createElement(tag);
    if (className) {
      node.className = className;
    }
    if (text !== undefined && text !== null) {
      node.textContent = String(text);
    }
    return node;
  }

  function errMsg(e) {
    if (e === null || e === undefined) {
      return "Unbekannter Fehler";
    }
    if (typeof e === "string") {
      return e;
    }
    if (e && e.message) {
      return e.message;
    }
    return String(e);
  }

  // Sort notes by `created_at` descending (newest first). The timestamp is
  // RFC3339 UTC, so a lexical comparison is also a chronological one; `id` is
  // used as a stable tie-breaker.
  function sortNotesDescending(notes) {
    return notes.slice().sort(function (a, b) {
      var at = a.created_at || "";
      var bt = b.created_at || "";
      if (at === bt) {
        return (b.id || 0) - (a.id || 0);
      }
      return at < bt ? 1 : -1;
    });
  }

  // Group an already-sorted list of notes by repo path, preserving the order
  // in which each repo first appears.
  function groupByRepo(notes) {
    var groups = [];
    var index = Object.create(null);
    notes.forEach(function (note) {
      var repo = note.repo_path || "";
      var entry = index[repo];
      if (entry === undefined) {
        entry = { repo: repo, notes: [] };
        index[repo] = entry;
        groups.push(entry);
      }
      entry.notes.push(note);
    });
    return groups;
  }

  function formatTimestamp(createdAt) {
    if (!createdAt) {
      return "";
    }
    try {
      var d = new Date(createdAt);
      if (isNaN(d.getTime())) {
        return createdAt;
      }
      return d.toLocaleString();
    } catch (e) {
      return createdAt;
    }
  }

  // --- Rendering ----------------------------------------------------------

  var listContainer = null;
  var searchInput = null;
  var searchTimer = null;

  function renderError(message) {
    if (!listContainer) {
      return;
    }
    listContainer.textContent = "";
    listContainer.appendChild(el("p", "empty-state", message));
  }

  function renderNote(note) {
    var card = el("article", "note-card");
    if (note.done) {
      card.classList.add("note-done");
    }

    card.appendChild(el("p", "note-text", note.text));

    var meta = el("div", "note-meta");
    meta.appendChild(el("span", "note-repo", note.repo_path));
    meta.appendChild(el("span", "note-branch", note.branch));
    meta.appendChild(el("span", "note-commit", note.commit_hash));
    meta.appendChild(el("span", "note-time", formatTimestamp(note.created_at)));
    card.appendChild(meta);

    var files = note.changed_files || [];
    if (files.length > 0) {
      var filesList = el("ul", "note-files");
      files.forEach(function (file) {
        filesList.appendChild(el("li", "note-file", file));
      });
      card.appendChild(filesList);
    }

    var actions = el("div", "note-actions");
    var toggleBtn = el(
      "button",
      "note-btn note-toggle",
      note.done ? "Erledigt" : "Abhaken"
    );
    var deleteBtn = el("button", "note-btn note-delete", "Löschen");

    toggleBtn.addEventListener("click", function () {
      onToggleDone(note.id);
    });
    deleteBtn.addEventListener("click", function () {
      onRemoveNote(note.id);
    });

    actions.appendChild(toggleBtn);
    actions.appendChild(deleteBtn);
    card.appendChild(actions);

    return card;
  }

  function renderGroup(group) {
    var section = el("section", "repo-group");
    section.appendChild(el("h2", "repo-heading", group.repo));
    group.notes.forEach(function (note) {
      section.appendChild(renderNote(note));
    });
    return section;
  }

  function renderNotes(notes) {
    if (!listContainer) {
      return;
    }
    listContainer.textContent = "";
    if (!notes || notes.length === 0) {
      listContainer.appendChild(el("p", "empty-state", "Keine Zettel"));
      return;
    }
    var groups = groupByRepo(sortNotesDescending(notes));
    groups.forEach(function (group) {
      listContainer.appendChild(renderGroup(group));
    });
  }

  async function loadNotes() {
    var invoke = getInvoke();
    if (!invoke) {
      renderError("Backend nicht verfügbar");
      return;
    }
    var query = searchInput ? searchInput.value.trim() : "";
    try {
      var notes;
      if (query) {
        notes = await invoke("search_notes", { query: query });
      } else {
        notes = await invoke("get_notes");
      }
      renderNotes(notes);
    } catch (e) {
      renderError("Laden fehlgeschlagen: " + errMsg(e));
    }
  }

  async function onToggleDone(id) {
    var invoke = getInvoke();
    if (!invoke) {
      return;
    }
    try {
      await invoke("toggle_done", { id: id });
      await loadNotes();
    } catch (e) {
      renderError("Aktion fehlgeschlagen: " + errMsg(e));
    }
  }

  async function onRemoveNote(id) {
    var invoke = getInvoke();
    if (!invoke) {
      return;
    }
    try {
      await invoke("remove_note", { id: id });
      await loadNotes();
    } catch (e) {
      renderError("Aktion fehlgeschlagen: " + errMsg(e));
    }
  }

  function onSearchInput() {
    if (searchTimer) {
      clearTimeout(searchTimer);
    }
    searchTimer = setTimeout(loadNotes, 250);
  }

  function init() {
    listContainer = document.getElementById("note-list");
    searchInput = document.getElementById("search-input");
    if (!listContainer || !searchInput) {
      return;
    }
    searchInput.addEventListener("input", onSearchInput);
    loadNotes();
  }

  if (typeof module !== "undefined" && module.exports) {
    module.exports = {
      sortNotesDescending: sortNotesDescending,
      groupByRepo: groupByRepo,
    };
  }

  if (typeof document !== "undefined") {
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", init);
    } else {
      init();
    }
  }
})();
