"use strict";

function formatTimestamp(createdAt) {
  if (!createdAt) {
    return "";
  }
  const date = new Date(createdAt);
  if (Number.isNaN(date.getTime())) {
    return createdAt;
  }
  return date.toLocaleString();
}

function sortNotesDescending(notes) {
  return notes.slice().sort((a, b) => {
    const aTime = a.created_at || "";
    const bTime = b.created_at || "";
    if (aTime < bTime) return 1;
    if (aTime > bTime) return -1;
    return 0;
  });
}

function groupByRepo(notes) {
  const groups = new Map();
  for (const note of notes) {
    const key = note.repo_path || "";
    if (!groups.has(key)) {
      groups.set(key, []);
    }
    groups.get(key).push(note);
  }
  const entries = Array.from(groups.entries());
  entries.sort((a, b) => {
    const aNewest = a[1][0] ? a[1][0].created_at || "" : "";
    const bNewest = b[1][0] ? b[1][0].created_at || "" : "";
    if (aNewest < bNewest) return 1;
    if (aNewest > bNewest) return -1;
    return 0;
  });
  return entries;
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = { formatTimestamp, sortNotesDescending, groupByRepo };
} else {
  (function () {
    const tauriGlobal = window.__TAURI__;
    const invoke = tauriGlobal && tauriGlobal.core ? tauriGlobal.core.invoke : null;

    const searchInput = document.getElementById("search-input");
    const listRoot = document.getElementById("note-list");

    let searchTimer = null;
    const SEARCH_DEBOUNCE_MS = 250;

    function el(tag, className, text) {
      const node = document.createElement(tag);
      if (className) {
        node.className = className;
      }
      if (text !== undefined && text !== null) {
        node.textContent = String(text);
      }
      return node;
    }

    function clearChildren(root) {
      while (root.firstChild) {
        root.removeChild(root.firstChild);
      }
    }

    function renderEmpty() {
      clearChildren(listRoot);
      listRoot.appendChild(el("p", "empty-state", "Keine Zettel vorhanden."));
    }

    function renderError(message) {
      clearChildren(listRoot);
      const box = el("div", "error-state");
      box.appendChild(
        el("p", "error-title", "Liste konnte nicht geladen werden.")
      );
      if (message) {
        box.appendChild(el("p", "error-detail", message));
      }
      listRoot.appendChild(box);
    }

    function buildChangedFiles(note) {
      const files = Array.isArray(note.changed_files) ? note.changed_files : [];
      const container = el("div", "meta-block");
      const label = el("span", "meta-label", "Geänderte Dateien");
      container.appendChild(label);

      if (files.length === 0) {
        container.appendChild(el("span", "meta-value meta-muted", "—"));
        return container;
      }

      const list = el("ul", "file-list");
      for (const file of files) {
        list.appendChild(el("li", "file-item", file));
      }
      container.appendChild(list);
      return container;
    }

    function buildMetaRow(labelText, valueText, valueClassName) {
      const row = el("div", "meta-row");
      row.appendChild(el("span", "meta-label", labelText));
      const value = el("span", valueClassName || "meta-value", valueText);
      row.appendChild(value);
      return row;
    }

    function buildNoteCard(note) {
      const card = el("article", note.done ? "note note-done" : "note");

      const top = el("div", "note-top");

      const toggleBtn = el(
        "button",
        note.done ? "toggle-btn toggle-btn-done" : "toggle-btn",
        note.done ? "✓ Erledigt" : "Abhaken"
      );
      toggleBtn.type = "button";
      toggleBtn.title = "Als erledigt markieren";
      toggleBtn.disabled = Boolean(note.done);
      toggleBtn.addEventListener("click", () => toggleDone(note.id));
      top.appendChild(toggleBtn);

      top.appendChild(el("p", "note-text", note.text));

      const deleteBtn = el("button", "delete-btn", "Löschen");
      deleteBtn.type = "button";
      deleteBtn.title = "Zettel dauerhaft löschen";
      deleteBtn.addEventListener("click", () => removeNote(note.id));
      top.appendChild(deleteBtn);

      card.appendChild(top);

      const meta = el("div", "note-meta");
      meta.appendChild(buildMetaRow("Repo", note.repo_path || "—"));
      meta.appendChild(buildMetaRow("Branch", note.branch || "—"));
      meta.appendChild(
        buildMetaRow("Commit", note.commit_hash || "—", "meta-value meta-mono")
      );
      meta.appendChild(
        buildMetaRow("Zeitstempel", formatTimestamp(note.created_at) || "—")
      );
      meta.appendChild(buildChangedFiles(note));

      card.appendChild(meta);
      return card;
    }

    function render(notes) {
      if (!Array.isArray(notes) || notes.length === 0) {
        renderEmpty();
        return;
      }

      clearChildren(listRoot);

      const sorted = sortNotesDescending(notes);
      const groups = groupByRepo(sorted);

      for (const [repo, groupNotes] of groups) {
        const section = el("section", "repo-group");
        const heading = el("h2", "repo-heading", repo || "Unbekanntes Repo");
        section.appendChild(heading);

        const cards = el("div", "note-cards");
        for (const note of groupNotes) {
          cards.appendChild(buildNoteCard(note));
        }
        section.appendChild(cards);
        listRoot.appendChild(section);
      }
    }

    function currentQuery() {
      return searchInput.value.trim();
    }

    async function loadNotes() {
      if (!invoke) {
        renderError("Backend nicht verfügbar (Tauri nicht verbunden).");
        return;
      }
      const query = currentQuery();
      try {
        const notes = query
          ? await invoke("search_notes", { query })
          : await invoke("get_notes");
        render(notes);
      } catch (err) {
        renderError(String(err));
      }
    }

    async function toggleDone(id) {
      if (!invoke) {
        return;
      }
      try {
        await invoke("toggle_done", { id });
      } catch (err) {
        renderError(String(err));
        return;
      }
      await loadNotes();
    }

    async function removeNote(id) {
      if (!invoke) {
        return;
      }
      try {
        await invoke("remove_note", { id });
      } catch (err) {
        renderError(String(err));
        return;
      }
      await loadNotes();
    }

    searchInput.addEventListener("input", () => {
      if (searchTimer !== null) {
        clearTimeout(searchTimer);
      }
      searchTimer = setTimeout(loadNotes, SEARCH_DEBOUNCE_MS);
    });

    loadNotes();
  })();
}
