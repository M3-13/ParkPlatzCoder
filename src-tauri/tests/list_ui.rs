use std::path::PathBuf;
use std::process::Command;

fn list_js_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/list.js")
}

fn read_list_js() -> String {
    std::fs::read_to_string(list_js_path()).expect("ui/list.js must exist")
}

/// AC-10: user-supplied note text must be rendered as pure text, never parsed
/// as HTML. The list script must not contain `innerHTML`.
#[test]
fn list_js_never_uses_inner_html() {
    let src = read_list_js();
    assert!(
        !src.contains("innerHTML"),
        "ui/list.js must not use innerHTML (AC-10)"
    );
}

/// The list script must render text via `textContent`.
#[test]
fn list_js_renders_text_via_text_content() {
    let src = read_list_js();
    assert!(
        src.contains("textContent"),
        "ui/list.js must set user text via textContent (AC-10)"
    );
}

/// The list script must call exactly the sprint commands it is promised.
#[test]
fn list_js_wires_the_sprint_commands() {
    let src = read_list_js();
    for cmd in ["get_notes", "search_notes", "toggle_done", "remove_note"] {
        assert!(
            src.contains(cmd),
            "ui/list.js must invoke the `{}` command",
            cmd
        );
    }
}

/// The pure helpers are tested by a Node script. If `node` is not installed,
/// skip silently rather than failing the suite.
#[test]
fn list_js_node_test_passes() {
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui/list.test.js");
    match Command::new("node").arg(&script).status() {
        Ok(status) => assert!(status.success(), "node ui/list.test.js must pass"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Node is not available; the helper tests are skipped.
        }
        Err(e) => panic!("failed to run node ui/list.test.js: {}", e),
    }
}
