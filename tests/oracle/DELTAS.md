# Accepted deltas

Cases listed here differ from their JS golden on purpose. Each entry names the
case id, what differs, and why it is an improvement. Nothing gets on this list
without review.

Format: `- \`<case-id>\`: <what differs> (<why>)`

## Recorded 2026-08-17: the engine names its own commands

The JS scripts printed their own file names in usage lines, directives, and
the hook manifests they wrote. The binary prints the verb (`impeccable doctor`)
or the launcher path (`"<scripts>/impeccable" hook`). Each case below was
re-recorded from the engine after a line-level review confirmed the only change
is that wording; behavior, exit codes, and every other byte are unchanged.

- `doctor-help`, `doctor-help-short`: `Usage: node doctor.mjs …` is now `Usage: impeccable doctor [--json] [--fix] [--target <path>]`.
- `doctor-legacy-text`: the closing hint reads `Run \`<self> doctor --fix\``.
- `pin-usage-no-args`, `pin-usage-one-arg`: `Usage: impeccable pin <pin|unpin> <command>`.
- `surface-brief-usage`, `surface-brief-unknown`, `surface-brief-write-usage`: usage lines name `impeccable surface-brief`.
- `critique-usage`, `critique-unknown`: usage lines name `impeccable critique-storage`.
- `context-monorepo-target-missing`: MONOREPO_TARGET_REQUIRED says `impeccable context ran without --target`.
- `hadmin-on`, `hadmin-on-twice`, `hadmin-off-then-status`, `hadmin-on-repairs-existing-manifest`, `hadmin-on-malformed-manifest-backup`: `hooks on` writes manifests that run the launcher (`"<scripts>/impeccable" hook`, Cursor `hook-before-edit`) instead of `node "<scripts>/hook.mjs"`.
- `hook-session-fresh-then-pending-then-stop`, `hook-session-two-sessions`, `hbe-denial-downgrade-after-6`: the short footer names `impeccable hooks ignore-value`.
- `live-help`, `live-accept-help`, `live-inject-help`, `live-insert-help`, `live-server-help`, `live-resume-help`, `live-commit-help`, `live-discard-help`, `live-complete-help`, `live-complete-no-id`: usage text names `impeccable live*` verbs.
- `live-server-already-running`, `live-daemon-server-status-poll-complete`, `live-status-empty`, `live-status-generating`, `live-status-many-sessions`, `live-status-stale-server-json`, `live-status-legacy-sessions-dir`, `live-status-from-subdir`, `live-status-manual-apply`, `live-resume-manual-apply`, `live-status-mount-failed`, `live-resume-mount-failed`, `live-resume-generating`, `live-resume-by-id`, `live-resume-first-active-sorted`, `live-resume-accept-requested`, `live-resume-carbonize-required`: recovery hints and next-command lines spell `<self> live-poll` / `live-server` / `live-complete` / `live-commit-manual-edits` instead of the `.mjs` names.

## Recorded 2026-08-17: live-inject adds `'wasm-unsafe-eval'` to a CSP meta script-src

The detector the live overlay loads from the helper origin is a WebAssembly
module in the engine (its `docs/WASM-BUNDLE.md`); a `script-src` that names the
origin but not `'wasm-unsafe-eval'` still refuses to compile it. The JS
`patchCspMeta` predates the wasm bundle and appended only the origin.

- `live-inject-csp-meta-no-connect-src`: the patched `<meta http-equiv="Content-Security-Policy">` reads `script-src 'self' http://localhost:8412 'wasm-unsafe-eval'` (was `script-src 'self' http://localhost:8412`). The `data-impeccable-csp-original` marker, the `connect-src` and `img-src` additions, idempotence, and the revert on unpatch are unchanged. `live-inject-vite-csp-meta` and `live-inject-next-jsx` carry meta tags the patch does not touch, so their goldens did not move.

## Recorded 2026-08-31: detector-engine ports landed, gap goldens restored

