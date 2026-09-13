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

## Recorded 2026-09-12: the URL scan reads the page after the reveal sweep

`crates/browser` now runs the reveal sweep before it captures the page, and
every deterministic pass reads that one post-reveal capture. The new fixture
`tests/fixtures/antipatterns/scroll-reveal.html` is what holds that order: its
left column carries faults a pre-reveal pass cannot see (a section at opacity 0
skips the element checks), its right column carries the fade-in a pre-reveal
pass reports as `buried-raster`. URL scans have no goldens, so the fixture is
pinned by `crates/browser/tests/evidence.rs`; the goldens below move only
because a file was added to the fixture directory the static engine walks.

- `detect-fixture-json-scroll-reveal-html`, `detect-fixture-text-scroll-reveal-html`: new cases. The static engine has no reveal to run, so it reports the fade-in from the stylesheet; that is its correct reading of the source and the fixture says so in a comment.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-scope-type`, `detect-scope-both`, `detect-no-advisory-json`, `detect-no-advisory-text`: the four new findings are appended and the total moves from 419 to 423. No existing fixture's findings changed.

## Recorded 2026-09-12: a fixture for the visual-contrast sampling decisions

`tests/fixtures/antipatterns/visual-contrast-sampling.html` is new: the
reduced false positives and protected true positives behind the visual pass's
sampling fix (glass panel, filtered wrapper, transparent gradient stop, a
ten-percent tint of the text's own color, vector avatar paint, gradient-clipped
heading, faded accordion trigger, translucent pill on a pale photo). The static
text scan reads the file like any other fixture; the only rule with an opinion
about it is `gradient-text`, which fires twice on the one `background-clip:
text` heading (the existing duplicate the CSS-text and element forms produce).
The dir-wide goldens gain those two findings and their count moves 419 → 421.
Nothing else in any golden changed: the fix is in the browser passes, which
have no goldens (browser output depends on the machine).

- `detect-dir-text-all-fixtures`, `detect-dir-json-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-text`, `detect-no-advisory-json`, and the two new per-fixture cases `detect-fixture-text-visual-contrast-sampling-html` / `detect-fixture-json-visual-contrast-sampling-html`.


## Recorded 2026-09-12: cramped-padding measures where the glyphs land

Corpus judging (both judges, 88 of 111 representatives) found the rule reading
the `padding` property and the child's border box rather than the text. A
40px flex row centres a 14px label on zero padding; an accordion row's inset
lives on a button two levels down; a collapsed panel and a screen-reader
heading paint nothing at all. The rule now decides on `getDirectTextRect`, the
union of an element's own text-node rects clamped to the box that paints them,
and ignores a transparent border and a white box on an unpainted light canvas.
The file scan has no layout to measure, so it follows the padding down instead:
an element holding one element and no text of its own hands the question to
what it wraps, which is how `wrapper > h3 > button` and `panel > div > p` now
read.

Every golden below was re-recorded from the binary and reviewed by hand; none
is exempted from comparison, so the oracle still pins each one.

Goldens that moved, and why:

* `detect-fixture-json-flush-against-border-html` and
  `detect-fixture-text-flush-against-border-html`: 6 findings to 11. The
  fixture grew five reduced cases from the corpus false positives plus
  `flag-overrun-field`, the shape both judges called harmful. A URL scan of
  the fixture reports the six `flag-` cases and nothing else; the file scan
  adds the five pass cases it has no layout to clear, listed by name in the
  fixture header and pinned in `crates/html/tests/static_flush.rs`.
* `detect-fixture-json-edge-flush-cards-html` and
  `detect-fixture-text-edge-flush-cards-html`: 3 findings to 0. Each was a
  `.scroller` whose cards carry 12px of padding one level below its own
  child, so the text was never near the edge. A URL scan of that fixture
  reports no cramped-padding finding either, before or after this change.
* `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures`, `detect-scope-layout-text`,
  `detect-scope-both`, `detect-no-advisory-json`, `detect-no-advisory-text`:
  the sweeps carry the same two files, 419 findings to 421. No other rule's
  output moves.

## Recorded 2026-09-12: `ai-color-palette.html` joins the fixture directory

`ai-color-palette` gained the evidence gates that separate a painted
violet-to-cyan palette from a declared one (occluded placeholder gradients,
tints, blurred washes, hairlines, flat repeats of one stop, and `color`
inherited by elements that paint no glyphs). The gates live on the browser
element path, which has no goldens, so the only oracle movement is the new
two-column fixture that documents them.

- `detect-fixture-json-ai-color-palette-html`, `detect-fixture-text-ai-color-palette-html`: new cases for the new fixture.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the directory sweeps pick up the same two findings from the new file (the static engine's own heading-color match, plus `radial-halo` on the violet glow blob), and the count moves 419 to 421. No other fixture's output changed.

A later revision added six more rows to that fixture (a same-color alpha fade,
a `<picture>` around a letterboxed image, a scroll-reveal wrapper, a
typewriter hero, a blurred wrapper, a `visibility: hidden` branch). They
document browser-path gates the static engine never runs, so no golden moved
and nothing was re-recorded.

## Recorded 2026-09-12: `clipped-overflow-container` names the child and drops the clips that do their job

Judging the rule over real sites put its precision at 0.36: most findings were
`overflow: hidden` working as intended (masked reveals, marquee and rail
tracks, image bleeds, ornament layers, boxless wrappers, page shells), and the
snippet never said which child was cut, so a finding could not be checked
without opening the page. The check now names the escaping child, skips
containers that generate no box or that hold the whole page, reads the
carousel and marquee words on the immediate scrolling child, treats a
transform-parked copy that fits the box as a masked reveal, only counts an
inset escape on an axis the container actually clips, and reports one
container per escaping layer instead of every clip in the chain. Menus,
dialogs, tooltips and popovers keep their findings.

The container it reports is the clip nearest the layer, which is the one that
cuts the layer first and the one whose component the layer belongs to.
Measured over real pages, the outermost clip is usually a page or app wrapper
that would absorb every layer beneath it and name none of the components that
own them. `html` and `body` report nothing at all: `overflow: hidden` there is
the standard guard against sideways scrolling, and the browser engine has
never scanned either.

- `detect-fixture-json-clipped-overflow-container-html`: the six existing findings now name their child; six cases added to the fixture flag column are reported (a ribbon above a card, a tooltip in a rail, two rows inside one clipping shell that each keep their own finding, a transform-parked menu, an empty menu layer); the pass column grew by the new exemptions, the shell around the two rows among them, and reports none of them.
- `detect-fixture-text-clipped-overflow-container-html`: same, in the text renderer.
- `detect-fixture-json-overlay-positioning-html`: the one finding there now names its child (`div clips positioned div`).
- `detect-fixture-text-overlay-positioning-html`: same, in the text renderer.
- `detect-dir-json-all-fixtures`: the directory sweep carries the same snippet change and the six new fixture findings (419 -> 425).
- `detect-dir-text-all-fixtures`: same, in the text renderer.
- `detect-dir-quiet-all-fixtures`: same, as the count line only.
- `detect-scope-layout-text`: same, scoped to the layout rules.
- `detect-scope-both`: same, over both scopes.
- `detect-no-advisory-json`: same, with advisories off.
- `detect-no-advisory-text`: same, in the text renderer.
## Recorded 2026-09-12: the tight-leading floor only measures body copy

Reviewing the rule's findings on real pages showed the 1.3 leading floor being
applied to type it was never written for: display sizes and heading text set on
`p` / `div` / `span` (the heading exemption was a tag test, so it missed the
heading text that sits in a child `<a>` or `<span>`), text that renders a
single line, source text nothing typesets (`<script>`, `<style>`, `<noscript>`,
head content, `display:none`, the screen-reader clip patterns), and pages that
set `line-height: 1.3` exactly, where the float division lands just under the
floor. The check now carries those carve-outs in both engines; the wrap test
needs layout, so it is browser-only.

- `detect-fixture-json-tight-leading-html`, `detect-fixture-text-tight-leading-html`: new fixture, two columns of real cases reduced from the reviewed pages.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-scope-type`, `detect-scope-both`, `detect-no-advisory-json`, `detect-no-advisory-text`: the directory sweep picks up the new fixture. The additions are its six findings and the total moves from 419 to 425; no existing fixture's findings changed.
## Recorded 2026-09-12: layout-transition and bounce-easing go advisory, image-hover-transform retires

