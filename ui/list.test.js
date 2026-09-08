"use strict";

const assert = require("node:assert");
const {
  formatTimestamp,
  sortNotesDescending,
  groupByRepo,
} = require("./list.js");

function note(id, createdAt, repoPath) {
  return {
    id,
    text: "text " + id,
    repo_path: repoPath,
    branch: "main",
    commit_hash: "abc",
    changed_files: [],
    created_at: createdAt,
    done: false,
  };
}

// sortNotesDescending: newest created_at first, and it does not mutate input.
{
  const notes = [
    note(1, "2026-09-01T10:00:00Z", "/a"),
    note(2, "2026-09-03T10:00:00Z", "/a"),
    note(3, "2026-09-02T10:00:00Z", "/b"),
  ];
  const sorted = sortNotesDescending(notes);
  assert.deepStrictEqual(
    sorted.map((n) => n.id),
    [2, 3, 1],
    "notes must be ordered newest first"
  );
  assert.deepStrictEqual(
    notes.map((n) => n.id),
    [1, 2, 3],
    "sortNotesDescending must not mutate its input"
  );
}

// Full render flow: sort newest-first, then group by repo_path.
// Groups are ordered by their newest note; notes inside a group stay newest first.
{
  const notes = [
    note(1, "2026-09-01T10:00:00Z", "/repo-a"),
    note(2, "2026-09-03T10:00:00Z", "/repo-b"),
    note(3, "2026-09-02T10:00:00Z", "/repo-a"),
  ];
  const groups = groupByRepo(sortNotesDescending(notes));
  assert.strictEqual(groups.length, 2, "must yield two repo groups");

  assert.strictEqual(groups[0][0], "/repo-b", "repo-b has the newest note");
  assert.deepStrictEqual(
    groups[0][1].map((n) => n.id),
    [2]
  );

  assert.strictEqual(groups[1][0], "/repo-a");
  assert.deepStrictEqual(
    groups[1][1].map((n) => n.id),
    [3, 1],
    "within a repo, notes stay newest first"
  );
}

// groupByRepo preserves the incoming (already newest-first) order inside a group.
{
  const notes = [
    note(3, "2026-09-02T10:00:00Z", "/repo-a"),
    note(1, "2026-09-01T10:00:00Z", "/repo-a"),
  ];
  const groups = groupByRepo(notes);
  assert.deepStrictEqual(
    groups[0][1].map((n) => n.id),
    [3, 1]
  );
}

// groupByRepo handles a missing repo_path by grouping under "".
{
  const groups = groupByRepo([note(1, "2026-09-01T10:00:00Z", "")]);
  assert.strictEqual(groups[0][0], "");
}

// formatTimestamp is deterministic for invalid/missing input.
assert.strictEqual(formatTimestamp(""), "");
assert.strictEqual(formatTimestamp(null), "");
assert.strictEqual(formatTimestamp("not-a-date"), "not-a-date");

console.log("ui/list.test.js: all assertions passed");
