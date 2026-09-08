"use strict";

var assert = require("assert");
var ui = require("./list.js");

function note(id, repo, created_at, done) {
  return {
    id: id,
    text: "Text " + id,
    repo_path: repo,
    branch: "main",
    commit_hash: "deadbeef" + id,
    changed_files: ["src/a" + id + ".rs"],
    created_at: created_at,
    done: !!done,
  };
}

// sortNotesDescending orders newest-first.
(function () {
  var older = note(1, "/a", "2026-09-07T10:00:00Z");
  var newest = note(2, "/a", "2026-09-09T10:00:00Z");
  var middle = note(3, "/a", "2026-09-08T10:00:00Z");
  var sorted = ui.sortNotesDescending([older, newest, middle]);
  assert.strictEqual(sorted.length, 3);
  assert.strictEqual(sorted[0].id, 2);
  assert.strictEqual(sorted[1].id, 3);
  assert.strictEqual(sorted[2].id, 1);
})();

// sortNotesDescending is stable for equal timestamps (id tie-breaker).
(function () {
  var a = note(10, "/a", "2026-09-08T10:00:00Z");
  var b = note(20, "/a", "2026-09-08T10:00:00Z");
  var sorted = ui.sortNotesDescending([a, b]);
  assert.strictEqual(sorted[0].id, 20);
  assert.strictEqual(sorted[1].id, 10);
})();

// groupByRepo groups by repo and preserves first-appearance order.
(function () {
  var b1 = note(1, "/repo-b", "2026-09-09T10:00:00Z");
  var a1 = note(2, "/repo-a", "2026-09-08T10:00:00Z");
  var b2 = note(3, "/repo-b", "2026-09-07T10:00:00Z");
  var groups = ui.groupByRepo([b1, a1, b2]);
  assert.strictEqual(groups.length, 2);
  assert.strictEqual(groups[0].repo, "/repo-b");
  assert.strictEqual(groups[0].notes.length, 2);
  assert.strictEqual(groups[1].repo, "/repo-a");
  assert.strictEqual(groups[1].notes.length, 1);
})();

// Empty inputs stay empty.
assert.deepStrictEqual(ui.sortNotesDescending([]), []);
assert.deepStrictEqual(ui.groupByRepo([]), []);

console.log("list.test.js: ok");