A corpus review of 50 real sites judged all three rules on their findings. Two
measure exactly what they claim and are almost never harmful where they fire
(`layout-transition` 1,435 findings on 37 sites, harmful in 9% of the judged
representatives; `bounce-easing` 160 findings on 13 sites, harmful in none), so
their registry severity is now `advisory`: still detected and listed, never
counted, never in the exit code. `image-hover-transform` is retired outright:
hover zoom on a card image is a long-standing convention rather than a
generated-UI tell, and it fired on mobile captures where hover cannot happen.
Its fixture (`tests/fixtures/antipatterns/gemini-tells.html`) and the two cases
generated from it are gone; the real-world hover-zoom constructions moved into
`motion.html`'s should-pass column, where they now produce nothing.

- `detect-fixture-json-motion-html`, `detect-fixture-text-motion-html`, `detect-fixture-json-multifile`, `detect-fixture-text-multifile`, `detect-multifile-json`, `detect-multifile-text`, `detect-fixture-json-linked-url-patterns-css`, `detect-fixture-text-linked-url-patterns-css`, `detect-fixture-json-jsx-should-flag-jsx`, `detect-fixture-text-jsx-should-flag-jsx`, `detect-fixture-json-vue-should-flag-vue`, `detect-fixture-text-vue-should-flag-vue`, `detect-fixture-json-svelte-should-flag-svelte`, `detect-fixture-text-svelte-should-flag-svelte`, `detect-fixture-json-cssinjs-should-flag-tsx`, `detect-fixture-text-cssinjs-should-flag-tsx`, `detect-fixture-json-framework-next-modules`, `detect-fixture-text-framework-next-modules`, `detect-fixture-json-framework-next-tailwind`, `detect-fixture-text-framework-next-tailwind`, `detect-fixture-json-framework-next-cssinjs`, `detect-fixture-text-framework-next-cssinjs`, `detect-framework-next-modules-text`, `detect-framework-next-tailwind-json`, `detect-framework-next-cssinjs-json`: the same findings, now carrying `severity: "advisory"` / `advisory: true` and printed under the advisory heading instead of the counted list.
- `detect-config-css-json`, `detect-config-css-text`: the workspace's only counted finding was a bounce-easing hit, so the scan reports `0 anti-patterns found.` and exits 0 instead of 2. The finding itself is still printed, as an advisory note.
- `detect-config-dir-json`, `detect-config-dir-text`, `detect-config-dir-dot`: same reclassification inside a dir scan.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the fixture sweep loses the two `image-hover-transform` findings with the fixture (436 → 434 findings) and moves 28 (15 layout-transition, 13 bounce-easing) out of the failure count (419 → 391). `--no-advisory` drops all 43 advisory findings, as it always has.
- `hook-config-per-edit-all`: the design hook defaults to `advisoryRules: "exclude"`, so the edited `Card.tsx`, whose only finding was bounce-easing, is now reported clean and the hook's message covers one file instead of two. Setting `advisoryRules: "include"` restores the old report.

