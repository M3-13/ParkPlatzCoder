use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

const LIST_JS: &str = include_str!("../../ui/list.js");

#[test]
fn list_js_renders_user_text_without_inner_html() {
    // AC-10: user text is rendered via textContent, never innerHTML.
    assert!(
        !LIST_JS.contains("innerHTML"),
        "ui/list.js must not use innerHTML to render user data"
    );
}

#[test]
fn list_js_is_wired_to_the_sprint_commands() {
    assert!(
        LIST_JS.contains("invoke(\"get_notes\")"),
        "ui/list.js must call invoke(\"get_notes\")"
    );
    assert!(
        LIST_JS.contains("invoke(\"search_notes\", { query })"),
        "ui/list.js must call invoke(\"search_notes\", {{ query }})"
    );
    assert!(
        LIST_JS.contains("invoke(\"toggle_done\", { id })"),
        "ui/list.js must call invoke(\"toggle_done\", {{ id }})"
    );
    assert!(
        LIST_JS.contains("invoke(\"remove_note\", { id })"),
        "ui/list.js must call invoke(\"remove_note\", {{ id }})"
    );
}

#[test]
fn list_js_pure_logic_passes_node_test() {
    let test_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/list.test.js");
    let status = match Command::new("node").arg(&test_file).status() {
        Ok(status) => status,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            eprintln!("node is not available; skipping the functional JS test");
            return;
        }
        Err(err) => panic!("failed to spawn node: {}", err),
    };
    assert!(
        status.success(),
        "ui/list.test.js exited with {:?}",
        status.code()
    );
}
