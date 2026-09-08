<!-- office-crew-developer-conventions: v23 -->
<!-- office-crew-developer-md: full -->
# Workspace conventions for AI developers

You are a developer in a team working on a single ticket. Read `SPEC.md` for
the full sprint scope, then implement YOUR ticket.

## Hard rules

0. **Act, don't announce.** Never end a turn with only a sentence like
   "Let me read SPEC.md" or "I'll create the file" and then stop. If you say you
   will do something, your VERY NEXT step MUST be the actual tool call
   (Read/Write/Edit/Bash) — in the same turn. A turn that contains only prose
   and no tool call counts as zero work done and wastes a round.

1. **Do NOT overwrite existing files from scratch.** Always Read first, then
   add to / modify them. This is especially important for `app/main.py`,
   `requirements.txt`, `pyproject.toml`, `README.md` — other tickets may have
   added content there that you must preserve.

2. **Module naming is fixed.** Use the names that already exist in the repo.
   If you see `app/db.py`, do not create `app/database.py`. If unsure, ask
   `knowledge_symbol` (or grep) for usages first.

2b. **Orient through the knowledge tools, not by reading the tree.** The team's
   knowledge base has already parsed this repository, and it is offered to you as
   tools: `knowledge_search` names the files a question is about (source apart from
   tests) and what the team already decided about it; `knowledge_symbol` says where a
   name is defined and who calls it; `knowledge_file` what a file defines, imports and
   what imports it; `knowledge_overview` the shape of the codebase. Ask those FIRST,
   and read only the files they point at — a file you are about to edit you read in
   full, a file you only need to understand you ask about. A developer who greps and
   reads twenty files to find one function spends the ticket's budget on orientation,
   and the answers were already there. Each tool says what it did NOT find, so "does
   this exist yet?" is one call. When the tools are not offered (a repository that has
   not been analyzed yet), grep and read as before.

3. **Never commit generated files.** The `.gitignore` already excludes
   `__pycache__/`, `*.pyc`, `*.db`, `.venv/`. Don't `git add -A` if you have
   such files in the working tree — use explicit paths.

4. **Tests are part of done.** Every ticket includes writing tests AND making
   them pass. `cargo test` must exit 0 before you stop.

   **A green run here does NOT mean a green pipeline.** This machine is a
   developer workstation with many packages already installed; the CI starts from
   an empty container and installs ONLY what the manifest declares. So a
   dependency you use but forget to declare imports fine here and dies there at
   collection time — the whole suite, not one test. Before you finish: for every
   third-party module your code imports, check that it (or the extra that brings
   it) is in `requirements.txt` / `pyproject.toml`. Watch the ones that arrive as
   an EXTRA rather than a package of their own — `pydantic.EmailStr` needs
   `pydantic[email]`, `python-jose[cryptography]`, `uvicorn[standard]`. Adding a
   dependency is part of the change that uses it.

   **Every dependency you add carries a version bound.** A bare name is not a
   dependency, it is a bet: the CI resolves it to whatever released most recently,
   which is not what you installed, and the two drift apart with no commit in
   between. Write `name>=X,<Y` (or an exact `==`) for everything you add — and
   when a package needs a companion at a matching version, bound BOTH, because a
   resolver satisfying one is free to break the other. Measured 2026-07-29: a bare
   `passlib[bcrypt]` resolved to bcrypt 4.x, whose removal of `bcrypt.__about__`
   makes passlib 1.7.4 raise on the very first password hash. Every red pipeline
   of that day — six of six, across two sprints — was this one line, and it cost
   two repair rounds and a full ticket redo while every local run stayed green.

5. **Stay within your ticket.** No drive-by refactors of unrelated code, even
   if it looks improvable. Note observations in the commit message instead.

6. **Do NOT create branches or push.** The orchestrator handles all git
   operations after you. Just commit-worthy state in the working tree is
   enough — actually, don't even commit; the orchestrator does that too.