The frozen call vectors keep the retired rule's recorded hits, since
`tests/oracle/vectors/calls/` can never be re-recorded. `crates/core/tests/vectors.rs`
drops findings carrying a retired id from both sides of the comparison
instead; every other hit on those lines still has to match.
## Recorded 2026-09-12: `extreme-negative-tracking` gets a size-scaled threshold and a CJK exemption

Corpus run 2 judged 26 representatives of this rule and found no harm in any of
them: -0.05em is exactly Tailwind's tracking-tighter and the tracking several
display faces recommend, so the old `<= -0.05em` line fired on ordinary display
type and on whole sites that set one utility class. The rule now flags below
-0.07em, and below -0.09em for text at 40px or larger, and it skips text whose
glyphs are CJK (Han, Hiragana, Katakana, Hangul), read from the element text
rather than a lang attribute. The snippet gained the font size the em value was
measured against.

The fixture was rewritten around the new lines (px values, since the static
engine resolves an `em` letter-spacing against the inherited font size), so the
three flagged rows change text and the pass column grew. No finding counts
change in any case below.

- `detect-fixture-json-extreme-negative-tracking-html`, `detect-fixture-text-extreme-negative-tracking-html`: the three flagged rows carry the new snippet form and the fixture's new values.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`, `detect-scope-type`, `detect-scope-both`: the same three lines inside the sweeps.
