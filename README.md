# ParkPlatzCoder

Ein lokal laufendes Rust/Tauri-Tray-Tool für Einzelentwickler: Per globalem
Hotkey parkt man in unter 5 Sekunden eine Freitext-Zeile zusammen mit
automatisch erfasstem Arbeitskontext (Repo-Pfad, Branch, letzter Commit,
geänderte Dateien, Zeitstempel). Beim Wechsel auf einen Branch mit vorhandenem
Zettel erscheint eine unaufdringliche Benachrichtigung; eine durchsuchbare,
chronologische Liste gruppiert nach Repo zeigt alle Zettel mit Abhaken und
Löschen. Alle Daten liegen lokal in SQLite — kein Konto, kein Server, kein
Netzwerk.

## Tech-Stack

- **Sprache:** Rust
- **GUI:** Tauri v2
- **Speicherung:** SQLite (`rusqlite`, folgt in einem späteren Ticket)
- **Git:** libgit2 (folgt in einem späteren Ticket)
- **Watcher:** notify (folgt in einem späteren Ticket)
- **Plattform:** Linux zuerst, macOS/Windows später

## Installation

Voraussetzungen: eine Rust-Toolchain (siehe <https://rustup.rs>) sowie die
Tauri-Systemabhängigkeiten für die Zielplattform
(siehe <https://tauri.app/start/prerequisites/>).

```bash
git clone <repo-url> && cd parkplatzcoder
cargo build
```

## Ausführen (Entwicklung)

```bash
cargo run
```

Die App startet im Hintergrund und zeigt ein Tray-Icon.

## Produktion bauen

```bash
cargo build --release
# gebündelte Installer/Artefakte:
cargo tauri build
```

## Bedienung

- **Globaler Hotkey `Ctrl+Alt+P`** öffnet das Eingabefenster mit einem
  Eingabefeld.
- Das **Tray-Icon** (rechts klickbar) bietet:
  - **Eingabe öffnen** — öffnet das Eingabefenster
  - **Liste öffnen** — öffnet die Zettelliste
  - **Exportieren** — exportiert alle Zettel als JSON
  - **Beenden** — beendet die App

## Datenpfad

Alle Daten liegen lokal unter `dirs::data_dir()/parkplatzcoder/`:

- `notes.db` — SQLite-Datenbank (Berechtigung `0600`)
- `export-<UTC-Zeitstempel>.json` — JSON-Exporte (Berechtigung `0600`)

## Features

- Tray-Icon mit Menü (Eingabe öffnen, Liste öffnen, Exportieren, Beenden)
- Globaler Hotkey `Ctrl+Alt+P` zum Öffnen des Eingabefensters
- Eingabefenster (`ui/input.html`) und Zettelliste (`ui/list.html`)
- Kommandos: `park_note`, `get_notes`, `search_notes`, `toggle_done`,
  `remove_note`, `export_json`
- Modul-Stubs für Storage, Git-Kontext, Watcher, Notifier und Export
- Datenschutz: Logs enthalten keine Zettelinhalte, Repo-Pfade, Branch-Namen
  oder Commit-Hashes