6b. **Never run a command that does not return on its own.** A dev server,
   a watcher or a REPL keeps running until it is killed, so the shell call
   never comes back — you sit there producing no output and the run is
   eventually cut off, mid-ticket, with the last step unfinished. This is the
   single most common way a ticket is lost.

   **Do NOT start a server to check your endpoints — not even a "bounded" one
   you intend to kill afterwards.** Assert them IN-PROCESS instead:

   ```python
   from fastapi.testclient import TestClient
   def test_register_and_login():
       c = TestClient(app)
       assert c.post("/auth/register", json={...}).status_code == 201
       assert c.post("/auth/login", json={...}).status_code == 200
   ```

   This is not a style preference — the background-and-kill recipe cannot be
   made to work here, and it cost real tickets before this rule existed. The
   shell captures `$!`, but on Windows `py`/`python` is launched through a
   store alias that starts the REAL interpreter as a separate process, so the
   pid you saved belongs to a launcher that has already exited. Your `kill`
   then hits nothing: the server keeps the port AND keeps the pipe your own
   command writes to open, so the tool call never returns and your run is cut
   off with the last step unfinished — with every file you wrote still there
   and no way to say you were done.

   `TestClient` needs no port, no pid and no cleanup, exercises the same
   routes, and its result is a test that stays in the repo. Tessa runs the
   real server later; proving the process boots is HER job, not yours.

   Same rule for the frontend: `npm run build` tells you it compiles;
   `npm run dev` never finishes. And if a port is already taken, do NOT wait
   or retry in a loop — that is a second way to never return.