The section previously here pinned the gap between main's post-freeze detector
fixes and the engine. Those fixes are now ported (engine repo commits:
`c0aa75f` oklch in visual-contrast/neon-text, upstream 1b7da15b #592;
`5cdeec8` color-mix nested hex, upstream 54440319 #578; the 1D grid fix,
upstream a236137b #615, rode along in `9046e8f` via a concurrent staging race;
`6d36231` comment stripping for regex matchers, upstream 067665cc #589 +
ddb60993 + ba873f75 + 9a7d0fbc; `33aef88` root-relative linked stylesheets,
upstream 2b88aa52 #652 + daae1d41; `6d0ecf1` URL userinfo redaction with
origin-scoped basic auth, upstream d5873ff8 + d690349d #657; `09f8ae7` inert
exact ignore-value refusal, upstream be87f5eb #662; `20c8347` the
comp-fidelity rules organic-clip-path and buried-raster, upstream 58561610).
The affected goldens were re-recorded from the fixed engine and each json
fixture golden was byte-verified against the last JS engine state in history
(`db1462b9^`, which carries both main's drift and the comp-fidelity rules):

- Moved to post-fix behavior: `detect-fixture-json-codex-grid-1d-pass-html`,
  `detect-fixture-text-codex-grid-1d-pass-html` (no finding, exit 0),
  `detect-fixture-json-organic-clip-path-html`,
  `detect-fixture-text-organic-clip-path-html`,
  `detect-fixture-json-buried-raster-html`,
  `detect-fixture-text-buried-raster-html` (the new rules fire),
  `detect-fixture-json-glow-html`, `detect-fixture-text-glow-html` (glow's
  `.photo-opaque-grad` column now carries its intended buried-raster finding),
  and the sweeps `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`,
  `detect-no-advisory-text`.
- Unchanged on re-record (already matched the fixed JS in the static engine):
  `detect-fixture-json-color-html`, `detect-fixture-text-color-html`,
  `detect-fixture-json-oklch-neon-text-html`,
  `detect-fixture-text-oklch-neon-text-html` (the oklch and color-mix fixes
  observably change the browser-side visual-contrast path, which these static
  scans do not exercise), `detect-scope-type`, `detect-scope-both`.

The frozen call vectors for `checkHtmlPatterns`
(`tests/oracle/vectors/calls/rules.checks/checkHtmlPatterns.jsonl`) were
re-recorded the same way: args untouched, results replayed through the
`db1462b9^` JS (14 of 101 moved: the comp-fidelity scans and the
comment-stripping/inline-fragment fixes to `enclosingCssSelector`). No case in
this section is an accepted delta any more; the engine matches the final JS.

## Recorded 2026-08-31: main's Aug 17-31 verb fixes ported to the engine, goldens re-recorded

The goldens below froze pre-fix behavior. Each fix landed on main in JS and
was ported to the engine; the cases were re-recorded from the binary and
reviewed line by line, so they now pin the fixed behavior.

- `hook-session-fresh-then-pending-then-stop`, `hook-session-two-sessions`: the Stop deep pass syncs the remembered set to the live scan, including findings the per-edit pass already surfaced, so a second Stop with nothing new is silent and a fixed-then-reintroduced finding fires again (upstream 3c442af7).
- `hadmin-on`, `hadmin-on-twice`, `hadmin-off-then-status`, `hadmin-on-repairs-existing-manifest`, `hadmin-on-malformed-manifest-backup`: the Claude manifests `hooks on` writes match on `Edit|Write` and the description names the current tools; Claude Code folded multi-edit behavior into Edit (upstream 7d5c60d2).
- `live-commit-mock-unreported-file-change`: the rollback-failure results share one constructor, which moved `unreportedFiles` and `notes` after `pageUrl` in the emitted JSON (upstream 1f2c3f9d).

## Recorded 2026-08-31: main's Sep-1 verb fixes ported after the rust-swap rebase

Five more fixes landed on main in JS between the swap branch and its rebase.
Each was ported to the engine and the affected goldens re-recorded from the
binary after a line-level review; the engine's output was also diffed
byte-for-byte against the upstream JS on the same inputs before recording.

- `critique-usage`, `critique-unknown`: the usage line now lists the new `close` subcommand (upstream 5211bdf4, #660).
- `critique-latest-existing`: `latest` applies the #660 identity/freshness path: a legacy snapshot carrying no fingerprint for a concrete local target is closed and `latest` exits 2 instead of printing the stale body (upstream 5211bdf4, #660).
- `critique-write-then-read`: `write` stamps `target_identity`/`target_fingerprint`/`target_path`, uses a fixed-width `~NNNN` collision suffix when two snapshots share a UTC second, `latest` freshness-closes the read snapshot, and `trend` now surfaces the `closed` flag and identity fields (upstream 5211bdf4, #660).
- `critique-write-monorepo-child`: `write` stamps the resolved `target_identity`, and a `latest` run from a sibling app resolves to a different identity so it exits 2 rather than returning the neighbor's backlog (upstream 5211bdf4, #660).
- `detect-fixture-json-overused-font-html`, `detect-fixture-text-overused-font-html`: new fixture added on the swap branch; overused-font primary selection now skips only the CSS generics, so a system stack keeps its system face as primary and later web-font fallbacks like Roboto no longer flag (upstream 2cfd6076, #678).
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-scope-type`, `detect-scope-both`, `detect-no-advisory-json`, `detect-no-advisory-text`: the directory sweep picks up the new overused-font fixture and the #678 primary-face change (upstream 2cfd6076, #678).

## Recorded 2026-08-31: E8 hook-manifest self-heal on upgrade

Two new cases pin the fix for triage E8 (the v3-to-launcher upgrade path). The
JS `automaticHookMode` counted any hook command naming the skill as an active
hook, including the JS-era `node .../hook.mjs` form. After a skill update the
`.mjs` script no longer exists, so that manifest points at a dead command yet
still suppressed `MANUAL_DETECTOR_REQUIRED`, leaving the detector dark. The
engine now treats a manifest that names ONLY the `.mjs` form as not an active
launcher hook, so the manual detector fallback fires until install/update
repairs the manifest to the launcher form. The launcher form still counts as
active exactly as before. No existing golden moved: every other `context` case
runs under the `source` provider, whose manifest list is empty, so none of them
scan a hook manifest.

- `context-stale-hook-manifest`: a `.claude/settings.local.json` naming `node "${CLAUDE_PROJECT_DIR}/.claude/skills/impeccable/scripts/hook.mjs"` under the `claude-code` provider emits `MANUAL_DETECTOR_REQUIRED` (the stale marker no longer counts as active).
- `context-launcher-hook-active`: the same manifest in the launcher form (`"…/impeccable" hook`) suppresses `MANUAL_DETECTOR_REQUIRED`, confirming the launcher marker is still recognized as active.

## Recorded 2026-09-01: the harness stages workspaces at their real path

Two goldens were re-recorded after `stageWorkspace` started returning the
realpath of the staged directory. macOS's tmpdir is a symlink (`/var` ->
`/private/var`), and the old goldens carried that artifact rather than the
verbs' behavior; Linux, where the two paths are the same, never reproduced
them. The binary's output is unchanged; the input the harness fed it is.

- `context-dir-override`: `productPath` is `elsewhere/PRODUCT.md`, the plain relative path, instead of `../../../../../../..<WS>/elsewhere/PRODUCT.md` (a relative path from the symlinked cwd to the resolved one).
- `live-accept-source-locked`: the accept now reports `source_locked`, which is what the case is named for. The staged lock named the file under the symlinked path, so the verb never matched it against its own resolved path and the old golden recorded a successful accept.

`context-lowercase-product-name` runs only on case-insensitive hosts
(`platforms: ['darwin', 'win32']` in the case): `product.md` is found through
the canonical name there and through the fallback scan elsewhere, both right.

## Recorded 2026-09-03: #710 resolves an explicit target at its own git boundary

Upstream `672ca296` (#710) scopes an explicit `--target` to its own repository.
A route-shaped target that begins with `/` is an absolute path outside the
workspace, so route cases that used to resolve inside the fixture now resolve
against the filesystem root. Every case below was re-recorded after confirming
`origin/main`'s `context.mjs` / `surface-brief.mjs` produce the same stdout and
the same exit code for the same run.

- `context-full-target-route`, `surface-brief-path-slash`, `surface-brief-path-outside`, `surface-brief-read-route`: stdout and exit code match origin/main byte for byte; nothing here is a delta beyond the upstream change itself.
- `surface-brief-write-route`: the write now fails on both engines (exit 1) because `/.impeccable/surfaces` is not writable. Node reports `ENOENT: no such file or directory, mkdir '/.impeccable/surfaces'`; the engine reports the failed write as `No such file or directory (os error 2)`. Same failure, different wording for an unwritable filesystem root.

## Recorded 2026-09-03: the OpenCode pinned command names the launcher

Upstream `9736a9f6` (#483) makes `pin` write an OpenCode slash-command bridge
whose body tells the agent to run `node <skill-base-dir>/scripts/context.mjs`.
The engine names its own command everywhere else the launcher replaced a
script path (see the 2026-08-17 section above), so the bridge says
`<skill-base-dir>/scripts/impeccable context` instead. Nothing else in the
file, the file set, or the printed lines differs from the JS.

- `pin-opencode-project`, `pin-opencode-user-scope`, `pin-opencode-skips-foreign-command`, `pin-opencode-then-unpin`, `pin-opencode-unpin-skips-foreign`.


## Recorded 2026-09-04: `--version` follows the npm package to 4.0.0

The npm shim answers `--version` / `-v` itself from its own `package.json`
(docs/CLI-CONTRACT.md), so the number users see tracks the package they
installed. The binary's `CLI_VERSION` moves from `3.6.0` to `4.0.0` with the
CLI 4.0.0 release; it is what the binary prints when run directly.

- `cli-version`.

## Recorded 2026-09-12: links and spans are scored for text contrast

The SAFE_TAGS gate in `check_colors` skipped every `a`, `span`, `li`, `td`,
`label` and `button` that did not paint its own background, so a page's links,
nav labels, table cells and small print went unscored while the heading above
them in the same colour was reported. The gate now lets the WCAG contrast
verdict through for a SAFE_TAGS element that paints reading text of its own:
direct text that is not an icon glyph or emoji, at least 9px (the floor the
styled control path already used), not visually hidden, not inside a disabled
control, on the page's own width, and in a colour that no text-bearing
ancestor on the same surface already carries, so an inherited run stays its
paragraph's single finding. `gray-on-color` and the class-list heuristics
(gradient-text, ai-color-palette) stay behind the tag gate.

Two things bound what this can print. A background the walk resolves to the
text colour itself is dropped, because a `1.0:1 — text #ffffff on #ffffff` is
the walk seeing through an image or a video to the page's own fill, never a
real report. And each page reports one colour pair from this path once: a nav
of fifty links in one washed-out colour is one finding on the first link, not
fifty identical lines. The dedupe is scoped to this path, so no finding that
predates the change moves.

Each golden below was read by hand.

- `detect-fixture-json-color-html`, `detect-fixture-text-color-html`: +1, the `.inline-link-low` anchor, `#aaaaaa` on `#fafafa` at 2.2:1. The fixture's note that plain inline links "must remain skipped" was written for the old gate and is rewritten in the same commit; the sub-9px `.chip-sub9-low` chip stays exempt.
- `detect-fixture-json-overlay-positioning-html`, `detect-fixture-text-overlay-positioning-html`: +1. The panel has three `.tiny` spans in `#374151` on `#1f2937` at 1.4:1; they are one colour on one surface, so they report once.
- `detect-fixture-json-link-text-contrast-html`, `detect-fixture-text-link-text-contrast-html`: new fixture, nine findings, all from the should-flag column (accent link, footer span, badge label inside a filled anchor, list item, table cell, ghost button, form label, a paragraph whose inner run stays silent, and a nav of eight links in one colour reporting once). The should-pass column is finding-free.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the sweep total moves from 419 to 430, which is the two findings above plus the nine the new fixture carries.

`legitimate-borders.html` is a negative control whose golden is an empty
result. Its trial-banner link was `#d97706` on the banner's own `#fffbeb` at
3.1:1, a real failure that the old gate hid; the link is darkened to `#92400e`
(6.8:1) in the same commit so the fixture keeps being a clean baseline and its
goldens do not move.

### Revision: markup the browser never renders

Widening the gate widened what the static engine can reach. The browser scan
sees a layout tree, so a `<template>`'s content and a `[hidden]` panel are
simply absent from it; the static tree carries both, html5ever hands template
content back as ordinary descendants, and the static cascade has no UA
stylesheet to turn `hidden` into `display: none`. The colour rule was
therefore able to report a washed-out link in markup nothing paints, and only
in this one engine.

`check_element_colors` now returns early on `is_in_non_rendered_markup`: a
`<template>`, `<noscript>` or `<head>` ancestor, or the `hidden` attribute, on
the element or within twelve parents of it. It covers every tag the colour
rule walks, not only the newly gated ones. `tiny-text` and `undersized-ui-text`
keep the element-local `is_non_rendered_text` they have always used; this is a
second gate beside that one, not a replacement for it, and the two do not read
the same facts.

Every fact it reads is viewport-independent, which is the correction this
revision makes to its first draft. That draft also stood an element down for a
winning `display: none`, and the static cascade descends `@media` blocks
unconditionally, so `@media (max-width: 900px) { .desktop-only { display:
none } }` deleted the whole subtree from the colour rule at every width,
coverage this engine had before the branch. `display` is out of the gate, and
a should-flag case in the fixture (`.flag-desktop-only-row`, a link inside a
row a `max-width` query collapses) plus
`a_media_query_never_hides_anything_from_the_contrast_pass` keep it out.
`hidden="until-found"` is now treated like plain `hidden`: that content is laid
out with `content-visibility: hidden` until find-in-page reveals it, so a
browser measures a zero-size rect and reports nothing there either.

- `detect-fixture-json-link-text-contrast-html`, `detect-fixture-text-link-text-contrast-html`: +1, the desktop-only row's link, `#8e8e8e` on `#ffffff` at 3.3:1. The should-pass column gains a `[hidden=until-found]` subtree and a `<noscript>`, both silent, and loses the `display: none` case, which is now the should-flag one above.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the sweep total moves from 430 to 431, which is that one finding.

### Revision: one report of a colour pair goes to an element that prints it

The per-page dedupe registered a colour pair when the hit was made, and both
engines filter inline `data-impeccable-ignore` afterwards, so a single
`data-impeccable-ignore="low-contrast"` on the first of fifty identical links
waived the whole page's report of that colour. `check_colors_deduped` now takes
the engine's own verdict on a hit and registers the pair only for hits that
survive it. Both engines pass their inline-ignore filter; the browser engine
adds the wrong-layer test below. No golden moves: no fixture waives a
SAFE_TAGS contrast finding.

### Revision: text over a picture is not scored against the fill behind it

The background walk reads the ancestor chain, so it is blind to a positioned
sibling (a hero photo, a video, a canvas), and it answers with the first
background colour it can parse even when an ancestor paints a raster image
over that colour. Both answers are a surface no reader sees, which is how
white label text over a photograph was reported at 1.1:1 against the page fill
and donckelektro.nl's accent orange was reported at 2.5:1 on a section grey it
measures 7.7:1 against in the rendered page.

A hit from the SAFE_TAGS text path is now dropped when
`collect_visual_contrast_reasons`, the visual-contrast pass's own candidate
helper and not a second walk, says a media layer paints behind the text: an
ancestor's raster background, or an `img`, `picture`, `video` or `canvas` in
the hit-test stack under it. A gradient ancestor is not in that set; it is
scored against its stops as before. Those elements are not handed to the
visual-contrast pass as candidates, because that pass takes the first twelve
candidates in document order and a page's links outnumber its headings by an
order of magnitude; widening it is its own change with its own measurement.

The same path also stands down where a transparent `-webkit-text-fill-color`
says the glyphs are not painted in `color` at all, which is how a gradient
heading is written. That guard is browser-only: the static cascade drops the
property, and a recorded call vector pins it dropping it.

No golden moves for either: no fixture puts SAFE_TAGS text over an image or
fills text with nothing.

The corpus numbers this revision first recorded (434 added) are superseded by
the next revision, which replaced the hit-test gate with a geometric one; see
there for what now holds.

### Revision: picture, surface and markup, measured the way a reader meets them

A review of the revision above found four more places the colour rule scored
text a reader does not see, or dropped text a reader does.

**Gradient-clipped runs, static engine.** Where a parent clips a gradient to
its text and the words sit in `<span>`s, the span is not `background-clip:
text` itself, and the static cascade drops `-webkit-text-fill-color`, so the
span was scored on its declared colour against the stops:
`<p class="wordsplit"><span>Split</span> <span>word</span></p>` reported
`1.2:1, text #ffffff on #fde68a`. The SAFE_TAGS text path now stands down
where an ancestor within twelve parents clips its background to text,
stopping at an ancestor with an opaque background of its own, which is a
real surface inside the clipped box. The cascade does carry the clip. The
browser engine asks the same question, so the two engines agree where the
fill is opaque, too.

**The wrong-layer gate, at any scroll position.** The gate read the
visual-contrast collector's hit tests, which skip every point below the
viewport, so an orange link over a dark photo at y 1600 reported
`2.7:1, text #f37b2e on #ffffff` while the same link above the fold did not.
`media_layer_under_text` is now its own geometric test and no longer calls
the collector. It climbs from the element, and at each level asks, in paint
order, the box's own background and then its earlier siblings (and a few
levels of their descendants) whose rect covers the text rect. An `img`,
`picture`, `video` or `canvas` there, or a raster background, is a picture
under the text. Bounds: 32 levels, 32 siblings per level, 3 levels and 8
children into a covering sibling. Only a page the climb cannot decide (a
transparent document, or a tree past those bounds) falls back to hit tests.

**Opaque surfaces between the picture and the text.** The first opaque
background met on the way, ancestor or covering sibling, ends the test with
no picture: a `#999` link on a white card over a hero photo is scored on the
card, as it should be. The hit-test fallback stops at an opaque box the same
way. A solid colour carrying a raster texture is a surface where the image
is a small repeating tile: not `no-repeat`, not `cover`, `contain` or a
percentage, every size component `auto` or at most 256px. The tile's pixels
are not in the computed style, so "faint" cannot be measured, and a
photograph drawn in tile shape (an auto-sized, repeating hero with no
`background-size`) is now scored against its section colour. An unknown
size keeps the quiet answer.

**The static non-rendered gate, reading markup.** An author `display` on a
`hidden` element beats the UA's `[hidden] { display: none }`, so
`<div hidden class="reveal">` with `.reveal { display: block }` renders and
is scored again; `hidden="until-found"` stays hidden, because no `display`
undoes it. The panel of a closed `<details>`, everything but its first
`<summary>`, is not rendered. `map` leaves `NON_RENDERED_TAGS`: a `<map>` is
an inline box and its flow content renders, only `<area>` paints nothing.
That constant also serves `tiny-text` and `undersized-ui-text`, and no golden
moves for them. The walk now goes to the root, because a `<template>` twenty
levels up hides an element as surely as its parent does.

The per-page dedupe also checks a hit against the pairs already reported
before it asks the engine for its verdict, so a duplicate link the dedupe
would drop anyway costs no layer walk.

- `detect-fixture-json-link-text-contrast-html`, `detect-fixture-text-link-text-contrast-html`: 10 to 16. Four should-flag cases join: a link on a white card over a hero photo (`#939393`, 3.1:1), a link on a white section with a texture tile (`#969696`, 3.0:1), a paragraph in a `[hidden]` panel author CSS reveals (`#8c8c8c`, 3.4:1), and a link inside a `<map>` (`#919191`, 3.2:1). The should-pass column gains a link in a closed `<details>`, a link fourteen levels inside a `<template>`, and the review's two gradient-clipped runs, none of which reports a contrast finding. The two gradient-clipped parents report `gradient-text`, which is that rule's verdict and the only non-contrast finding in the fixture.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the sweep total moves from 431 to 437, which is those six findings.

**Corpus, run 2, 346 captures.** Removed 0, added 423, violations 0 (434
before this revision). The geometric test reads the rects a capture already
carries, so in replay it decides a page without the hit-test facts run 2 did
not record; the per-site effect on the clusters above was not re-measured
for this revision. The review's repro pages, scanned live from a local
server: the orange links over a dark photo above and below the
fold both report nothing, the `#999` link on the white card reports
`2.8:1, text #999999 on #ffffff`, and the textured section's link reports
`2.7:1, text #9d9d9d on #ffffff`.

### Risks carried, not fixed

- A background that resolves to the text colour itself is dropped, which also
  drops text genuinely painted in its own background: invisible, and a real
  1:1 failure. It is exact hex equality, so a link one shade off its surface
  still reports, and the guard covers only the SAFE_TAGS text path, so `<p>`
  and `<div>` still report the `1.0:1`. Written into
  `resolved_bg_matches_text`'s doc comment beside the code.
- The wrong-layer test is the browser path's. The static engine has no layout
  and no hit testing, so a link over a positioned photo is still scored
  against whatever the ancestor walk resolved there.
- The ratio label can still name the wrong surface where nothing media-shaped
  is involved: an opaque sibling panel that is not an ancestor is not in the
  set the test reads. The real fix for all of it is a resolved background that
  carries where it came from, which is not cheap from `ColorOpts` as it
  stands.