7. **Ask questions when stuck — DON'T guess.** If the ticket or SPEC is
   ambiguous on a decision that affects the result (e.g. "should pagination
   default to 20 or 50?", "sync or async client?", "what behavior on duplicate
   input?"), do this instead of inventing an answer:

   The same applies when a requirement is NOT ambiguous but you believe it is
   WRONG — a wrong HTTP verb, a contradiction inside the ticket, a name that
   fights the convention the rest of the code follows. That case has no obvious
   home: rule 1 says implement exactly the ticket, and your judgement says the
   ticket is mistaken. **What you must NOT do is quietly build the better
   version.** The reviewer judges the diff against the ticket, so a silent
   deviation is the one outcome that is guaranteed to block the merge — and
   nobody learns the ticket was wrong. So: implement it AS WRITTEN, and end your
   final message with one line per objection, in exactly this form:

   ```
   OBJECTION: <which requirement, and why you believe it is wrong>
   ```

   That line is read mechanically — it reaches the reviewer, the merge request
   and the Tech Lead, so the NEXT ticket can be better. Prose without the marker
   reaches no one. If following the requirement would produce something
   contradictory or actively harmful, use `QUESTION.md` and stop instead.

   - Write a file `QUESTION.md` in the repo root with:
     ```
     # Question to Tech Lead

     **Context**: <which file / which spec point is unclear>

     **Question**: <your specific question, with options if any>

     **Why it matters**: <how the choice changes the implementation>
     ```
   - Then STOP working. Do NOT write code that depends on the unknown choice.
     Do NOT commit anything. Exit cleanly.
   - The Tech Lead (or the architect) will answer in `ANSWER.md`, then you'll
     be re-spawned to continue. You'll find `ANSWER.md` in the repo root.
   - When you read `ANSWER.md`: act on the answer immediately, then DELETE
     `QUESTION.md` and `ANSWER.md` (no clutter in the MR).

   Limit: max 3 questions per ticket. Use this for real decisions, not for
   trivial style choices — for those, make a reasonable choice and document it
   in a code comment as before.

<!-- office-crew-developer-subagents: off -->

## CI/CD is active (GitHub Actions with Quality Pack)

The repo has a `.github/workflows/ci.yml`. On every `git push` the following runs automatically:
- **test** (`cargo build` + `cargo test`) — BLOCKING
- **lint** (`cargo fmt --check` + `cargo clippy`) — warning, not blocking

Important consequences for you:
1. **`cargo build` and `cargo test` must be green locally.** New functions need
   tests (`#[test]` / `tests/*.rs`).
2. **Run `cargo fmt`** before you finish; keep clippy reasonably clean.
3. **No hardcoded secrets/keys/URLs** — use env vars.
4. **Keep `Cargo.toml` current** when you add dependencies.

## Documentation requirement

The repo MUST ship with a REAL `README.md` at the root — never just the repo
name or the GitHub auto-init stub. It MUST contain:
- a one-paragraph project description;
- the tech stack;
- install steps;
- how to RUN in dev AND (if there is a build) how to BUILD for production;
- HOW TO USE IT — describe whatever actually fits THIS product: controls/key
  bindings for a game, the main screens and actions for an app, the public
  API (endpoints/functions) for a service or library, the commands and flags
  for a CLI. Not every project has "controls" — document what this one has;
- the feature list.
Keep it in sync with your ticket — new endpoints / functions / config / controls
get reflected here. For REST APIs also give, per endpoint: method, body schema,
response example. For a CLI, show example invocations; for a library, a minimal
usage snippet. If the project has a `docs/` structure, deep docs go there but
the README stays the entry point.

**The README belongs to the ticket that DECLARES `README.md` in its `files`, and to
no other.** If your ticket does not declare it, do not create it and do not rewrite
it: its absence from YOUR branch is not your defect and no review may hold it
against you. Name what you would have documented in your final message instead, and
its owner carries it.

That rule is not a formality. A sprint's skeleton is often SPLIT into two halves
that branch from the same empty repository in the same second, so neither can see
the other's work — and the half that does NOT own the README looks at its branch,
finds none, and writes one to avoid being rejected. Measured 2026-08-06, in both
runs of that day: "Scope collision: README.md belongs to 'Backend-Skelett…'", one
trim/justify turn each time, in wave 0 — the wave every other ticket waits on.

The reviewer REJECTS an MR whose product ships without this README **when that MR
is the one that owns it**.

## A stub answers 501, never 500 (BINDING)

A sprint is built ticket by ticket, so a route another ticket still owes is normal
and expected. Declare it and answer **501 Not Implemented**, explicitly:

```python
raise HTTPException(status_code=501, detail="wardrobe #5 implements this")
```

NOT `raise NotImplementedError(...)`. Your framework turns that into a **500**, and
500 means one thing only: the server crashed handling a request it was supposed to
serve. The whole runtime QA layer rests on that distinction — the browser pass
treats a 500 as a hard failure, because no correct product ever emits one, while a
501 is a fact it reports and walks past. Blur the two and either every mid-sprint
build looks broken, or a real crash hides among the stubs.

Measured 2026-07-31 (run 20260731-104915): `backend/wardrobe.py` and
`backend/outfits.py` both raised `NotImplementedError`, so a signed-in user's
`GET /api/wardrobe/items` answered 500 — indistinguishable from the auth defect
being hunted at that moment in the same run.

The same rule applies to a frontend page another ticket owns: render a placeholder,
never throw.

## Never test a stub's answer, and keep your tests in your own file (BINDING)

A stub is a promise that something will change. A test that asserts what it answers
TODAY is therefore a test that must fail tomorrow — and it fails on **main**, after
both branches were green, where no review looks and no developer owns it.

```python
def test_stub_items_list_returns_501():     # ← this is the bug
    assert client.get("/api/items/").status_code == 501
```

Assert what is true on both sides of the gap instead: the route exists and is
reachable (not a 404), it is registered under the agreed path and verb, it rejects an
unauthenticated caller if it always will. Same for a placeholder screen, an empty
list or a "not implemented" message — test that the thing is WIRED, never the
temporary body.

And put your tests in **your own** test file, named after your slice
(`tests/test_<slice>.py`, `<slice>.test.ts`, `<slice>_test.go`). One test file the
whole product appends to is a shared collection file exactly like the app entry
point: every parallel ticket has to edit it, so every parallel branch conflicts on
it. If you write the scaffold, put in it ONLY the tests for what the scaffold itself
delivers.

Measured 2026-08-01 (run 20260801-171624): the skeleton wrote
`test_stub_auth_register_returns_501`, `test_stub_auth_login_returns_501`,
`test_stub_items_list_returns_501` and `test_stub_outfits_list_returns_501` into one
`backend/tests/test_main.py`. Three later tickets each had to edit that one file to
delete an assertion about their own route, they collided on it in the rebase, and the
review of one of them was praised for keeping a stub test "consistent". Main's
pipeline went red twice.

## A test provisions its own state and owns only its own tables (BINDING)

Your tests run on a branch, in parallel with other tickets, and in a container that
has nothing in it. Three rules follow, and each one is a failure that has happened:

1. **Create the schema you use.** Never assume a table exists because some other
   part of the app would have created it. Build it in the fixture, or drive the
   app's own startup so it builds it — and do that BEFORE the first statement runs.
2. **Touch only the tables your ticket owns.** `DELETE FROM <a table another ticket
   owns>` couples your ticket to whichever sibling merged first, and on your branch
   that table may not exist at all. Reset what you wrote; leave the rest alone.
3. **The test database is not a file in the repository tree.** Use an in-memory
   database (`sqlite:///:memory:`) or a path under pytest's `tmp_path`. A file next
   to the source survives between runs on the machine that wrote it and does not
   exist in the pipeline — which is the "green here, red there" report nobody can
   reproduce. It also means two test modules can point at two different files while
   sharing one engine, and then one of them finds a schema the other never made.

Measured 2026-08-01: an authentication ticket's autouse fixture ran
`DELETE FROM outfit_items, outfits, clothing_items, users` against
`sqlite:///./test_4.db` before anything had created a single table. Twelve tests
ERRORed with `no such table: outfit_items` — a table belonging to two other tickets
— while the rest of the suite passed. The pipeline was red for a defect that had
nothing to do with the feature under test.

## RUN.json — how this product starts (BINDING, and it is EXECUTED)

The repo root carries `RUN.json`: the machine-readable half of the README's
"how to run". Tessa's test runner and the CI pipeline do not guess how to start
this product — they read this file and execute exactly what it says. A start
command that does not start the product is a defect of the same severity as a
crash, and prose in the README that disagrees with this file is a defect too.

One entry per startable part:

```json
{
  "runcontract": 1,
  "services": [
    {
      "name": "api", "dir": "backend", "stack": "python", "kind": "server",
      "install": [["${python}", "-m", "pip", "install", "-q", "-r", "requirements.txt"]],
      "start": ["${python}", "-m", "uvicorn", "app.main:app", "--port", "${PORT}"],
      "port": 8000, "port_env": "PORT",
      "health": { "path": "/api/health", "expect_status": [200] },
      "env": {
        "DATABASE_URL": { "dev": "sqlite:///./dev.db" },
        "JWT_SECRET":   { "generate": "hex", "bytes": 32 },
        "STRIPE_KEY":   { "external": true,
                          "degraded_without": "POST /api/checkout answers 503" }
      }
    }
  ]
}
```

- Commands are **argv lists**, never shell strings (`["npm", "run", "build"]`).
- Placeholders, and only these three: `${python}` (the interpreter running the
  service), `${PORT}` (the port actually bound), `${service:<name>.origin}` (the
  URL another declared service ended up on — use it for CORS origins and for a
  frontend's API base, and never hardcode a port instead).
- Every variable the process needs to BOOT belongs here, in exactly one class:
  `dev` (a literal that is demonstrably not a secret), `generate` (a recipe —
  the value is rolled per run and never exists in the repo), or `external` (no
  value at all: the product MUST boot without it and degrade, and
  `degraded_without` says what stops working).
- **A secret never has a value here.** `JWT_SECRET: {"dev": "..."}` is rejected —
  that is a signing key in a git repository. Use `generate` or `external`.
- **A `generate` variable has no source in the repo by design — so the repo must
  hand a HUMAN what the pipeline rolls for itself.** Ship a committed
  `.env.example` that names every one of them, or a start command in the README
  that exports them. Documenting the variable in a table is NOT a mechanism: the
  reader copies the code block, not the table. Measured 2026-07-30: `JWT_SECRET`
  was declared here correctly, every pipeline run invented one and was green, and
  the customer who cloned the repo could not sign in.
- Config is read LAZILY, never at import time — but VALIDATED ONCE AT STARTUP.
  `os.environ["X"]` in a module body or a class body kills the process before it
  can serve anything, and no default and no error message can help the person who
  cloned the repo. Reading it lazily is only half the rule: on its own it moves the
  crash from boot to the first request that happens to need the value, so the
  process binds, the health endpoint answers 200, and the product looks alive to
  every automated check while being unusable. In the startup/lifespan hook, touch
  the whole settings object once, so a missing REQUIRED value refuses to start and
  says which one. Measured 2026-07-30: `/api/health` 200, `POST /api/auth/register`
  201, `POST /api/auth/login` 500 — the one handler that never read config was the
  one the pipeline probed.
- The health endpoint must exercise the SAME configuration the real routes use.
  One that answers before reading anything proves only that a port is bound.
- `kind`: `server` (needs `health` or `ready_log`), `static` (needs
  `serve_dir` — the built output), `cli`, `worker`.
- Keep it TRUE as the product grows: a ticket that adds a variable, a port, an
  endpoint used as health check or a whole service updates this file in the same
  MR. If you cannot state a field honestly, leave it out rather than guess.
- **PROVE IT BEFORE YOU FINISH: run `py _office_run_check.py` from the repo
  root.** This is the ONE exception to rule 6b, and it is not a server you start:
  it is the office's own runner, and it always comes back. It reads this file,
  starts every declared `server` with the environment declared here, probes the
  declared health path, kills the whole process tree and prints a verdict.
  A non-zero exit means the product does not start the way you just declared —
  fix whichever half is wrong before you push.

  Do this even when your tests are green, because a green suite cannot settle it:
  tests import your modules directly and never take the declared start path.
  Measured 2026-08-02: `RUN.json` said `uvicorn app.main:app` while the app was
  `backend/main.py`. pytest passed, the reviewer found it, and the ticket paid a
  CHANGES_REQUESTED plus a full extra round for one wrong word — one that this
  command names in about two seconds.

  `_office_run_check.py` is office-owned and gitignored: never edit it, never
  commit it, and never work around a failure it reports.

## Design tokens (UI projects only — when DESIGN.md is in the repo)

If the file `DESIGN.md` exists in the repo root, **IT IS BINDING** for all
visual decisions:

1. **Do not improvise your own hex colors.** When you need a color, use exactly
   one from the `## Colors` section of DESIGN.md. Ideally as a CSS
   custom property: `:root { --color-bg: #...; }` (from the tokens), then in the
   code just use `var(--color-bg)`.
2. **Spacing comes from the scale.** No random `padding: 13px` — if the
   scale has 4/8/12/16/24, take a value from it. Also think in custom properties:
   `--space-0`, `--space-1`, etc.
3. **Border radii from DESIGN.md** — do not guess them yourself.
4. **Follow the component specs.** If DESIGN.md says "Button: padding 12/24,
   radius md, min-height 44px" — implement it exactly that way, not "approximately".
5. **If DESIGN.md does not specify a component** that your ticket needs:
   improvise GENTLY in the same style (same tokens, same logic
   as the other components). Note it in the commit message: "new component X
   improvised without a spec — the designer can align it later".
6. **Never touch DESIGN.md** — it is maintained by the designer (Luna).

The reviewer checks token consistency on UI MRs.

## UI/UX Pro Max Skill (use on UI/frontend tickets)

On this system the Claude Code skill **`ui-ux-pro-max`** is installed globally
(67 styles, 96 palettes, font pairings, UX guidelines, stack best practices).
If your ticket builds visible UI (pages, components, styling, landing,
dashboard), use it BEFORE and DURING the implementation:

1. **Pull the design system** (pattern, style, colors, typography, effects,
   anti-patterns for the project type):
   ```
   py "C:/Users/Anwender/.claude/skills/ui-ux-pro-max/scripts/search.py" "<product type industry keywords>" --design-system -f markdown
   ```
2. **Get the stack guidelines** (default stack `html-tailwind`):
   ```
   py "C:/Users/Anwender/.claude/skills/ui-ux-pro-max/scripts/search.py" "<topic>" --stack html-tailwind
   ```
3. **Invocation rule (IMPORTANT):** always use `py` + the ABSOLUTE path exactly as above.
   `python` / `python3` are broken Store stubs on this system and will
   fail.
4. **Priority on UI projects:** if a `DESIGN.md` is in the repo, its tokens
   ALWAYS win (colors/spacing/radii — maintained by Luna). The skill
   only fills in where DESIGN.md is silent: UX patterns, accessibility,
   hover/transitions, stack idioms and the pre-delivery checks.

## Quick context

- Stack: depends on project — read existing files first to find out
- For Python/FastAPI projects: DB usually in `app/db.py`, tests via pytest
- For frontend projects: see "Browser frontends" section below

## Browser frontends (HTML/CSS/JS for direct browser use)

If your ticket produces files that are meant to be opened in a browser:

1. **The README MUST contain a "How to run" section**, because `file://` URLs
   break ES-Modules, fetch(), and most modern web APIs (CORS blocks them).
2. Minimum acceptable instructions:
   - Static files only: `py -m http.server 8000`, then open `http://localhost:8000`
   - Or: use VS Code Live Server extension
   - If a build tool is involved (Vite, Webpack, etc.): the exact build + serve commands
3. If you write a small launcher script (e.g. `start.bat` on Windows, `start.sh` on Unix), even better.
4. Do NOT assume the user knows to start an HTTP server. State it explicitly.

This applies regardless of whether tests are required for the ticket.

## Separate Frontend + Backend (cross-origin) — CORS is mandatory

If your ticket creates a Frontend that talks to a separate Backend (Vite + FastAPI,
React + Flask, etc.), CORS MUST be configured from the start:

- **FastAPI:** add `CORSMiddleware`, and read the allowed origin from CONFIGURATION —
  never as a hardcoded port list:
  ```python
  import os
  from fastapi.middleware.cors import CORSMiddleware

  app.add_middleware(
      CORSMiddleware,
      # One value, from the environment, with a dev default. RUN.json declares it as
      # "FRONTEND_ORIGIN": {"dev": "${service:frontend.origin}"} — the runner fills in
      # where the frontend REALLY ended up, so this list is right on any port.
      allow_origins=[os.environ.get("FRONTEND_ORIGIN", "http://localhost:5173")],
      allow_credentials=True,
      allow_methods=["*"],
      allow_headers=["*"],
  )
  ```
  The same rule in the other direction: a frontend's API base is
  `"VITE_API_URL": {"dev": "${service:api.origin}"}`, never a literal
  `http://localhost:8000`. A hardcoded port on either side means the two halves can
  only ever be started on exactly those two numbers — and when one of them is busy,
  the product rejects its own frontend and the report reads like a product defect.
  NEVER widen the rule to make a rejection go away: `allow_origin_regex` over
  `localhost|127.0.0.1` with credentials allowed is a security defect, and it has been
  written into two sprints' products by exactly this reasoning.
- **Flask:** `from flask_cors import CORS; CORS(app, origins=[...])`
- The default behavior without CORS is: the browser blocks — the user sees
  `Access-Control-Allow-Origin` missing errors, the app is unusable.
- **A signed-in session MUST survive a page reload.** A token kept only in a module
  variable or in component state is gone the moment the browser loads a page — a
  reload, a bookmarked URL, a link opened directly — and every protected request
  then answers 401 while the UI still believes it is signed in. Persist it
  (`localStorage`, or an httpOnly refresh cookie) and restore it at start-up. If an
  acceptance criterion forbids that storage, say so with an `OBJECTION:` line
  instead of shipping a session that dies on F5 — the criterion is then incomplete
  and its author has to resolve it. Measured 2026-08-06 (run 20260806-142759):
  `api.js` held `let _token = null` and nothing else; the browser pass reported
  `GET /api/outfits -> 401` AND `GET /api/wardrobe -> 401` from two different pages,
  and the bug was filed against the outfits router, which was innocent.
- **A 5xx must keep its CORS headers.** `add_middleware(CORSMiddleware)` places
  CORS INSIDE Starlette's `ServerErrorMiddleware`, so an unhandled exception is
  sent by the outer layer and arrives at the browser with no
  `Access-Control-Allow-Origin` at all. The frontend then sees
  `TypeError: Failed to fetch` with `err.response === undefined` — indistinguishable
  from "the backend is not running" — and its catch block renders a generic
  message. Every server bug becomes an undiagnosable client bug. Add a catch-all
  handler so the error response is produced INSIDE the CORS layer:
  ```python
  @app.exception_handler(Exception)
  async def unhandled(request: Request, exc: Exception):
      logger.exception("unhandled error on %s %s", request.method, request.url.path)
      return JSONResponse(status_code=500, content={"detail": "Internal Server Error"})
  ```
  Measured 2026-07-30: a missing setting made every sign-in answer 500, and because
  the response lost its CORS header the user's only symptom was "Anmeldung
  fehlgeschlagen" with an empty console.

You never need to know the exact frontend port: that is what the environment variable
and `${service:frontend.origin}` are for. If a user-given config indicates production,
use the proper production origin as the default instead.
