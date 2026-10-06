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

## Recorded 2026-09-12: `wide-tracking` spares short labels typed in capitals

Corpus run 2 judged 70 `wide-tracking` findings across nine sites; the hits
both judges called harmless were eyebrows, badges and buttons, several of them
typed in capitals in the markup. The rule exempted `text-transform: uppercase`
only, so a label spelled `LIMITED EDITION RELEASE 2026` was measured against
the body-text threshold. It now also exempts a run that is already all
capitals when it is at most 40 characters and does not wrap; running text and
mixed-case labels are unchanged.

No existing fixture's output moved. The new
`tests/fixtures/antipatterns/wide-tracking.html` adds four findings (three
`wide-tracking` in the flag column, one `all-caps-body`), which is what these
goldens re-record.

- `detect-fixture-json-wide-tracking-html`, `detect-fixture-text-wide-tracking-html` (new cases), `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-scope-type`, `detect-scope-both`, `detect-no-advisory-json`, `detect-no-advisory-text`.

## Recorded 2026-09-12: `all-caps-body` needs an 80-character run of its own

Judged against real sites, every `all-caps-body` hit on the corpus was a short
label: a card CTA, a section kicker, an eyebrow, a diagram legend, a footer
copyright line. Both judges called all 26 representatives harmless, and none
of the 109 findings across 14 sites was a caps paragraph. Uppercase on a run
the reader takes in as a shape costs nothing, so the rule now fires only from
80 characters, where a run is read as a sentence. Those labels reach 71
characters on the corpus, which is where the floor comes from.

The length is the element's own text rather than its subtree, so a bar or a
form control whose children hold the labels is no longer charged for their
sum; both engines apply the same test, and a run's verdict no longer depends
on the viewport it was measured in.

- `detect-fixture-json-hero-eyebrow-chip-html`, `detect-fixture-text-hero-eyebrow-chip-html`: the 46-char uppercase table-of-contents label no longer flags. Its `hero-eyebrow-chip` finding is unchanged.
- `detect-fixture-json-overlay-positioning-html`, `detect-fixture-text-overlay-positioning-html`: sixteen overlay captions of 31-52 characters no longer flag.
- `detect-fixture-json-text-occlusion-html`, `detect-fixture-text-text-occlusion-html`: the 32-char kicker no longer flags; `kicker-above-heading` still owns it.
- `detect-fixture-json-quality-html`, `detect-fixture-text-quality-html`, `detect-fixture-json-typography-html`, `detect-fixture-text-typography-html`: the 285-char and 159-char caps paragraphs still flag, with the same counts, since each is one element's own run; only the rule description moved.
- `detect-fixture-json-all-caps-body-html`, `detect-fixture-text-all-caps-body-html`: new fixture, three caps paragraphs flagged (162, 140 and 159 chars) and seven short caps runs silent, including a bar and a form label whose subtrees pass 80 characters.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-scope-type`, `detect-scope-both`, `detect-no-advisory-json`, `detect-no-advisory-text`: the sweep loses those eighteen findings and gains the new fixture's three (436 to 421, `all-caps-body` 20 to 5).

## Recorded 2026-09-12: `justified-text` narrows to narrow columns in word-spaced scripts

Judging the rule's findings on real sites found 947 of them on four sites,
nearly all of them CJK news and marketing pages where justification is correct
typography: characters are uniform width, there are no word spaces to stretch,
and `hyphens: auto` is not the remedy the finding proposes. Both judges called
every representative harmless. The rule now reads the element's own text and
skips CJK, Thai and Arabic script, where justification sets on a character grid
or elongates glyphs, and for the remaining scripts fires only in a column of
45 characters per line or fewer (the estimate `line-length` reports). The
registry description says so. The static engine reads that measure from the
nearest declared `width`, the only width its cascade carries; with none
declared it has no measure and does not fire.

- `detect-fixture-json-quality-html`, `detect-fixture-text-quality-html`,
  `detect-fixture-json-typography-html`, `detect-fixture-text-typography-html`:
  the finding is unchanged; only the rule description moved. Both fixtures now
  declare the flagged column's width in pixels so the case states the measure
  it is about.
- `detect-fixture-json-justified-text-html`,
  `detect-fixture-text-justified-text-html`: new fixture. Three should-flag
  cases (a 300px column, a 260px column with `hyphens: manual`, a 240px
  sidebar) and five should-pass ones (a 760px measure, `hyphens: auto`, and
  Chinese, Thai and Arabic paragraphs at 300px).
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures`, `detect-scope-type`, `detect-scope-both`,
  `detect-no-advisory-json`, `detect-no-advisory-text`: the sweep picks up the
  new fixture's three findings (419 to 422) and the new description. Nothing
  else moved in any of them.

## Recorded 2026-09-12: dark-glow gains a perceptibility floor

A glow layer now has to put out light a reader can see before it is reported:
its blur radius times the shadow's alpha times the element's own opacity must
reach 3px, a negative spread that swallows the blur suppresses it, and where
layout is known the lit ring may not cover more than twice the element's own
area. The floors come from the site corpus: every glow both judges could find
in the screenshot scores 3.0 or more and stays within 1.8x its element, while
the ones they called invisible top out at 2.1 and the indicator lights (a 3px
typing caret, a 6px status LED, a pulse travelling a connector) start at 2.2x.
A layer under the floor is passed over rather than ending the scan, so a
stacked elevation ramp is still reported from the layer that carries the light.

The glow fixture grew four cases: two flag cases above the floor (a 197x40 CTA
with a 24px halo, a 96x96 tile under a six-layer ramp) and two pass cases whose
halo is out of scale with a tiny element (a 6x6px status LED, a 5x8px pulse).
The file scan has no layout, so it keeps reporting those last two; the browser
engine, which does, drops them. The three pass cases that turn on alpha,
spread, and opacity (a half-faded typing caret, a 10%-alpha wash, a spread that
eats its blur) are dropped by every engine and add no findings anywhere.

- `detect-fixture-json-glow-html`, `detect-fixture-text-glow-html`: the four
  new fixture findings; nothing the old fixture reported was lost.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures` (419 to 423), `detect-no-advisory-json`,
  `detect-no-advisory-text`: the same four findings in the directory sweeps.

## Recorded 2026-09-13: the integration of the branches above

`corpus/integration` merges every branch whose entry appears above. Where two
branches moved the same golden, neither side's recording describes the merged
engine, so these cases were re-recorded from the integrated binary. Each was
checked against the union of the entries above: the directory sweep equals the
base findings plus every branch's additions minus every branch's removals, key
for key, with nothing extra and nothing missing (436 findings to 452; 419
counted to 409 with advisories off). Two findings moved only because one
branch's registry text reached another branch's golden: the `all-caps-body`
finding in `wide-tracking.html` carries the 80-character description, and the
`justified-text` findings in `quality.html` and `typography.html` carry the
narrow-column description.

- `detect-fixture-json-overlay-positioning-html`, `detect-fixture-text-overlay-positioning-html`: clipped-overflow's named child plus all-caps-body's sixteen removed captions.
- `detect-fixture-json-quality-html`, `detect-fixture-text-quality-html`, `detect-fixture-json-typography-html`, `detect-fixture-text-typography-html`: both the all-caps-body and justified-text descriptions.
- `detect-fixture-json-wide-tracking-html`, `detect-fixture-text-wide-tracking-html`: the all-caps-body description.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-scope-type`, `detect-scope-layout-text`, `detect-scope-both`, `detect-no-advisory-json`, `detect-no-advisory-text`: the union of every sweep delta above.

## Recorded 2026-09-12: the radial-spotlight-glow fixture states its surfaces

`radial-spotlight-glow` now asks how prominent a declared glow is: bright
against the surface it paints on (a contrast of 1.30 between the glow's peak
and that surface), not scaled away by the element's opacity (an effective
alpha of 0.14), and behind copy. The fixture had to declare those things, so
every case gained a ground color and a heading.

Three changes show up in the goldens.

The hex-alpha case moved from `#506fff3d` to `#506fff66`. It was testing
8-digit hex parsing, and at alpha 0.24 that blue no longer clears the contrast
line against the fixture's `#0b0d13` ground, so it would have been testing the
threshold instead. Alpha 0.40 keeps it on the parser. The pair pins the rule's
practical firing floor for a mid blue on a near-black ground between 0.24 and
0.26, which is where `.flag-hero-blue` (alpha 0.26) sits.

Three should-flag cases are new, one per gap the review found: a glow over a
hero painted with a gradient, a glow over a hero painted with a photograph,
and a two-stop glow with the bright stop declared second. Two should-pass
cases are new for the same gates: a pale gradient hero that swallows its glow,
and a wash over a photograph. That takes the fixture from 5 flag / 14 pass to
8 flag / 16 pass, and `detect-dir-quiet-all-fixtures` from 419 findings to
422.

The registry description changed. It opened by calling the gradient soft and
low-opacity, which describes what the old declaration test matched rather than
what now fires, so it names the brightness instead.

- `detect-fixture-json-radial-spotlight-glow-html`, `detect-fixture-text-radial-spotlight-glow-html`, `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`.

## Recorded 2026-09-13: radial-spotlight-glow measures every stop and every layer

Review of the prominence gate found two ways it went silent on glows it
exists to catch.

The gate measured one stop, the brightest by luminance, with its alpha
ignored. A pale highlight core (`rgba(255,228,186,0.08)`) over a saturated
ring (`rgba(255,90,0,0.40)`) measured the core, failed, and hid the ring. The
same hue at two alphas flagged or not depending on which alpha was declared
first. The adapters now test every chromatic stop, and the finding names the
stop that passed with the most contrast. The pure `checkRadialSpotlight`
snippet still names the first chromatic stop, so the frozen call vectors
replay unchanged.

A translucent gradient anywhere in the backdrop, including a faint fade to
`transparent`, made the surface unreadable, which switched the contrast test
off, so a pastel wash in a hero with a decorative fade flagged. Translucent
gradient layers are now composited, as the alpha-weighted mean of their stops,
over whatever resolves beneath them. Only an image that shows through still
skips the test. The glow element's own layers beneath the glow count as the
surface too.

The fixture gains three should-flag cases (Saturated Ring Under Pale Core,
Weak Stop Declared First, Glow Under A Dark Fade) and two should-pass cases
(Pastel Under A Faint Layer, Pastel Over Its Own Pale Layer). That takes it
from 8 flag / 16 pass to 11 flag / 18 pass. The two new pass cases would have
flagged under the previous revision: the first because the fade made the
surface unreadable, the second because the dark page was measured instead of
the element's own pale lower layer. Every golden change is one of the three
new findings: the fixture goes from 9 findings to 12, `detect-dir-json-all-fixtures`
from 439 to 442, and `detect-dir-quiet-all-fixtures` and `detect-no-advisory-json`
from 422 to 425.

- `detect-fixture-json-radial-spotlight-glow-html`, `detect-fixture-text-radial-spotlight-glow-html`, `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`.

## Recorded 2026-09-13: radial-spotlight-glow joins the integration

`corpus/integration` merges `corpus/premise-radial-spotlight-glow`. Both fixture
goldens replay as the branch recorded them. The five directory sweeps moved on
both sides, so they were re-recorded from the integrated binary and checked
against the two entries above, finding for finding: the integration moved by
exactly the branch's own delta, with nothing extra and nothing missing. The
five existing radial-spotlight-glow findings carry the new registry description
(and the hex-alpha case its 0.40 alpha), and six findings are new: the three
should-flag cases from each revision. The sweep goes from 452 findings to 458, and from 409 counted to 415
with advisories off.

- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the radial-spotlight-glow delta on top of the integration above.

## Recorded 2026-09-13: clipped-overflow-container goes advisory

Even after the rule was fixed to name the child and drop the clips that do
their job, the new findings of a live recapture judged 0.08 pattern precision
and 0.04 harm: the clip is almost always the intended effect (carousels,
tickers, accordions, collapsed nav variants, closed video menus). Its registry
severity is now `advisory`, the same move `layout-transition` and
`bounce-easing` made: still detected and listed, never counted, never in the
exit code. It also leaves the design hook's immediate tier, which is reserved
for unambiguous problems worth interrupting an edit for; the hook drops
advisory findings by default, and with `advisoryRules: "include"` the rule now
waits for the Stop deep pass. No finding was added or removed. Every golden
change is the same findings moving from the counted list to the advisory
section.

- `detect-fixture-json-clipped-overflow-container-html`, `detect-fixture-text-clipped-overflow-container-html`: the twelve findings carry `severity: "advisory"` / `advisory: true` and print under the advisory heading; the fixture's only counted finding is the `cramped-padding` hit on `pass-split-container` (13 counted to 1).
- `detect-fixture-json-overlay-positioning-html`, `detect-fixture-text-overlay-positioning-html`: the one clipped finding moves to the advisory section (32 counted to 31); the exit code stays 2 on the other findings.
- `detect-scope-layout-text`, `detect-scope-both`: the same thirteen findings inside the layout-scope sweeps (`detect-scope-both` 166 counted to 153, 170 findings unchanged).
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`: 458 findings unchanged, 415 counted to 402, advisory notes 43 to 56.
- `detect-no-advisory-json`, `detect-no-advisory-text`: `--no-advisory` now drops the thirteen findings with the other advisories (415 to 402).

## Recorded 2026-09-12: the hairline-and-halo pair has to repeat across a row

Corpus judging put `gpt-thin-border-wide-shadow` at 0.70 pattern precision
with no harm found on 19 representatives: a hairline border beside a soft
shadow is the resting card and popover style of most mature design systems.
The rule now reports it only where a row repeats it. Two gates on top of the
pair, both measured on what the element already declares:

- the widest shadow layer drawn outside the box reaches 32px of blur (was
  16px, and an inset layer never counts, since a well pressed into a surface
  is not an elevation under it). Offsets are read by nothing: every step of
  every mainstream elevation scale casts a y-offset, so a shadow lit from
  above is the common case rather than the exception.
- at least three comparable boxes of one row carry the same pair. The row is
  what the layout repeats, not only the element's DOM siblings: the walk
  climbs at most two wrappers, and at each level reads the wrapper's siblings
  of the same tag for a card at the element's own depth below them, so a grid
  whose cells each wrap their card in a link, or a stack of articles each
  holding one panel, is one row. It reads at most 24 siblings on each side per
  level and 32 boxes inside each sibling cell, one child at a time. A browser
  scan compares rects, which also silences an element that paints nothing,
  such as a closed dropdown; a file scan has no layout and asks for the same
  tag plus comparable pixel sizes wherever both boxes declare them.
- every card of that row, the element included, shows at rest. The wrapper
  climb would otherwise read a nav bar whose items each hold a flyout as a
  row of cards, since flyouts laid out ahead of their hover have real sizes
  and carry the pair. A popover waiting for its trigger is closed, or lifted
  out of the flow (absolute or fixed, itself or up to two wrappers up) and
  hidden. A browser scan reads a closed box as one with no area, and reads
  an out-of-flow box as hidden when its computed visibility is hidden, its
  opacity multiplies down to nothing along its ancestors, or its rect sits
  past the page's left or top edge or the viewport's right edge. A file scan
  reads the `hidden` attribute or `display: none` on the box or an ancestor
  as closed, and `visibility: hidden` or a transparent opacity on an
  out-of-flow box as hidden. A transform that parks a box off the page needs
  layout, so only the browser scan reads it. Content staged in the flow for
  a scroll reveal, transparent and offset until the reveal runs, still
  counts: a visitor sees it by scrolling, and the corpus's one row, three
  chart panels on evergrovelabs.com, sits at opacity 0 in the scan snapshot.

The snippet now says which of the two the reader has to act on, so removing
the shadow from the one named card does not read as the whole repair:
`1px border + 40px shadow blur, repeated across the row`. The registry
description names the row for the same reason.

Every golden below was re-recorded from the binary and reviewed by hand, so
none of them is an accepted delta. They are named in prose rather than in the
bullet-then-case-id form this file's header describes, which run.mjs reads as
a standing exception to a golden it would otherwise fail on.

- Re-recorded on `gpt-tells.html`: `detect-fixture-json-gpt-tells-html` and
  `detect-fixture-text-gpt-tells-html` go from 4 findings to 3. The fixture's
  lone hairline card, a 24px halo on one box, now sits in the pass column; the
  rule's own cases moved to the new `gpt-thin-border-wide-shadow.html`.
- New cases for that fixture: `detect-fixture-json-gpt-thin-border-wide-shadow-html`
  and `detect-fixture-text-gpt-thin-border-wide-shadow-html`. Fifteen findings,
  one per card of its five flag rows (a 40px halo, a 48px halo across cards of
  slightly different sizes, a row lit from above at 8px offset under a 40px
  blur, grid cells that each wrap their card in a link, and one panel per
  article two wrappers down with the middle article flipped); its ten pass
  rows (a lone popover, a pair of cards, a tight shadow, a shadow drawn inside
  the box, a border too faint to read, a heavy border, three boxes of one tag
  sized nothing alike, one panel in a stack of articles that hold no panel, a
  flyout per nav item hidden until hovered, a dropdown per nav item closed
  with `display: none`) report nothing. A browser scan of the file, copied
  outside the repo so the root DESIGN.md does not apply, reports the same
  fifteen and nothing else.
- Re-recorded sweeps: `detect-dir-json-all-fixtures`,
  `detect-dir-text-all-fixtures` and `detect-dir-quiet-all-fixtures` carry both
  files, 436 findings to 450 and 17 advisory notes to 31. The counted total
  stays at 419; no other rule's output moves.
- Known limits, merged as leftover noise the base also reports rather than
  regressions: a closed native `<details>` dropdown, and a megamenu whose
  hidden container sits several levels up, still count as a row; the file
  scan still flags repeated closed `<dialog>` and `[popover]` elements; a
  flyout closed by clipping still counts.

## Recorded 2026-09-13: gpt-thin-border-wide-shadow joins the integration

`corpus/integration` merges `corpus/premise-gpt-thin-border`. The branch's four
fixture goldens replay as it recorded them. The three directory sweeps
(`detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`,
`detect-dir-quiet-all-fixtures`) moved on both sides, so they were re-recorded
from the integrated binary and checked against the entry above, finding for
finding: the integration moved by exactly the branch's own delta, fifteen
findings added from the new fixture and the one `gpt-tells.html` finding
removed, with nothing extra and nothing missing. The sweep goes from 458
findings to 472 and from 56 advisory notes to 70; the counted total stays at
402. The `--no-advisory` and scope sweeps replay unchanged, since the rule's
findings are advisory. Named in prose for the same reason as the entry above.

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
- `detect-fixture-json-link-text-contrast-html`, `detect-fixture-text-link-text-contrast-html`: new fixture, nine findings, all from the should-flag column (accent link, footer span, badge label inside a filled anchor, list item, table cell, ghost button, form label, a paragraph whose inner run stays silent, and a nav of eight links in one colour reporting once). The should-pass column is finding-free in the static engine, which is what these goldens record; a browser scan reports two of its links (see "Known limits at merge" below).
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
`hidden="until-found"` is now treated like plain `hidden` in the static engine:
that content is laid out with `content-visibility: hidden` until find-in-page
reveals it. The browser engine does not stand down there: the snapshot keeps a
box for the link, and a browser scan reports it (see "Known limits at merge"
below).

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
`2.7:1, text #9d9d9d on #ffffff`. These numbers are superseded by the next
revision.

### Revision: the layer under a link, past the page's own fill

A review of the revision above found the geometric climb treating an opaque
`body` or `html` as the surface, so the hit-test fallback only ever ran for a
fully transparent document. A photo laid after the content at
`z-index: -1` reported `2.7:1, text #f37b2e on #ffffff`, and a photo inside
zero-height wrapper divs reported `2.6:1, text #f58030 on #ffffff`, where
revision 3 had been silent on both. Across the corpus, 56 of 395 added
findings inside the screenshot sat on pixels more than 60 away from the
background they named, and 9 of 10 cropped named a surface that was not under
the text: text over photos (thairath.co.th, zigzag.kr), a dark hero behind a
transparent header (aisupply.framer.website), white labels on green buttons
reported on pink or near-white (bt.cn). A second finding: a link or its list
item carrying a small icon image was dropped as if it sat over a photo.

**What the browser test answers.** `media_layer_under_text` is replaced by
`layer_under_text`, which names what paints under the text instead of
answering yes or no: a picture, an opaque surface the background walk never
read (`Detached`, with its colour), paint the walk cannot turn into a colour
(`Unmodelled`), the ancestor fill the walk answers with (`Ancestor`), or
nothing decided. `resolved_surface_is_under_text` keeps a hit on the SAFE_TAGS
text path for `Ancestor` and for nothing decided, and for a `Detached` surface
only where its colour sits within 24 (summed over the three channels) of the
background the walk resolved. Where the snapshot cannot say what is under the
text, this path stays silent rather than name a surface a reader does not see.
No candidate is handed to the visual-contrast pass; that pass's budget is
unchanged.

**The climb.** At each level it reads the box's own paint first:

- a `::before` or `::after` that is absolutely or fixed positioned, painted,
  and at least the size of the text: a raster image is a picture, a gradient
  or a translucent colour is unmodelled, an opaque colour is a detached
  surface;
- a raster background that is neither an icon (below) nor a texture tile of a
  stated size, which is a picture;
- a gradient drawn larger than its box (`background-size` above 100%, or in
  pixels larger than the box), which shows one slice of its stops at a time
  and is unmodelled: the animated `200% 200%` install button on bt.cn;
- an opaque fill, which ends the climb as `Ancestor`.

Then it reads the siblings that paint beneath the text, topmost first, on a
coarse stacking scale: negative `z-index`, then in-flow boxes, then positioned
boxes at 0, then positive `z-index`, where `z-index` counts for a positioned
box or a flex or grid item and an opacity below 1 or a transform opens a
context at 0. A later sibling counts only on a strictly lower layer than the
text (the `z-index: -1` photo, the section under a `z-index: 1` header). An
earlier sibling counts unless it opens a positive `z-index` above the text,
because a positioned `z-index: 0` media box before in-flow text is, on real
pages, under that text (microsoft.com's store cards). Inside a sibling the
test descends where the sibling covers the text or lets its children overflow
(`display: contents` clips nothing), and below that only into children that
cover the text, have no size, or use `display: contents`, so a zero-height
wrapper, a carousel track narrower than its slides and a `display: contents`
section are looked inside while off-screen slides are skipped. A box a few
levels in that covers the text decides it. Bounds: 32 levels, 32 siblings
each side, depth 6, 64 children per box, 1024 nodes per test.

**The opaque document.** Where the climb reaches an opaque `body` or `html`,
or a transparent document, the hit-test stack answers, read down to the first
opaque box: a picture needs every answered point (a run half over a photo and
half over the page is scored on the page, which that half does fail), a
detached or unmodelled answer at any point stands, and otherwise the page
fill stands. Hit tests can only be asked for points inside the viewport, so
this runs in a live scan above the fold. In a replayed capture it answers
only the points the capture recorded, and below the fold, or for an
unrecorded point, the page fill stands.

**Icons and textures.** A background image on the text's own element or its
nearest `li` is an icon, not a picture, where it is one `no-repeat` image of a
size the style states at most 32px on both axes: pixel `background-size`
values, or the intrinsic size of an inline SVG data URI at `auto`. For those
boxes the background walk runs again with their images read as absent
(`resolve_background_info_skipping_images`), which is what lets the link be
scored at all, since the walk gives up on any raster image. A texture tile now
needs a stated size too: a remote file drawn at `auto` has no size the style
states, so it is a picture, which silences bt.cn's green tab image on a
`#f7f8f9` list item.

**The static engine.** It has no layout, no rects and no `z-index`, and
carries neither `background-size` nor `background-repeat`, so it reads
structure (`picture_under_text` in `crates/html/src/layer.rs`): a media
element or a raster box taken out of flow and stretched over its containing
block (`inset: 0`, every side at 0, or `width` and `height` at 100%), where
no positioned box between the sibling and the photo bounds it, so the
containing block holds the text; and a `::before` or `::after` photo drawn
the same way on the text's element or an ancestor, which the cascade pre-pass
now marks (`set_pseudo_picture`). The climb stops at the first opaque ancestor
fill. It reads a stretched photo before or after the content as beneath,
because one stretched over the text it covers would hide that text. Icons are
inline SVG only, and `data_svg_intrinsic_size` resolves the CSS escapes the
static serializer writes into an unquoted `url()`.

**Two effects of the earlier revisions, stated.** Removing `map` from
`NON_RENDERED_TAGS` also adds `tiny-text` for text written directly inside a
`<map>`: the review's repro now reports `9px body text` beside its contrast
finding, in both engines. No fixture carries that shape and no golden moves.
And the static engine scores a `hidden` menu that a mobile-only media query
reveals, because the static cascade reads every `@media` block: the review's
repro reports `2.6:1, text #a1a1a1 on #ffffff` statically, and a browser at
1280px reports nothing.

Each golden below was compared finding by finding against the previous
recording: two added, none removed.

- `detect-fixture-json-link-text-contrast-html`, `detect-fixture-text-link-text-contrast-html`: 16 to 18. Two should-flag cases join: a link with its own external-link icon (`#979797`, 2.9:1) and a link in a list item with an arrow bullet image (`#989898`, 2.9:1). The should-pass column gains four links over photos, none reporting: inside zero-height wrappers, after the content at `z-index: -1`, in content raised to `z-index: 1` over a later photo, and over a `::before` photo.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the sweep total moves from 437 to 439, which is those two findings.

**Corpus, run 2, 346 captures.** Removed 0, added 391, violations 0 (423
before this revision). The review's pixel scan, repeated over the new added
set (the median of the screenshot pixels just outside each finding's rect
against the background it names): 34 of 365 differ by more
than 60 (56 of 395 before), 16 by more than 120. Of the review's ten
crops, aisupply.framer.website (three captures), bt.cn (four) and zigzag.kr
now add nothing there, and thairath.co.th's photo-card label is silent.
ladepeche.fr and yna.co.kr stay. The ladepeche title sits in a white section
in the capture's own geometry and on a photo in the screenshot, which moved
between the two; the yna selector matches two elements, and the finding
belongs to the second, white on its own green `#75a54f`, while the scan crops
the first. microsoft.com's near-white card caption over its product image,
which the first draft of this revision still reported, is silent. Of the
other remaining rows read by hand, two land on content the snapshot does not
place there (aajtak.in, a second thairath.co.th label), two are real surfaces
the scan reads around rather than under (yungching.com.tw's grey search
field on a dark band, whose rect is the field itself, and an app sidebar
under a modal backdrop), and one is a verdict against the walk's gradient
stops (white on `#ffee99` over an orange hero gradient, hrsimple.app), which
is how the walk scores every gradient. The review's
repro pages, scanned live and statically, all answer as intended; the table
is in the commit message.

### Risks carried, not fixed

- A background that resolves to the text colour itself is dropped, which also
  drops text genuinely painted in its own background: invisible, and a real
  1:1 failure. It is exact hex equality, so a link one shade off its surface
  still reports, and the guard covers only the SAFE_TAGS text path, so `<p>`
  and `<div>` still report the `1.0:1`. Written into
  `resolved_bg_matches_text`'s doc comment beside the code.
- The hit-test fallback runs only where the climb reaches an opaque `body` or
  `html`, or a transparent document, and only for points inside the viewport
  that a live browser answers or a capture recorded. A photo the climb cannot
  reach within its bounds is missed below the fold and in replay, and the link
  is scored on the page fill.
- The stacking scale is coarse. It knows `z-index`, positioning, flex and grid
  items, opacity and transforms, and not `isolation`, `filter`, `will-change`
  or `contain`, and an earlier sibling at layer 0 is read as beneath in-flow
  text by rule. A later sibling laid beneath the text by anything the scale
  does not know is not read, and the link is scored on the fill the walk
  found.
- A rect is read unclipped: a photo inside an ancestor that clips it away
  still covers the text for this test. Children that neither cover the text
  nor have no size are skipped, so a covering photo inside a sized,
  non-covering box further down is not reached.
- A pseudo-element photo is read where the pseudo is absolutely or fixed
  positioned with a computed size that covers the text. An inline or
  statically laid out pseudo is not read.
- An icon or a texture tile needs a size the style states. A remote icon
  drawn at `auto` leaves its link unscored, as before this branch, and a
  remote texture drawn at `auto` is now a picture, which silences the links on
  it. The static engine reads only inline SVG sizes and cannot see
  `no-repeat`, so a repeating inline SVG of at most 32px on the link is read
  as an icon there.
- A detached surface within 24, summed over the channels, of the resolved
  background is taken to be the same surface.
- The static engine reads structure, not layout. A photo stretched over a
  positioned wrapper that is itself sized to the section (`height: 100%`) is
  bounded by that wrapper and is not read, so the link over it is scored on the
  page fill. Pseudo-element photos are read only from author rules with
  `inset`, every side at 0, or `width` and `height` at 100%.
- A gradient under the text is scored against its stops, the worst of which
  may not be the part under the run. The gradient drawn larger than its box is
  the one shape this path stands down for.

### Known limits at merge

Recorded when `corpus/integration` merged this branch, from its regression
review, which found no finding lost against base in the static engine, the
browser engine, or live pages. These are what the branch still gets wrong.

1. **The browser engine reports content that is never shown.** It scores
   links inside a closed `<details>` and under `hidden="until-found"`: the
   fixture's `a.pass-details-link` reports 1.9:1 and `a.pass-until-found-link`
   2.0:1 in a browser scan. Base already reports a `<p>` there, so this is not
   a regression. Two sentences above claimed otherwise (that a browser measures
   a zero-size rect under `until-found` and reports nothing, and that the
   should-pass column is finding-free); in browser mode neither holds, and both
   are corrected in place.
2. **A sibling indicator is not read as the surface.** A sliding pill drawn by
   a sibling box (a tab switcher's active indicator) is missed, so the finding
   names the track behind it.
3. **Snapshot and screenshot disagree** in about 10% of sampled findings:
   layout shifts between the capture and the screenshot, rects over icons,
   modal backdrops.
4. **Gradients are scored against their worst stop**, which may not be the
   part under the text.
5. **The static engine reports `hidden` mobile menus** that a mobile-only
   media query reveals, while a 1280px browser scan does not.

## Recorded 2026-09-13: link and span contrast joins the integration

`corpus/integration` merges `corpus/fix-link-contrast`. The branch's
`color-html` and `link-text-contrast-html` fixture goldens replay as it
recorded them. Seven goldens moved on both sides, so they were re-recorded from
the integrated binary rather than merged by hand, and each was compared finding
by finding against the integration's previous recording and against the
branch's own delta (its base against its tip): the two `overlay-positioning-html`
fixtures, the three directory sweeps (`detect-dir-json-all-fixtures`,
`detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`), and the two
`--no-advisory` sweeps. The integration moved by exactly the branch's delta: one
finding added in `overlay-positioning.html` and twenty in each sweep, none
removed, nothing extra and nothing missing. The only text lines that differ from
the branch are the summary counts, 402 to 422 here against 419 to 439 there, the
same twenty. The static engine reads neither the reveal sweep nor the pixel
pass, so nothing else moves.

## Recorded 2026-09-13: text and raster rules score only what is painted at capture

The URL engine's element pass now asks one predicate
(`crates/core/src/browser/painted.rs`) before it reports a text or raster
measurement, and drops the finding when a visitor cannot see the element at
rest. An element is not painted at capture when:

- `checkVisibility()` is false, its computed `visibility` is `hidden` or
  `collapse`, or an ancestor has `display: none` or `content-visibility: hidden`;
- its effective opacity (its own times its ancestors') is at or below 0.02;
- an ancestor that clips it has no area on a clipped axis, or misses it on the
  x axis (`hidden`, `clip`, `auto`, `scroll`: carousel tracks, scrolled table
  columns) or on the y axis (`hidden` and `clip` only). Which ancestors clip
  follows the containing block: every ancestor for an in-flow box, the
  containing block and up for an absolute box, a containing ancestor and up
  for a fixed box. `html` and `body` never count as clips, and neither does a
  vertical scroll container or a full-viewport fixed layer on the y axis, so an
  app shell or a smooth-scroll viewport keeps the content below its fold. Nor,
  for what lies below its bottom edge, does a box that hides overflow, is at
  least as tall as the fold (the viewport, or the root's layout height when
  that is shorter), and whose content runs past it (or whose `scrollHeight`
  was not recorded): smooth-scrollbar and Locomotive Scroll wrap the page in
  such a box without fixing it and move the content by script, which a capture
  cannot tell from content held clipped. Content above its top edge, the x
  axis, and clips shorter than the fold (carousels, accordions, collapsed
  menus) are tested as before.
  Once the walk passes a scroll container that has content to scroll to
  (`scrollWidth` or `scrollHeight` past the client size, or not recorded), the
  ancestors above it are tested against the scroller's box on that axis, not
  the element's: scrolling brings the element into the scroller's box, so a
  Tailwind shell (`h-screen overflow-hidden` around `main { overflow-y: auto }`)
  keeps everything main scrolls to, while a column with nothing to scroll
  inside a frame that hides overflow still loses what the frame hides. Cells
  past a horizontal scroller's own edge are dropped as before;
- its box lies wholly outside the scrollable document (before the start, or
  past the root's scroll width, right to left aware), or it sits in a fixed
  layer whose own box lies outside the viewport.

A fixed box is contained, not a viewport layer, under an ancestor with
`transform`, `translate`, `scale`, `rotate`, `perspective`, `filter` or
`backdrop-filter` other than `none`, a `will-change` naming one of them,
or `contain: paint | layout | strict | content` (`container-type` applies
only size and style containment and holds nothing). The snapshot now records
`willChange`, `contain`, `translate`, `scale`, `rotate` and `perspective`, and
`scrollHeight` as an eighth metric. A recording without them still loads (the
properties read as empty, the metric as NaN), and the gate reads an unrecorded
property as undecided: a fixed box under an undecided ancestor is kept, and an
undecided ancestor never makes a box clip sooner.

Covered: `all-caps-body`, `body-text-viewport-edge`, `cramped-padding`,
`extreme-negative-tracking`, `gray-on-color`, `justified-text`, `line-length`,
`low-contrast` (the computed and placeholder forms of the element pass),
`text-overflow`, `tight-leading`, `tiny-text`, `undersized-ui-text`,
`wide-tracking`, and `buried-raster`. `buried-raster` measures the element's
own opacity, so for it only ancestors count toward transparency, and a raster
under 0.15 opacity is a state layer and skipped when an animation moves its
opacity (or its keyframes cannot be read), or when it declares an opacity
transition (`opacity` or `all` with a non-zero duration), rests at 0 (an
effective opacity at or below 0.02), and carries a second marker: any
animation, `loading="lazy"` or a lazy-loading library's attribute
(`data-src`, `data-srcset`, `data-lazy*`, `data-original`, `data-bg`,
`data-loaded`, `data-ll-status`), a class on it or its parent naming `lazy`,
`loading` or `preload`, a `<video>` parent, or a sibling that is or holds a
video or a raster over at least half of its box (a crossfade stack, a poster
over a video). A declared transition alone is not enough, because Tailwind's
`transition` utility lists `opacity` on everything it animates, and neither are
the markers at a faint value other than 0: a fade starts from 0, while a buried
image sits at 0.1 under an overlay, and Next.js images are lazy by default. A
capture records no transition in progress, so a script-driven crossfade over
layers that are not siblings is still reported, and an image genuinely held
buried at 0 that also carries `loading="lazy"` or sits over a sibling image is
skipped.

Not covered: the visual-contrast pixel pass, the page passes (`heading-rhythm`,
`text-occlusion`, `first-viewport-column-overflow`, `kicker-above-heading`,
`repeated-container-text`, `em-dash-overuse`, the typography pass),
`content-hidden-at-rest` (hidden text is what it reports), the style tells that
describe authored CSS in whatever state is showing (`gradient-text`,
`ai-color-palette`, `overused-font`, `side-tab`, `dark-glow`,
`italic-serif-display`, `icon-tile-stack`, `nested-cards`,
`clipped-overflow-container`, the `design-system-*` rules), rule-pack findings,
and the static HTML and text engines, which measure no boxes.

The new fixture `painted-at-capture.html` pairs each hidden case (a collapsed
submenu, the third cell of a horizontal scroller, a wrapper with no size, a
faded crossfade layer, an off-canvas panel, a column below a frame with
nothing to scroll, a lazy image fading in, a poster over a video, a fixed
drawer past the viewport) with a visible twin carrying the same measurement,
plus a popover that escapes a clip below its containing block, text below an
inner scroller's fold, a buried image with Tailwind's transition list, two
buried lazy images at 0.08 with a transition (Tailwind's list and
`transition-opacity`), text below the fold of a viewport-tall frame that hides
overflow around transformed content, and a
fixed badge inside each containing-block trigger (`will-change`, `contain`,
`translate`, `scale`, `rotate`, `perspective`, `backdrop-filter`). The file
scan has no boxes and reports all thirty-one; the
browser test `the_rule_pass_skips_what_is_not_painted`
(crates/browser/tests/evidence.rs) pins that the URL engine reports only the
twins, and fails with the gate off.

On the run 9 recordings the gate removes 3,589 of 15,609 findings and adds
none: `body-text-viewport-edge` 139, `buried-raster` 1,043, `cramped-padding`
8, `extreme-negative-tracking` 12, `justified-text` 55, `line-length` 49,
`low-contrast` 1,017, `text-overflow` 13, `tight-leading` 102, `tiny-text` 135,
`undersized-ui-text` 986, `wide-tracking` 32. Of the 446 confirmed-harmful
removals, 418 keep a painted finding of the same cluster in the same capture;
the other 28 are the yna.co.kr weather carousel, whose green and orange status
words are only off-screen slides at capture. The recordings predate the new
containing-block properties and `scrollHeight`, so the 23 fixed-layer removals
of the first cut (a parked mobile menu, a side nav, a newsflash popup) are kept
as undecided, 4 swiper arrow buttons at opacity 0 with only a declared
transition are reported again, and every frame as tall as the fold reads as
overflowing: that keeps 174 opacity-0 lazy images on four zigzag.kr product
captures and 24 low-contrast headings on framai.framer.website, which clipping
removed before. The zigzag.kr images sit below a collapsed product-details
panel 1,600px tall that hides overflow until a "more" button opens it, so
those are kept wrongly: a collapsed panel as tall as the fold has the same
shape as a smooth-scroll frame. The faint-value rule keeps nothing more on run 9: every
`buried-raster` finding above opacity 0 there was already kept.

- `detect-fixture-json-painted-at-capture-html`, `detect-fixture-text-painted-at-capture-html`: new cases, the static engine's thirty-one findings.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures` (409 to 440), `detect-scope-type`, `detect-scope-both`, `detect-no-advisory-json`, `detect-no-advisory-text`: the same thirty-one findings in the sweeps; every changed line adds a finding on the new fixture or moves the count.

### Known limits at merge

1. **Collapsed tall panels.** A collapsed panel as tall as the fold that hides
   overflow (a "more" product-details panel) has the shape of a smooth-scroll
   wrapper, so the images below it stay reported: 174 `buried-raster` findings
   on zigzag.kr in run 9.
2. **Horizontal scrollers.** Cells past a horizontal scroller's own edge are
   dropped even though scrolling reaches them, unlike vertical scrollers. This
   is by design.
3. **Not caught.** `transform: scale(0)`, clipping by `clip-path`, and text
   covered by another layer.
4. **Old recordings.** Where the new containing-block properties or
   `scrollHeight` are missing, the gate keeps elements rather than drop them.
5. **Harmful-cluster removals.** Run 9 removes 446 findings from
   harmful-labelled clusters. 418 keep a painted finding of the same cluster in
   the same capture; the other 28 are yna.co.kr carousel duplicates that are
   only off-screen slides at capture.

## Recorded 2026-09-13: painted-at-capture joins the integration

`corpus/integration` merges `corpus/fix-painted-at-capture`. The code merged
without conflicts. `link-contrast` scores links and spans through the SAFE_TAGS
text path of `check_element_colors_dom`, which reports one finding per colour
pair per page. The driver's paint gate drops `low-contrast` after the element
pass, but by then an unpainted first link (a collapsed submenu, a duplicate
carousel slide) would already have claimed the pair, and every visible link in
that colour would go unreported. The path's `keep` callback
(`safe_tag_text_hit_stands`) now asks the same predicate for a gated rule
before it registers the pair, beside the inline ignore and the wrong-layer test.
`an_unpainted_link_does_not_spend_the_pages_contrast_report`
(crates/core/src/browser/driver.rs) pins it and fails without the callback
change. The placeholder form and the full `check_colors` pass go through the
driver's gate as the branch wrote it.

Six goldens moved on both sides, so they were re-recorded from the integrated
binary and compared finding by finding against the integration's previous
recording and against the branch's own delta (its base against its tip): the
three directory sweeps, `detect-scope-both`, and the two `--no-advisory`
sweeps. `detect-scope-type` merged cleanly and replays as merged. Each moved by
exactly the branch's delta: the thirty-one fixture findings in the directory
and `--no-advisory` sweeps, eighteen in the two scope sweeps, none removed,
nothing extra and nothing missing. The only text lines that differ from the
branch are the summary counts, 422 to 453 here against 409 to 440 there. The
static engine measures no boxes, so no other fixture moves.

## Recorded 2026-09-13: heading-rhythm reads the layouts the reveal sweep exposed

Once URL scans measured the page after the reveal sweep, headings that sat at
opacity 0 in the old first capture were measured for the first time, and the
check misread their layouts: eyebrows wrapped in their own boxes, accordion
triggers and card headlines that end their box, rules, photos, icon badges and
stacked headings above, title bands that draw their own rule, standfirsts behind
`display: contents`, empty spacers and padding held open above. heading-rhythm
is browser-only, so no static golden moves.

What the check now treats as the end of a heading's box, and so as nothing below
to measure: a box that draws a bottom edge (a border, a shadow, a background band
that differs from its backdrop), or a box that repeats as a run of like siblings:
the neighbour has the same tag, holds a heading of the same level in the same
place (the same child path, or the same chain of tags and classes down to it),
and has a similar outline (accordion rows, list items, cards in a grid). A layout
wrapper that shares a generic class with its neighbour but holds other content (a
heading alone in a `.row` before a `.row` of feature columns, a `w-container`, a
`wp-block-group`) repeats nothing. Any other wrapper is measured past, and its bottom
padding and margin count as space below, so a padded section header, a
block-editor heading block, a title row stretched by an icon or a tall button,
and a heading last in one grid column with content under the row all flag as
integration did. Empty spacer boxes count as space below as well as above, so
builder layouts that hold every gap open with a spacer can flag. A line above the
heading folds into its cluster only when it reads as a label (smaller than the
body text, uppercase, tracked out, or a small chip); a plain date line set like
body copy stays content of its own.

Policy kept deliberately: a heading set tight under a picture (a photo, a card
thumbnail, a hero image) or under a block that ends in a rule (an hr, a divided
list) is not flagged. The picture or rule already separates it, matching the
rubric's `media-above` and `divider-above` codes and the judges' labels. A rule
is a bottom edge drawn alone: a code block, a panel, a table cell or a callout
bordered on another side is framed content, and a heading tight under it flags.

`heading-rhythm.html` was rewritten into two columns (should flag, should pass)
with a case per misread shape, and a third column after them holds the wrapper
spacing cases (header padding, block padding, small padding, icon row, tall
button row, grid column, spacer below, spacer stack, plain line above, a framed
code block above, a title alone in a layout row before a row of feature columns)
plus a heading that ends a ruled box. `.tiles` gained a background so the tile row reads
as content rather than a spacer. The static engine's one finding on the fixture,
cramped-padding on `.pass-band`, is kept byte for byte, so the fixture and
directory goldens are unchanged. The browser behavior is pinned by
`crates/browser/tests/heading_rhythm.rs` and `crates/core/tests/heading_rhythm.rs`.

- No golden re-recorded.

### Known limits at merge

1. **No corpus evidence for the recall fixes.** The padded-wrapper,
   bordered-box and layout-row fixes are proven only on constructed pages. The
   corpus had none of those shapes.
2. **Cards whose structure differs.** A card with a badge beside one without,
   sharing no class, is measured past. A heading ending such a card can flag.
3. **Two-row accordion with one row open.** The closed row can score below the
   0.7 structure overlap and is then measured as base does.
4. **Kicker folding.** A same-size, mixed-case brand-colour kicker no longer
   folds into the heading.
5. **Background comparison.** It reads colour only and ignores images and
   gradients.
6. **Exempt by design.** A card title under an image, a heading directly under
   an `hr` or a bottom-only rule, and a section heading that ends in its own
   border.

## Recorded 2026-09-13: heading-rhythm joins the integration

`corpus/integration` merges `corpus/fix-heading-rhythm-reveal`. The code merged
without conflicts: the integration had not touched `page_checks.rs` since the
branch point. Only this file and the generated browser asset conflicted; the
asset was regenerated with `cargo xtask bundle`. heading-rhythm is browser-only
and the static engine's one finding on its fixture is unchanged, so every golden
replays as recorded and none was re-recorded.

heading-rhythm is a page pass and not one of the paint-gated rules, so the
driver's painted predicate does not reach it. Its own candidate filter
(`rhythm_visible_flow`) skips a heading that is itself `display: none`,
`visibility: hidden`, at or below 0.05 opacity, out of flow, or without a box,
but not a heading hidden by an ancestor's opacity, clipped by an ancestor, or
off screen. This merge adds no gating.

## Recorded 2026-09-12: a side accent reports only on a rounded card

The corpus judging pass found `side-tab` firing on square boxes with a colored
left rule: a themed notification banner, a bespoke timeline entry, a table-row
marker. The maintainer's call on those crops is that the square version is an
older convention and the rounded card is the tell. A left or right accent now
reports only when the two corners away from the stripe are at least 4px, so a
box rounded only along the stripe still reads as square. Top and bottom bands
are unchanged in every producer: `border-accent-on-rounded` owns the rounded
half of that scope, and the square band was not what the corpus judged.

The gate holds in every producer of the rule, so one visual answers the same
way however it is authored:

- the border check (`check_borders`), in the browser and static engines;
- the browser pseudo-element stripe check;
- the two style-text scans every HTML engine runs over `<style>` and linked
  stylesheets, the absolute `::before` / `::after` bar and the inset
  box-shadow stripe. The scan functions stay as recorded; the static engine
  gates what they return on the cascade of the elements the rule paints, the
  browser on the live elements, and a rule no element on the page matches on
  its host rule's own declarations;
- the text engine: the same two scans over `.css` files, style blocks and
  CSS-in-JS, reading the host rule's declarations, and the six line matchers.
  A utility class reads the `rounded-*` classes in its markup tag; a CSS
  declaration or style-object property reads the radius declarations in its
  own block, template literal, `style=""` value, or (in `.sass`) indentation
  block.

Corners are read from every declaration that names one. The static cascade
expands `border-radius` into the corner longhands with the shorthand's own
cascade order, so `border-radius: 12px; border-top-right-radius: 0` (what
`rounded-lg rounded-r-none` compiles to) is square at that corner, and a later
shorthand resets an earlier longhand. The text readers apply declarations in
source order, and utility classes in the order the framework emits them. `em`
reads against the element's font size, and a `border-radius` built from `var()`
reads each corner's own position once the value resolves, as the browser does.
A radius the reader cannot resolve (a
`calc()`, an unresolved `var()`, `$radius`, a theme key) is unknown rather than
zero, and an unknown card keeps its finding.

Nesting resolves the way a preprocessor compiles it. A nested `&::before` bar,
an `&.is-accent` or `&:hover` rule and a BEM `&--modifier` read the corners of
the rule they sit in; a CSS-in-JS template's own declarations style `&`; a media
query passes through. A stripe revealed on `.card:hover::after` reads `.card`,
the host the static engine looks up, and a rule whose selector is one compound
(`.card`) styles every host that carries its classes (`.card.accent`). The
style-text pseudo-element scan no longer dedupes a nested selector on its text,
so a square card's `&::before` cannot hide a rounded card's `&::before` later
in the same file. The text engine reads each stylesheet's blocks once, whatever
the stripe count.

The gate fails safe to the pre-gate behavior. It removes a finding only where
the card is known to be square: a scope the reader read completely that
declares no radius (the initial square box), or a literal radius under the
rounded threshold. Wherever a reader cannot determine the radius, the card is
treated as possibly rounded and the finding stays, as it did before the gate.
In the text engine that covers an interpolation (a `${...}` radius, a bare
`${mixin}`, an interpolated selector, a `css` template inside another
template's interpolation), an unresolved `var()` or theme token, a mixin call
(`@include x;`, `+x`, `@extend`, `@apply`, `composes`, a Less `.x();`), a style
object spread or a theme-scale number (`sx={{ borderRadius: 2 }}`), a class
attribute that is an expression, and a markup tag that does not close on its
line. At-rules (`@media`, `@supports`, a block `@include breakpoint(md) { }`)
and style-object at-rule keys pass through to the card around them, a context
rule (`.dark &`) names the same element, and indented Sass follows `&` nesting
and `+mixin` wrappers the way braces do. In the static engine it covers a
radius the cascade cannot apply (a rule nested in a style rule, a rule inside
`@container` or an unknown at-rule, a selector the matcher refuses) on the
elements it may reach, a `rounded-*` class with no compiled rule (read off the
utility scale), and, for a card no read declaration gave a radius, a linked
stylesheet the engine did not read (other than a font service). A radius on a
pseudo-element or behind a hover or focus state cannot round the card at rest
and is not counted.

In stylesheet text (a `.css`, `.scss`, `.sass` or `.less` file, a Vue, Svelte
or Astro `<style>` block, a CSS-in-JS template) the declarations around an
accent are not enough to call the card square: another rule for the same
element, or a class the element may carry, can round it. So a left or right
accent there drops only when one of two things holds, and the border
declaration, the pseudo-element bar and the inset box-shadow all read it
through the stylesheet's host index, so one visual answers the same way
however it is drawn:

- (a) the element is known square: the index ties the accent's rule to the
  radius rules for the same element (the same selector, a compound such as
  `.card` for `.card.is-active` or `.card:hover`, a grouped selector, a nested
  `&` rule), and those rules declare both corners away from the stripe with
  literal values under the threshold, with no unknown radius among them;
- (b) the whole file is known square: no radius declaration in any of its
  stylesheets can round those corners (none at all, or only literal values
  under the threshold, read corner by corner at their largest), and nothing in
  them could bring a radius in unseen (a mixin call, `@extend`, `@apply`,
  `composes`, a spread, a bare interpolation, an interpolation naming a radius,
  a `var()` or other value the reader cannot resolve).

Everything else reports as it did before the gate: a tied rule that rounds the
card or leaves its radius unknown, and a file that declares a radius on some
selector the index cannot tie to the accent's rule. Indented Sass has no index;
its accent reads (a) from its indentation scope and (b) from the file. The
static and browser engines read the cascade.

A markup accent (a utility class, a `style` attribute, a style object or a JSX
prop inside a tag) reads its own tag as described above, and in a file with no
radius in its style text (its `<style>` blocks, CSS-in-JS templates,
`createGlobalStyle` / `injectGlobal` templates and styled-jsx blocks) that is
the whole answer, so a plain Tailwind file answers as before. When the file's
style text does declare a radius, a tag that reads square is also checked
against it, through the same host index:

- every radius rule whose subject could match the tag (each class it names is
  on the tag, its type is the tag's, it paints no pseudo-element; a template's
  own `&` declarations only for a styled component the file defines) must leave
  the corners away from the stripe square, and one that rounds them or leaves
  them unknown (`var()`, a mixin, an interpolation) keeps the finding;
- past that, the tag is known square when a rule tied to its own classes
  declares both corners square, or when every radius in the style text is
  literal. A tag with a `css` prop, or whose classes are an expression, keeps
  the finding.

`rounded-*` utilities on the tag still round it.

A file that imports a stylesheet the reader does not follow (a script `import`
or `require` of a `.css`, `.scss`, `.sass`, `.less`, `.styl` or `.pcss` file, a
CSS module among them, an `@import` or a non-`sass:` `@use` in its style text,
a `<style src>` block, a `<link rel="stylesheet">`) could round any class a tag
carries. There a markup accent drops only when its own tag squares it off: a
`rounded-none` or `rounded-0` utility, or literal square radii for both corners
away from the stripe in its radius props, `style` attribute, style object or
`sx` object. Every other tag keeps the finding, and a file with no such import
answers as described above.

The static border snippet now prints the radius in px, the way the browser's
computed style does: `border-radius: 0.375rem` reports `6px` where it used to
print the unconverted `0.375px`.

The fixtures moved with the rule, so the goldens below carry fixture edits and
the two intended output changes, not lost findings.

- `detect-fixture-json-border-baseline-html`, `detect-fixture-text-border-baseline-html`: `border-baseline.html` retired its square `border-left: 4px` flag case (it now sits in the should-pass column as a square callout), added `border-left: 6px` on a card rounded away from the stripe, a card rounded by `border-top-right-radius` / `border-bottom-right-radius` (`border-left: 4px`), and one whose radius is a `calc()` (`border-left: 9px`, a width of its own so the two snippets attribute).
- `detect-fixture-json-pseudo-stripe-css`, `detect-fixture-text-pseudo-stripe-css`, `detect-fixture-json-pseudo-stripe-vue`, `detect-fixture-text-pseudo-stripe-vue`: the left and right flag cases gained host rules with a radius and each file gained square-host pass cases; the findings are the same and move down by the inserted lines. `pseudo-stripe.html` rounds `.row-stripe` and adds a square host and a rounded-under-the-stripe host as pass cases, so its goldens do not move. `astro-inset-shadow-stripe.astro` rounds its left and right flag cases and adds a square pass rule after the others, so its goldens do not move either.
- `detect-fixture-json-should-flag-html`, `detect-fixture-text-should-flag-html`: the four side accents on the `0.375rem` card print `border-radius: 6px`.
- `detect-fixture-json-framework-next-modules`, `detect-fixture-text-framework-next-modules`, `detect-framework-next-modules-text`: `Sidebar.module.css` is a square sidebar with `border-right: 3px solid #4f46e5` and no radius, the convention the premise retired; its finding is gone (6 to 5 findings).
- `detect-fixture-json-side-accent-producers-html`, `detect-fixture-text-side-accent-producers-html`, `detect-fixture-json-side-accent-producers-css`, `detect-fixture-text-side-accent-producers-css`, `detect-fixture-json-side-accent-producers-jsx`, `detect-fixture-text-side-accent-producers-jsx`: new fixtures that draw the same accent through every producer, square and rounded. Each reports only its flag column: six findings for the HTML page (one a border whose radius is `var(--r)` = `0 12px 12px 0`), six for the stylesheet (one a `:hover::after` bar), three for the components.
- `detect-fixture-json-side-accent-nested-scss`, `detect-fixture-text-side-accent-nested-scss`, `detect-fixture-json-side-accent-nested-tsx`, `detect-fixture-text-side-accent-nested-tsx`: new fixtures for nested accents, rounded and square, in SCSS and in styled-components templates. Each reports only its flag column: five findings for the stylesheet, two for the components.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the sum of the above, 421 to 442 findings.
- `detect-unreadable-file-in-dir`: the case's readable `a.html` carries `border-radius: 10px`, so it still produces the finding the case exists to show next to the unreadable file's error; the snippet gains `+ border-radius: 10px`.

Recorded 2026-09-13, the fail-safe revision. Base reported every shape these
cases add, and the gate had silenced three of them in the text engine: a
styled-components card whose radius is an interpolation with a nested
`&::before` bar, an accent inside `@media` or a block `@include` on a rounded
card, and an indented Sass `&.on` rule under a rounded card. The goldens move
only by the new flag cases; every pass case is a literal square host in the
same shape and stays silent.

- `detect-fixture-json-side-accent-nested-scss`, `detect-fixture-text-side-accent-nested-scss`: `side-accent-nested.scss` gains an accent inside `@media` (`border-left: 12px`) and inside a block `@include breakpoint(md)` (`border-right: 13px`) on a rounded card, and a nested bar on a card an `@include card-shape;` rounds (`&::before`, 6px), with square `border-radius: 0` twins for the two wrapped accents. 5 to 8 findings.
- `detect-fixture-json-side-accent-nested-tsx`, `detect-fixture-text-side-accent-nested-tsx`: `side-accent-nested.tsx` gains a template whose radius is `${({ theme }) => theme.radii.md}` with a nested `&::before` bar (7px), a template a bare `${cardShape}` interpolation styles with a nested `&::after` bar (8px), and an `sx` style object with a `'@media (min-width: 600px)'` key holding the accent on a rounded card (`borderLeft: '10px solid`), with square twins for the literal template and the style object. 2 to 5 findings.
- `detect-fixture-json-side-accent-nested-sass`, `detect-fixture-text-side-accent-nested-sass`: new fixture for indented Sass. It reports its three flag cases (an `&.on` rule under a rounded card, an accent inside `@media` on a rounded card, an `&.on` rule under a card a `+card-shape` mixin styles) and none of its square pass cases.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the sum of the above, 442 to 451 findings (459 to 468 with advisories).

Recorded 2026-09-13, the stylesheet revision (rules (a) and (b) above). Base
reported every accent whose card another rule rounds, and the text engine's
border matchers had silenced them because they read only the accent's own block
and the blocks around it: `.card { border-radius }` then `.card.is-active {
border-left }`, the same with `:hover`, a second `.alert` block, a grouped
`.panel, .widget` radius, a second SCSS block, a Vue `<style scoped>` block, and
the unknown cases (`.list-item { border-radius: var(--radius) }` with
`.list-item.active`, a radius on `.card` with the accent on `.card-accent`). The
pseudo-element and inset scans already asked the index for the tied rules; they
now also read (b), so a bar on a class no radius rule names reports in a file
that rounds another class.

Pass cases that sat square in a file whose flag cases round other selectors
answered square only because their own rule declared no radius. That is the
separate-class shape rule (b) keeps, so those cases now square themselves off
with `border-radius: 0` (rule (a)), and the no-radius-anywhere shape moved to a
fixture of its own.

- `detect-fixture-json-side-accent-producers-css`, `detect-fixture-text-side-accent-producers-css`: the five square pass cases gain `border-radius: 0`, and the file gains flag cases for the same selector (`border-left: 13px`), a compound rule (14px), a grouped radius (`border-right: 15px`), a `var()` radius (16px) and a radius on another class, as a border (17px) and as a `::before` bar (6px). The six original findings move down by the inserted lines; 6 to 12 findings.
- `detect-fixture-json-side-accent-nested-scss`, `detect-fixture-text-side-accent-nested-scss`: the square nested bar, the square children and the square BEM element gain `border-radius: 0`, and the file gains a state rule in a second block for a card rounded in the first (17px) and a flat compound rule after the card's rule (`border-right: 18px`). 8 to 10 findings.
- `detect-fixture-json-side-accent-nested-tsx`, `detect-fixture-text-side-accent-nested-tsx`: the two square templates gain `border-radius: 0`; the findings are the same and move down by the inserted lines. `side-accent-nested.sass` squares its child case off the same way and its goldens do not move.
- `detect-fixture-json-pseudo-stripe-css`, `detect-fixture-text-pseudo-stripe-css`: the square host pass case gains a `border-radius: 0` host rule; the findings are the same and move down by the inserted lines. `pseudo-stripe.vue` and `astro-inset-shadow-stripe.astro` square their pass cases off the same way, below their findings, so their goldens do not move.
- `detect-fixture-json-side-accent-square-sheet-css`, `detect-fixture-text-side-accent-square-sheet-css`: new fixture, a stylesheet that rounds nothing but a `2px` chip. A border, a width longhand, a logical border, a `::before` bar, an inset shadow and an accent on a separate class all pass; no findings.
- `detect-fixture-json-side-accent-flat-vue`, `detect-fixture-text-side-accent-flat-vue`: new fixture, a Vue `<style scoped>` block with a compound accent on a rounded card (`border-left: 4px`, line 16) and a radius on another class (`border-right: 5px`), both reported, and a card squared off in its own rule, silent. The whole-file pass gates a declaration inside a `<style>` block the way the block pass does, so the finding keeps the line the whole-file pass reports, as on base.
- `detect-fixture-json-framework-next-modules`, `detect-fixture-text-framework-next-modules`, `detect-framework-next-modules-text`: `Sidebar.module.css` declares `border-radius: 8px` on `.navItem`, a selector the index cannot tie to `.sidebar`, so the sidebar's `border-right: 3px solid #4f46e5` reports again, as on base (5 to 6 findings).
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the sum of the above, 451 to 462 findings (468 to 479 with advisories).

Recorded 2026-09-13, the markup revision (the markup accent rule above). Base
reported a utility accent on a tag whose class the file's own style text rounds
(`<div class="card border-l-4">` with a scoped `.card { border-radius: 12px }`
in Vue, Svelte or Astro, a `createGlobalStyle` or styled-jsx `.card` rule, a
styled component whose template rounds it), and the gate had silenced it
because the markup reader saw only the tag.

- `detect-fixture-json-side-accent-markup-vue`, `detect-fixture-text-side-accent-markup-vue`, `detect-fixture-json-side-accent-markup-svelte`, `detect-fixture-text-side-accent-markup-svelte`: new fixtures, a scoped `.card { border-radius: 12px }` next to `<div class="card border-l-4 border-teal-700 p-4">`; one finding each.
- `detect-fixture-json-side-accent-markup-square-vue`, `detect-fixture-text-side-accent-markup-square-vue`, `detect-fixture-json-side-accent-markup-square-svelte`, `detect-fixture-text-side-accent-markup-square-svelte`: new fixtures, the same markup with a scoped `.card { border-radius: 0 }`; no findings.
- `detect-fixture-json-side-accent-nested-tsx`, `detect-fixture-text-side-accent-nested-tsx`, `detect-fixture-json-side-accent-producers-jsx`, `detect-fixture-text-side-accent-producers-jsx`: the two `sx` media-key cases moved from `side-accent-nested.tsx` to `side-accent-producers.jsx`. In the templates file their `<Box>` has no class the index can tie, and the file's style text holds unknown radii (an interpolated radius, a bare `${cardShape}`), so the square twin would now report. The style-object reading they pin needs a file without style text. `side-accent-nested.tsx` 5 to 4 findings, `side-accent-producers.jsx` 3 to 4 (`borderLeft: '10px solid`, line 32); the square twin stays silent.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the sum of the above, 462 to 464 findings (479 to 481 with advisories).

Recorded 2026-09-13, the import revision (the imported-stylesheet rule above).
Base reported a utility accent on a tag whose class an imported stylesheet may
round (`import './card.css'` with `className="card border-l-4"`), and the gate
had silenced it because the file carried no style text of its own. The import
is not followed; it only makes the tag's radius unknown.

- `detect-fixture-json-side-accent-import-jsx`, `detect-fixture-text-side-accent-import-jsx`: new fixture, `import './side-accent-import-card.css'` next to `<div className="card border-l-4 border-teal-700 p-4">`; one finding (line 7).
- `detect-fixture-json-side-accent-css-module-tsx`, `detect-fixture-text-side-accent-css-module-tsx`: new fixture, a CSS module import next to ``className={`${styles.card} border-l-4 border-teal-700 p-4`}``; one finding (line 7). The class expression already read as unknown, so this pins the shape rather than a change.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`, `detect-no-advisory-text`: the sum of the above, 464 to 466 findings (481 to 483 with advisories). No other golden moves: the framework fixtures' `globals.css` imports sit in layout files with no side accent.

### Known limits at merge

Recorded when `corpus/integration` merged this branch. These are what the
branch still gets wrong or leaves as base had it.

1. **Markup accents read only their own file.** A stylesheet import counts as
   unknown and keeps the finding, but a global stylesheet applied without an
   import in that file (one a layout loads) is invisible to the reader. There,
   a markup accent with no radius on its tag reads square and drops.
2. **Pseudo-stripe line lookup is quadratic.** `line_of_offset` rescans the
   file for every pseudo-stripe finding, about 7.5s at 8,000 findings. Base has
   the same cost.
3. **Tailwind CDN pages.** On a page with no compiled CSS, `rounded-lg
   border-l-4` reports nothing, on base as well.

## Recorded 2026-09-13: side-tab joins the integration

`corpus/integration` merges `corpus/premise-side-tab`. Five source files
conflicted, all in import lists or adjacent additions: `element_checks.rs`,
`adapters.rs` and `engine.rs` keep the integration's gpt-thin-border,
link-contrast and one-report-per-colour-pair imports beside the branch's
corner readers, `css_scan.rs` keeps radial-spotlight-glow's
`glow_is_perceptible` beside `DeclaredCorners` and `NOMINAL_CARD_WIDTH_PX`, and
`StaticDocument` in `dom.rs` keeps the integration's `pseudo_picture` accessors
beside the branch's radius flags. `check_gpt_thin_border_wide_shadow` and
`positioned_style_implies_escape` are not imported, since the integration no
longer calls them there. The generated browser asset was regenerated with
`cargo xtask bundle`. side-tab is not one of the paint-gated rules, so the
painted predicate and the rounded-card gate do not meet.

Every fixture golden the branch recorded replays as it recorded them. The five
sweeps moved on both sides, so they were re-recorded from the integrated binary
and compared finding by finding against the integration's previous recording
and against the branch's own delta (its base against its tip):
`detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`,
`detect-dir-quiet-all-fixtures`, `detect-no-advisory-json` and
`detect-no-advisory-text`. Each moved by exactly the branch's delta: 56 findings
added and 9 removed in the JSON sweeps, every removal a side-tab finding (the
retired square flag case, the reworded `6px` snippets, and findings that moved
down by inserted fixture lines), nothing extra and nothing missing. The only
text lines that differ from the branch are the summary counts, 453 to 500 here
against 419 to 466 there (523 to 570 with advisories).

## Recorded 2026-09-13: evidence fixes move no golden

`corpus/fix-evidence-bugs` fixes three findings whose evidence a corpus judge
could not trust, all in the URL engine: the pixel contrast pass printed a
verdict above its median (`pixel contrast 3.5:1 median 1.7:1`), script errors
named no script (`Uncaught [object Object]`), and `text-overflow` reported
ellipsized boxes as spills. The oracle records file scans only, so the replay
passes against the branch binary with nothing re-recorded. The three fixtures
the branch extends (`visual-contrast.html`, `script-error.html`,
`text-overflow.html`) gained only browser-only cases, and their static and
sweep goldens replay byte for byte. The browser behavior is pinned by
`crates/browser/tests/evidence_findings.rs` and specified in `CLI-CONTRACT.md`
(the script-error source suffix, the glyph-core verdict).

### Known limits at merge

Recorded when `corpus/integration` merged this branch, from its regression
review, which approved it.

1. **The printed median always equals the verdict.** Both come from one
   glyph-core set now, so the median no longer carries information of its own;
   a reader cannot see how far the verdict sits from the rest of the glyphs.
2. **Script URLs split cluster keys.** A script-error finding now names the
   script URL and top frame, and a deploy that renames a hashed bundle gives the
   same error a new snippet, so corpus clusters split across deploys.
3. **React invariants are not decoded.** A minified React error keeps its
   invariant number; the description names the thrown object but not the
   decoded message.
4. **Old recordings lack `webkitLineClamp`.** A capture recorded before the
   property joined the snapshot reads it empty, so a line-clamp box in such a
   recording is still measured and can still report.
5. **Pixel verdicts near the threshold in tests.** `evidence_findings.rs`
   asserts pixel contrast verdicts close to the threshold, which may be flaky on
   other Chrome builds whose glyph rasterization differs.
6. **Clamped boxes skip sideways spills.** A `-webkit-line-clamp` box holding a
   long unbreakable token that spills sideways stays skipped. Rare.
7. **Inline truncation (fixed at merge).** `truncates_by_design` ran before the
   box and inline split, so an inline element (client width 0) with overflow
   hidden, an ellipsis and `nowrap` was skipped, although CSS ignores overflow
   on an inline box and its text really spills. Base reports `span.truncate`
   and `a.truncate` inside a 140px block; the branch did not. The integration
   commit after this merge applies the self-check only to an element that
   generates a box, and the parent walk passes over inline ancestors.

## Recorded 2026-09-13: painted-gate leaks (corpus/fix-painted-leaks)

Recorded from the binary. The branch closes leaks in the URL engine's
painted-at-capture gate and applies it to three rules that reported elements
that never render: screen-reader text inside a 1px box whose `clip` or
`clip-path: inset()` removes it, text rules on an element with no text or no
area, `layout-transition` on a collapsed tray, a player's hidden volume panel
or a seek bar parked off the canvas, `clipped-overflow-container` naming a
child that never renders or a `position: fixed` layer the host is not the
containing block of, and `nested-cards` on a closed mega-nav panel (a BEM
`__dropdown` class, `role="menu"` / `role="listbox"`, `visibility: hidden`).
Every change is in the browser rule pass, which none of these goldens runs;
they move only because `painted-at-capture.html` gained the twin cases the URL
test (`the_rule_pass_skips_what_is_not_painted` in
`crates/browser/tests/evidence.rs`) pins. The static HTML engine measures no
boxes, so it reports both columns.

- `detect-fixture-json-painted-at-capture-html`, `detect-fixture-text-painted-at-capture-html`: 31 to 52 findings (45 counted, 7 advisory in the text form). The 21 added: `clipped-overflow-container` 3 (`div.clip-host clips positioned div.clip-menu` twice, `div.clip-host.clip-host-cb clips positioned div.clip-fixed-cb`; the fixed layer in `#pass-clip-fixed-host` is not reported by the static engine), `cramped-padding` 2 (the two `player-bar` strips), `layout-transition` 4 (`max-height` twice, `width`, `height`), `low-contrast` 2 (`#ffffff on #7485a9` and `#7384a8`), `nested-cards` 5 (three `div`, two `ul`), `tight-leading` 2, `undersized-ui-text` 3 (`Loaded` twice, `24.53%`). Nothing removed.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`: the same 21 findings, 570 to 591 (500 to 514 counted, 70 to 77 advisory). No other fixture moves.
- `detect-no-advisory-json`, `detect-no-advisory-text`: the 14 counted ones, 500 to 514.
- `detect-scope-type`: the 5 type findings (`tight-leading` 2, `undersized-ui-text` 3), 150 to 155.
- `detect-scope-layout-text`, `detect-scope-both`: the layout findings (`cramped-padding` 2, `nested-cards` 5, `clipped-overflow-container` 3), 25 to 32 counted and 13 to 16 advisory in the text form, 188 to 203 in `both`.

### Known limits at merge

Recorded when `corpus/integration` merged this branch, from its regression
review, which approved it. The merge conflicted only in adjacent test additions
in `element_checks.rs` (both kept), in this file, and in the generated asset
(regenerated). Every golden the branch recorded replays against the integrated
binary, since the evidence fixes moved none.

1. **A transient typewriter span.** Finding 70404, a typewriter hero's span,
   is transient state caught between cycles rather than a leak, and it stays
   dropped.
2. **Motion on boxes that grow on interaction.** A zero-size box that grows
   when used (a progress fill, a hover underline, a collapsed accordion) is not
   painted at capture, so its `layout-transition` is no longer reported.
3. **Fade-in children.** A child that is transparent at rest, such as a
   dropdown that fades in, no longer counts as the named child of
   `clipped-overflow-container`.
4. **Static engine unchanged.** The file engine measures no boxes and reports
   both columns of `painted-at-capture.html`, as the goldens above record.

## Recorded 2026-09-13: full-page screenshot fixtures

`corpus/fix-fullpage-screenshot` adds four fixtures for the URL engine's
evidence screenshot: `fullpage-screenshot.html` (a tall, wide document),
`fullpage-screenshot-scroller.html` (the body scrolls instead of the document),
`fullpage-screenshot-frame.html` (a fixed frame a smooth-scroll library moves)
and `fullpage-screenshot-virtual.html` (rows rendered once scrolled to). Their
URL behavior is pinned by `crates/browser/tests/fullpage_screenshot.rs`; the
static engine runs no script and measures no boxes, so its output for them is
ordinary.

New goldens, recorded from the binary and read by hand:
`detect-fixture-json-fullpage-screenshot-html`,
`detect-fixture-text-fullpage-screenshot-html`,
`detect-fixture-json-fullpage-screenshot-scroller-html`,
`detect-fixture-text-fullpage-screenshot-scroller-html`,
`detect-fixture-json-fullpage-screenshot-frame-html`,
`detect-fixture-text-fullpage-screenshot-frame-html`,
`detect-fixture-json-fullpage-screenshot-virtual-html` and
`detect-fixture-text-fullpage-screenshot-virtual-html`. The tall, scroller and
frame fixtures each report `low-contrast` on the faint foot copy,
`overused-font` for Arial and `repeating-stripes-gradient` for the striped
spacers; the virtual fixture reports `overused-font` only.

Re-recorded sweeps, compared finding by finding against the previous goldens:
`detect-dir-json-all-fixtures` (570 to 580), `detect-scope-type` (150 to 154),
`detect-scope-both` (188 to 192), `detect-no-advisory-json` (500 to 507), and
the text forms `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`
and `detect-no-advisory-text`. Every added finding belongs to the four new
fixtures and nothing was removed; in the text forms the only removed lines are
the summary counts (500 to 507 anti-patterns, 70 to 73 advisory notes).

### Known limits at merge

Recorded when `corpus/integration` merged this branch, from its regression
review, which approved it. `cdp.rs` conflicted where the branch's viewport
shots and `content_size` sat beside evidence-bugs' `page_errors`, which now
returns `PageError` with where it was thrown; both kept. The pixel pass merged
cleanly: the branch's scroll into view wraps the glyph-core measurement that
evidence-bugs changed. The seven sweep goldens that moved on both sides were
re-recorded from the integrated binary and each moved by exactly the branch's
delta, nothing removed: `detect-dir-json-all-fixtures` 591 to 601,
`detect-scope-type` 155 to 159, `detect-scope-both` 203 to 207,
`detect-no-advisory-json` 514 to 521, and in the text forms
(`detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`,
`detect-no-advisory-text`) only the four fixtures' blocks and the summary counts
(514 to 521 anti-patterns, 77 to 80 advisory notes).

1. **Seams.** Stitched viewport tiles can repeat script-driven chrome (a sticky
   header, a cookie bar) at tile seams.
2. **Capture time.** Full-page evidence takes about 26% longer to capture.
3. **Some past-cut findings lack crops.** Element shots stop at
   `MAX_ELEMENT_SHOTS` (40), and a collapsed container has no rect to shoot.
4. **Solid tall sections.** A real tall solid section reads as a blank band and
   falls back to tiles; that costs time only, never pixels.
5. **Scroller detection counts `overflow: hidden`.** A clipping box is treated
   like a page scroller when unrolling the page.
6. **Off-canvas precision unmeasured.** The pixel pass now scrolls off-canvas
   candidates into view, and its precision on those candidates is not yet
   measured.
7. **Harness.** The corpus harness reads `element_shots` through uncommitted
   changes to `harness/build.rs`, `Cargo.toml` and `capture.rs`; they are
   committed from the corpus side, not here.

## Recorded 2026-09-13: low-contrast reads the surface and the ink a reader sees

`corpus/fix-gradient-surface` changes how low-contrast resolves the background
and the text colour, in both engines where they share the logic
(`crates/core/src/checks/gradient_geometry.rs`, the contrast walk in
`crates/core/src/browser/background.rs` and `crates/html/src/background.rs`,
and `contrast_findings` in `crates/core/src/checks/rules.rs`):

- **A gradient is named as the surface.** The snippet appends the box that
  painted it, `(gradient on a.cta)`, and the page's one report of a colour is
  keyed on the text colour and that box.
- **The URL engine reads a gradient where the text sits.** Linear and radial
  layers are evaluated at a 3x3 grid of points inside the text box, from the
  painting box's size, `background-size`, `-position`, `-repeat` and
  `-origin`, composited over what sits behind the box. Anything it cannot
  read (a `calc()` stop, a conic or repeating gradient, `fixed` attachment, a
  four-value position, a translucent sample over an unreadable backdrop) keeps
  the worst-stop verdict. The static engine has no layout and keeps it always.
- **A gradient tile that paints under no glyph is not a background.** An
  underline drawn as `linear-gradient(#000, #000) no-repeat 0 100% / 0% 2px`
  and a radial glow that has faded out before the text are read past. The
  static engine reads this from a `background` shorthand's size and repeat,
  since its cascade carries no `background-size` longhand.
- **Every translucent fill that paints is composited.** The contrast walk no
  longer skips fills at or under an alpha of 0.1. The walk the glow, palette
  and hover checks share is unchanged.
- **The ink is blended before it is scored and printed.** The text colour's
  own alpha, and the opacity of the boxes between the text and its surface,
  fade the glyphs toward the surface; a faded box with a fill of its own fades
  that fill too. Ink at an alpha of 0.02 or less paints nothing and is not
  scored. The frozen call vectors pass no adapter ink and score as recorded.

Goldens re-recorded from the binary and reviewed finding by finding:

- `detect-fixture-json-color-html`, `detect-fixture-text-color-html` (4),
  `detect-fixture-*-dark-gradient-ground-html` (3),
  `detect-fixture-*-dark-theme-modern-color-html` (1),
  `detect-fixture-*-linked-url-patterns-html` (3),
  `detect-fixture-*-nav-cta-constructions-html` (1),
  `detect-fixture-*-overlay-positioning-html` (1): each gradient-surface
  finding gains its source suffix. Ratio, text and background are unchanged,
  because the static engine keeps the worst stop.
- `detect-fixture-*-buried-raster-html`: one finding added,
  `1.1:1 (need 4.5:1) — text #f2f2f2 on #ffffff`, the `.faint-text` paragraph
  at `opacity: 0.05` that used to be scored as solid black. It is a pass case
  for buried-raster, which still passes it.
- `detect-fixture-*-design-system-html`: one finding added,
  `2.0:1 (need 4.5:1) — text #dfaaa1 on #ffffff`, the `.pass-alpha-color` case
  in `rgba(184, 66, 46, 0.45)`, a design-system pass that was scored as the
  opaque colour.
- `detect-fixture-*-visual-contrast-sampling-html`: one finding added,
  `2.2:1 (need 4.5:1) — text #b0b0b0 on #ffffff`, case 6, the collapsed
  accordion trigger faded by `opacity: 0.34`, which the fixture describes as
  pale grey a visitor is asked to click.
- `detect-fixture-json-gradient-surface-contrast-html`,
  `detect-fixture-text-gradient-surface-contrast-html`: new. The seven
  should-flag cases flag. The three should-pass cases marked "URL engine" flag
  here at their worst stop, as the fixture header says; the URL behavior is
  pinned by `crates/browser/tests/gradient_surface.rs`.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`,
  `detect-no-advisory-text`: exactly the sum of the above. Thirteen suffix
  rewrites and thirteen added findings (the three above plus the new
  fixture's ten), nothing removed; 500 to 513 anti-patterns.

The generated browser asset was regenerated with `cargo xtask bundle`.

## Recorded 2026-09-13: one report per gradient box, and both ends of a line sampled

Review of `corpus/fix-gradient-surface` found the per-page dedupe merging
different gradient surfaces. The key was the text colour plus the source
label, and the label is only a tag and a first class, so a row of
`div.w-14` tiles on amber, lime and blue gradients shared one key and only
the first failing tile reported (chorusai.replit.app, base findings 71770,
71779, 71866 and 71875 lost). The revision:

- **Keys the claim on the painting box's identity.** `ColorOpts` carries
  `bg_source_host`, an opaque id of the box that painted the gradient (the
  `ElId` in the URL engine, the `NodeId` in the static one). A SAFE_TAGS hit
  on a gradient is dropped when the page has already reported its snippet or
  its text colour on that box, and a hit that stands claims both. Fifty links
  on one header stay one report, separate tiles report separately, and
  identical tiles whose snippet is the same pair stay one report as they were
  before the branch. With no identity the snippet alone is the key.
- **Samples both ends of the line in the URL engine.** The grid keeps its
  three rows a sixth inside the text box and adds two columns at its left and
  right edges, half a pixel in, so a long line over a horizontal gradient is
  read where its last word sits (15 samples instead of 9). The static engine
  scores the worst stop and is unaffected.

Goldens re-recorded from the binary and reviewed finding by finding:

- `detect-fixture-json-gradient-surface-contrast-html`,
  `detect-fixture-text-gradient-surface-contrast-html`: the fixture gains five
  cases and five findings, the ten recorded above unchanged.
  - `1.2:1 (need 4.5:1) — text #cbd5e0 on #e2e8f0 (gradient on nav.pale-header)`:
    two links on one header, reported once.
  - `1.6:1 (need 3:1) — text #fdfdfd on #fbbf24 (gradient on div.feature-tile)`
    and `1.5:1 (need 3:1) — text #fdfdfd on #a3e635 (gradient on div.feature-tile)`:
    same-class tiles on different gradients, both reported. The navy tile
    beside them passes.
  - `1.3:1 (need 4.5:1) — text #e0e0e1 on #ffffff (gradient on div.edge-banner)`:
    copy justified to the light end of a banner.
  - `1.3:1 (need 4.5:1) — text #dfdfe0 on #ffffff (gradient on div.edge-banner)`:
    the should-pass short copy at the banner's dark end, marked "URL engine",
    which this engine reports at the worst stop like the other three.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`,
  `detect-no-advisory-text`: exactly those five added, nothing removed or
  rewritten; 513 to 518 anti-patterns.

The generated browser asset was regenerated with `cargo xtask bundle`.

### Known limits at merge

Recorded when `corpus/integration` merged this branch, from its regression
review, which asked for changes on the per-label dedupe; `18106116` fixed them
and the review approved. The merge conflicted only in this file, the generated
asset (regenerated) and five sweep goldens. `element_checks.rs` merged cleanly:
painted-leaks' `unpainted_for` gate still decides a SAFE_TAGS hit before the
page claims its colour on a gradient box.

**One interaction with `corpus/fix-evidence-bugs`.** That branch added a faded
date through an opacity stack to `visual-contrast.html` (`div.fade-heavy > p`,
`#d5dbe6` at `opacity: 0.35` on `#0a0b10`), which only the pixel pass could
read, at about 2.6:1 from its glyph cores. The ink blend here folds that
opacity into the text colour, so both engines now report it in the element
pass at the same ratio, `2.6:1 (need 4.5:1) — text #51545b on #0a0b10`, and the
URL engine's pixel pass skips an element that already carries a low-contrast
finding. The readable twin at `opacity: 0.55` still passes in both engines.
`pixel_contrast_reads_glyph_cores` in `crates/browser/tests/evidence_findings.rs`
now accepts the faded date from either path and checks the readable date by
selector as well as by pixel text. This fixture no longer yields a pixel
snippet, so the verdict-not-above-median rule is pinned by the unit tests in
`screenshot_contrast.rs` and `visual.rs`.

Goldens re-recorded from the integrated binary and compared finding by finding:
`detect-fixture-json-visual-contrast-html` and
`detect-fixture-text-visual-contrast-html` (1 to 2, the faded date above), and
the sweeps `detect-dir-json-all-fixtures` (601 to 620),
`detect-no-advisory-json` (521 to 540) and the text forms
`detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures` and
`detect-no-advisory-text`. Each sweep moved by exactly this branch's delta (31
added and 13 removed in the JSON form: the thirteen suffix rewrites plus
eighteen new findings) and the one faded date, nothing else.

1. **568 newly flagged elements in the corpus.** Mostly from blended ink and
   opacity, including decorative text and text caught mid-reveal; to be judged
   in the next run.
2. **Blended ink exposes surfaces base already misread.** A light overlay the
   walk never sees (context.dev) and an off-screen carousel item (drom.ru) now
   score with faded ink over the wrong surface.
3. **Opacity on the surface's own box is not folded.** Opacity on the box that
   paints the surface, or on its ancestors, is not blended into the ink.
4. **Static engine.** It keeps worst-stop gradients and cannot read a
   `background-size` longhand.
5. **API.** `ColorOpts` gains a public `bg_source_host` field.

## Recorded 2026-09-13: script errors and contrast ratios print one way (corpus/fix-script-error-format)

Recorded from the binary. The branch fixes three evidence problems from corpus
run 16 (`observations-16.md` issues 11, 20 and 36):

- **Script errors read `Uncaught <Type>: <message> (at <source>)`.** The
  thrown-object path used to print puppeteer's `err.message`, which drops the
  type (`jQuery is not defined`), while a cross-origin throw kept CDP's typed
  text (`Uncaught TypeError: ...`). Every message now carries CDP's marker and
  the type, and a frame in a script with no URL names `<anonymous>` where it
  printed no source. The message after the type keeps the 160-character cut.
- **One throw reports once.** The dedupe reads `Uncaught (in promise)` as
  `Uncaught`, so a throw reported synchronously and as an unhandled rejection
  keeps only the first report.
- **A failing ratio never prints as the bar.** Every contrast producer prints
  one decimal, two when one would round up to the threshold, and those two cut
  when rounding them would still reach it (`4.49:1 (need 4.5:1)` where it read
  `4.50:1` or `:hover state 4.5:1`). The helper is
  `impeccable_core::color::ratio_label`. The old two-decimal rule is kept
  wherever two decimals already sat under the bar, so no existing golden or
  frozen vector moves.

Script errors come only from the URL engine, which the oracle does not run, so
`script-error.html` (a new `new Function` case) and the new
`script-error-twins.html` add no static findings. Their URL behavior is pinned
by `crates/browser/tests/evidence_findings.rs`.

New goldens, read by hand:

- `detect-fixture-json-low-contrast-near-threshold-html`,
  `detect-fixture-text-low-contrast-near-threshold-html`: four findings, one
  per should-flag case. They are `4.49:1 (need 4.5:1) — text #777777 on #070707`,
  `2.99:1 (need 3:1) — text #595959 on #000000`,
  `4.49:1 (need 4.5:1) — text #7b7b7b on #101010` (a link) and
  `:hover state 4.49:1 (need 4.5:1) — text #747474 on #000000`. Base prints
  `4.50`, `3.00`, `4.50` and `4.5` for them. Nothing in the should-pass column.
- `detect-fixture-json-script-error-twins-html`,
  `detect-fixture-text-script-error-twins-html`: no findings (`[]`, empty text
  output).

Re-recorded sweeps, compared finding by finding against the previous goldens:
`detect-dir-json-all-fixtures` (620 to 624), `detect-no-advisory-json` (540 to
544), and the text forms `detect-dir-text-all-fixtures`,
`detect-dir-quiet-all-fixtures` and `detect-no-advisory-text` (540 to 544
anti-patterns). Each gains exactly the four findings above. Nothing is removed
or rewritten.

The generated browser asset was regenerated with `cargo xtask bundle`.

### Known limits at merge

Recorded when `corpus/integration` merged this branch, from its regression
review, which approved it. The merge had no conflicts: the branch sits on the
integration head, and the generated asset regenerated to the branch's bytes.
Every golden the branch recorded replays against the integrated binary, so
nothing was re-recorded.

1. **The first report wins.** When the rejection arrives before the
   synchronous throw, the snippet keeps `Uncaught (in promise)`. It is still
   one report.
2. **Twins are matched on text only.** Two different throws with the same
   message on one page merge, as they already did on the sync path.
3. **`<anonymous>` names no owner.** A script with no URL (tag manager custom
   HTML, `eval`) now has a source, but the source does not say whose code it
   is. The caller's script sits deeper in the stack and is not printed.
4. **Cluster keys move.** Every script-error snippet gains the marker and the
   type, so corpus cluster keys for script errors change once, and new-vs
   matching reads each of them as a new finding once.
5. **A subclass without `name` prints its class name.** The type is taken from
   the first line of CDP's description, so an `Error` subclass that sets no
   `name` of its own prints the class name found there.
6. **The twin browser test assumes loopback.** `script-error-twins.html` loads
   its script from `localhost` while the page sits on 127.0.0.1, so the test
   assumes `localhost` resolves to loopback.

## Recorded 2026-09-13: the visual passes gate their candidates like the text path (corpus/fix-pixel-pass-gate)

Three fixes from `reports/observations-16.md` in the corpus repo, issues 9, 14
and 25.

- **Candidates pass the text gates first (issue 9).**
  `collect_visual_contrast_candidates` now refuses a candidate before it takes a
  `maxCandidates` slot, asks a hit test, is sampled or is scrolled into view,
  when:
  - its own text has no letter and no digit;
  - it is a disabled control (`[disabled]`, `[aria-disabled="true"]`);
  - its font is under 1px, or its `color` is at alpha 0.02 or less, or its
    `-webkit-text-fill-color` is transparent (a box that clips its own
    background to its text keeps its slot as before);
  - or the painted-at-capture predicate (`painted.rs`, text gate) says it is not
    painted.

  The pixel pass scrolls only candidates the analyses carried, so it no longer
  brings a clipped tab cell or a parked carousel slide into a capture.
- **An image has to be under the text (issue 14).** In the stack walk, a
  background image drawn `no-repeat` (the first layer of the computed
  `background` shorthand) whose painted rect covers neither the candidate's text
  box nor its own box ends the walk as `background image does not cover the
  text`. `body` and `html` are asked only about the text box. This holds at a
  size the capture can place (pixels, percentages, `cover`, or any size once
  the image's intrinsic size is loaded), whether or not the pass could load the
  file. Before, the box's own `background-color` went unread, and the icon's
  transparent pixels composited over whatever sat further down the stack: a
  white count on a dark badge scored 1.1:1 against the page's fill. The
  candidate now goes to the pixel pass. A repeating tile, and a placement the
  capture cannot state, are read as before.
- **Glyph-only text is not scored on any tag (issue 25).** `ColorOpts` gains
  `is_glyph_only`, and the full `check_colors` pass skips contrast for it, as the
  SAFE_TAGS path already did through `paints_own_text`. Both engines set it. In
  both engines a descendant no longer stands down for a glyph-only ancestor that
  shares its colour, so `<p>{ <span>name</span> }</p>` reports the words, not
  the braces. The frozen call vectors pass `false` and replay as recorded.

Goldens, recorded from the binary and reviewed finding by finding:

- `detect-fixture-json-visual-contrast-gates-html`,
  `detect-fixture-text-visual-contrast-gates-html`: new, 3 findings, all
  should-flag cases the static engine can score.
  - `1.3:1 (need 4.5:1) — text #4aab5d on #1d9634`, words at a fifth of white on
    green.
  - `2.1:1 (need 4.5:1) — text #b3b3b3 on #ffffff`, the code line in words.
  - `2.4:1 (need 4.5:1) — text #6d83cb on #1e40af`, the visible launcher.

  The glyph-only circle and braces report nothing; the base binary reports both.
  The URL-only cases leave no static finding.
- `detect-dir-json-all-fixtures` (620 to 623), `detect-no-advisory-json` (540 to
  543), and the text forms `detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures` and `detect-no-advisory-text`: exactly those 3
  added, nothing removed. In the text forms only the new fixture's block and the
  summary count (540 to 543 anti-patterns) change.

No other golden moved: no existing fixture holds glyph-only text on a
non-SAFE_TAGS element with low contrast.

The URL behavior is pinned by
`the_visual_passes_measure_only_text_a_reader_sees_on_its_own_surface`
(`crates/browser/tests/visual_contrast_gates.rs`). The base binary, over the
same fixture, reports seven should-pass cases: the clipped tab cell, the
disabled button, the circle, the braces, the white count in a framing sprite,
the label beside a button icon, and the 0px transparent launcher. The branch
reports none, and every should-flag case flags.

Measured on the corpus (the pixel and sampled passes do not replay, so issues 9
and 14 were checked offline and live):

- **Ratchet over run 16, scan path.** low-contrast 4,039 to 4,013, with 0
  violations.
  - 27 removed, every one glyph-only direct text: `||` and `|` separator rows
    (yna.co.kr), `/` in swiper fraction pagination, `◯` (bt.cn 80918, 81097), `—`
    (adant.ai), `{  }` (context.dev 89422, 89800), `×` on a delete button, and
    symbol-only `code` chips such as `/`, `<*>`, `--` and `[ ]`.
  - Labels on the removals: pattern-absent 2, real-harmless 2, unjudged 23.
  - 1 added: the link inside a `||` separator div that stood in for it.
- **Candidate replay over the run 16 scan snapshots.**
  - 84 of the 97 visual-contrast findings are among base's candidates offline.
    The other 13 depend on hit tests the capture did not record.
  - The branch collector drops 20 of the 84: the yna.co.kr gallery badges (10),
    fedex.com 87961 (0px transparent label), overdrive.health 88446 and 88478
    (disabled), the context.dev tab cells (6, including 89960 and 89961), and a
    context.dev date on a carousel slide past the 390px page.
  - Freed slots admit 3 candidates on 2 captures.
- **Live scans.** Base and branch binaries back to back per page, 16 pages of
  context.dev, yna.co.kr, fedex.com, bt.cn, adant.ai and dograh.com, at 1280x800
  and 390x844.
  - fedex.com timed out on navigation in both.
  - Visual-contrast findings: context.dev mobile 7 to 0 (the tab cells and the
    parked date), yna.co.kr desktop 8 to 1 on two back-to-back runs (the badges),
    adant.ai and dograh.com 0 to 0.
  - Element-pass differences are the glyph-only removals, plus page drift.

### Known limits at merge

Recorded when `corpus/integration` merged this branch, from its regression
review, which approved it. The code merged cleanly on top of
`corpus/fix-script-error-format`: that branch's `ratio_label` in the pixel
snippet sits beside this branch's collector gates in `visual.rs`, and this
branch adds no ratio printer of its own. This file, the generated asset
(regenerated) and the five sweep goldens conflicted. The sweeps were
re-recorded from the integrated binary and compared finding by finding:
`detect-dir-json-all-fixtures` 624 to 627 and `detect-no-advisory-json` 544 to
547, exactly this branch's three `visual-contrast-gates.html` findings, and
against this branch's own goldens (623 and 543) exactly the four
`low-contrast-near-threshold.html` findings. In the text forms
(`detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`,
`detect-no-advisory-text`) only the new fixture's block and the summary count
(544 to 547 anti-patterns) change. Nothing removed.

1. **The budget moves.** A candidate that no longer spends a slot frees it for
   one base never analyzed, so a page can report a visual finding base did not.
   3 entrants on 2 replayed captures; none produced a finding in the live scans.
2. **Symbol-only code chips.** `--`, `<*>` and `[ ]` in a filled `code` chip are
   no longer scored, the way an unfilled one already was not.
3. **One yna.co.kr badge still reports.** The active slide's count, `via
   solid-background`, on desktop. None of its samples read the sprite, so the
   walk did not reach the badge at those points. Not explained here; a slide
   that moves between capture and read would do it.
4. **More pixel reads, and cluster keys split.** A no-repeat image that covers
   neither its box nor the text (a letterboxed `contain` photo, a picture parked
   at one side), or covers only the text, now gets a pixel verdict instead of a
   sampled one, so the snippet names the pixel pass, a screenshot pair is spent,
   and the corpus cluster keys for those findings split from the sampled ones.
5. **API.** `ColorOpts` gains `is_glyph_only`. `css_url_source_point` and
   `css_url_no_image` take the candidate element, and the wasm exports
   `vc_css_url_source_point` and `vc_css_url_no_image` change signature (the
   in-page bundle is regenerated).
6. **Single digits under a glyph-only parent report nowhere.** The parent is
   skipped as glyph-only, and the digit's own element pass returns early under
   10px of width, so neither reports (zigzag.kr pagination; every such finding
   was judged not harmful).
7. **Cards past a horizontal scroller's edge.** The painted gate refuses a card
   parked past the scroller's clip, so it loses its visual findings.
8. **A sprite under a translucent badge.** An icon sprite beneath the glyphs of
   a translucent badge reports nothing.

## Recorded 2026-09-13: the ink blend skips reveals caught mid-frame

`corpus/fix-transient-opacity` stops low-contrast from folding the opacity of
a box caught in the middle of a reveal into the ink. Round 3's blend scored
Framer's word-by-word reveals at whatever frame the scan landed on: each word
parks at `opacity: 0.001; filter: blur(10px); transform: translateY(10px);
will-change: transform` and animates in, and a word read a few frames in
(landio.framer.website 85169 at opacity 0.073 with a 9.3px blur) came out as
nearly invisible. One testimonial produced 14 findings, one per word.

The URL engine's fold (`fold_surface_opacity` in
`crates/core/src/browser/element_checks.rs`) now asks, for every faded box
between the text and its surface, whether that box is at rest
(`opacity_at_rest`). A box is not at rest when:

- an animation or transition running on it at capture moves its `opacity`
  or its `filter`;
- it carries a `filter: blur()` with a radius above 0 (or a radius the
  engine cannot read);
- its opacity is under 0.1 while a `transform` other than the identity, a
  `translate`, `scale` or `rotate` other than `none`, or a `will-change`
  naming one of them, `opacity` or `filter`, moves it.

A box that is not at rest contributes no fade, so the text is scored at its
declared colour, as before round 3. Boxes around it that are at rest still
fade the ink: a word mid-reveal inside an accordion item held at 0.5 is
scored at the item's fade. The painted gate and the 0.02 floor read the real
opacity as before.

The visual-contrast collector skips a candidate with a box caught mid-reveal
in its ancestry (`caught_mid_reveal`). Without that, the pixel pass read the
same frame from pixels once the element pass stopped reporting it: in the
fixture, the sliding row and the heading faded by a running CSS animation
came back as `pixel contrast ... on opacity stack`.

Running animations are a new capture fact. `15-snapshot.js` records, per
element, the properties of every `document.getAnimations()` entry that is
running or pending and targets the element itself (`an`), and marks the
snapshot as having looked (`anim: true`). `Dom::running_animation_properties`
answers `None` where the capture did not look, which is every recording made
before this change; there the blur and reveal-opacity markers still apply and
anything else is at rest. The in-page probe (`10-probe.js`) answers the same
question from `el.getAnimations()`.

The static HTML engine is unchanged. Its cascade carries no `filter`,
`transform` or `will-change` and nothing runs, so it keeps blending declared
opacity.

The new fixture `transient-opacity-contrast.html` pairs four should-flag cases
(an accordion header in an item faded at rest, a card held faded with
`will-change`, a label with an opacity transition at rest, a faint word
mid-reveal scored at its declared colour) with four should-pass cases (a dark
word mid-reveal, a row sliding in from 0.05, a heading faded by a running CSS
animation, a word faded by a running Web Animations API animation). The
browser test `the_url_engine_blends_only_the_opacity_at_rest`
(`crates/browser/tests/transient_opacity.rs`) pins that the URL engine reports
exactly the four should-flag cases; the base binary reports all eight, the
pass column at blended colours.

Goldens recorded from the binary and reviewed finding by finding:

- `detect-fixture-json-transient-opacity-contrast-html`,
  `detect-fixture-text-transient-opacity-contrast-html`: new, six findings.
  The three faded-at-rest cases (`text #85888a`, `#94989c`, `#a4a4aa on
  #ffffff`). The static engine also blends the two cases marked "URL engine"
  (`1.6:1 (need 4.5:1) — text #cecfd1 on #ffffff`, the dark word, and
  `1.1:1 (need 4.5:1) — text #f3f3f3 on #ffffff`, the sliding row) and scores
  the faint word at its blended colour (`1.1:1 (need 4.5:1) — text #efeff0 on
  #ffffff`) where the URL engine prints the declared `#b0b1b2`. The two running
  animations read their declared opacity of 1 there and pass.
- `detect-dir-json-all-fixtures`, `detect-no-advisory-json`: exactly those six
  added, nothing removed or rewritten.
- `detect-dir-text-all-fixtures`, `detect-no-advisory-text`,
  `detect-dir-quiet-all-fixtures`: the same six, and the count moves from 540
  to 546.

The generated browser asset was regenerated with `cargo xtask bundle`.

On the run 16 recordings the ratchet removes 21 low-contrast findings and adds
none: 19 labelled pattern-absent and 2 disputed, no confirmed-harmful removal.
They are the landio.framer.website testimonial words 85143 to 85151 and 85163
to 85169 (85164 and 85166 unlabelled, same cluster), aisupply.framer.website
85024 and 85128 (opacity 0.21 with a 7.9px blur), and kraflio.com 84526, 84529
and 84532 (a message row at 0.028 sliding 19px). The faded Framer accordion
headers (aisupply 84895, 85025, 85204, 85205, #85888a on a computed #0a1015)
stay. The ratchet replays the element pass only; the collector change is
measured on live scans.

### Known limits at merge

Recorded when `corpus/integration` merged this branch, from its regression
review, which approved it. `crates/core/src/browser/visual.rs` conflicted in
the collector, where `corpus/fix-pixel-pass-gate` added its text gates
(`candidate_text_reads_at_rest`) at the same spot this branch added
`caught_mid_reveal`. Both are kept, the text gates first, so a candidate passes
both before its reasons are read or it takes a slot. `element_checks.rs` merged
cleanly. The snapshot lists still match between `15-snapshot.js` and
`snapshot.rs` (115 style and 16 pseudo properties, each once; `filter`,
`transform`, `translate`, `scale`, `rotate` and `willChange` were already among
them), and `an` and `anim` are each written once and read once. This file, the
generated asset (regenerated) and the five sweep goldens conflicted. The sweeps
were re-recorded from the integrated binary and compared finding by finding:
`detect-dir-json-all-fixtures` 627 to 633 and `detect-no-advisory-json` 547 to
553, exactly this branch's six `transient-opacity-contrast.html` findings, and
against this branch's own goldens (626 and 546) exactly the four
`low-contrast-near-threshold.html` and three `visual-contrast-gates.html`
findings. In the text forms (`detect-dir-text-all-fixtures`,
`detect-dir-quiet-all-fixtures`, `detect-no-advisory-text`) only the new
fixture's block and the summary count (547 to 553 anti-patterns) change.
Nothing removed.

The ratchet over run 16 with all three branches integrated removes 51
low-contrast findings and adds 4 (4,039 to 3,992, 0 violations), the exact sum
of the three branch ratchets (27 and 1, 21 and 0, 3 and 3), label for label.

1. **Reveals above 0.1 with no blur on old recordings.** A box sliding in at
   0.3 opacity with no blur and no recorded animation still blends. Old
   recordings catch a reveal only through a blur or near-zero opacity with a
   transform; new captures catch the rest through the running-animation fact.
2. **Running decorative loops.** An infinite opacity or filter animation (a
   pulsing status badge) is scored at its declared colour, never at the faded
   colour it passes through.
3. **Scroll-driven animations.** An animation on a scroll or view timeline is
   `running` while attached, so a box it holds faded at the capture's scroll
   position is scored at its declared colour.
4. **Static engine.** It blends the declared first frame of a reveal
   (`opacity: 0.2; filter: blur(8px)`) because its cascade carries no filter or
   transform.
5. **API.** `Dom` gains `running_animation_properties` (default `None`),
   `Snapshot` gains `animations_recorded` and `SnapNode` gains `animations`,
   and `FakeDom` gains `set_running_animations`.
6. **A near-zero watermark with a transform is dropped.** A decorative box held
   under 0.1 opacity with a transform (a rotated watermark, a scaled background
   word) reads as a reveal, so its fade is not blended and the visual collector
   skips it.
7. **The visual collector drops a candidate mid-reveal.** It does not fall back
   to the declared colour the element pass uses; the candidate leaves the pixel
   and sampled passes.

## Recorded 2026-09-13: low-contrast skips covered text and surfaces the walk never read (corpus/fix-covered-text)

Corpus run 16, issues 6, 32 and the hit-test half of 2. All three changes are
in the URL engine's element pass, which no golden runs. The goldens move only
because `covered-text-contrast.html` joins the fixture directory.

- **Covered text is not scored.** `impeccable_core::browser::text_layers`
  reads `elementsFromPoint` at the points the occlusion grid asks for the same
  box (`occlusion_probe_points`, now shared with `check_text_occlusion_dom`).
  A live scan answers them in the same `resolve_needs` round, and a recording
  made for text-occlusion already holds them. Where every deciding point has
  an image, a video, a canvas or an opaque fill at full opacity above the text
  (not the element, its descendant or its ancestor), the text is covered at
  capture and `low-contrast` prints no verdict. Examples: the hero stats under
  hrsimple.app's fixed consent banner, a photo avatar over an SVG initial.
- **No verdict against a surface the walk never read.** Between the text and
  the box that ended the background walk, the same stacks name paint that is
  nobody's ancestor. Where every deciding point has such paint that could
  really change what the text sits on, or a layer above, no verdict is
  printed. That paint is a picture (an `img`, `video`, `canvas` or SVG
  `image`, or a `url()` background drawn `no-repeat`, `cover`, `contain`, at a
  percentage or larger than 256px), a gradient or fill that moves the named
  colour by more than 24 summed over the channels once composited over it,
  or, where the walk named no single colour or only reached the page ground
  (`html`, `body`, the canvas), a gradient, fill or SVG shape it cannot
  compare. A candidate of the pixel pass is still measured there; nothing is
  added to its candidates.
- **Texture is not a surface.** A layer under the text is read past, and the
  walk's surface stands, where it is decoration: a gradient drawn in cells of
  64px or less on both axes, a `repeating-*` gradient, a gradient whose stops
  are mostly transparent or that pairs a transparent stop with a stop within
  3px (dot grids, hairline grid lines), a `url()` repeated at a drawn size of
  256px or less (a grain tile), anything with a `mask-image`, and a picture,
  gradient or shape at an effective opacity of 0.3 or less. Paint at an alpha
  times opacity of 0.1 or less is a wash and is read past too, and so is a
  detached fill in the named colour. A layer this cannot place (a remote
  image tiled at its own size, an `image-set`) keeps the verdict.
- **Two capture columns.** `STYLE_PROPS` and `browser-bundle/15-snapshot.js`
  gain `maskImage` and `webkitMaskImage`. A recording made before them reads
  both as empty, so on replay a masked layer is placed by its other styles.
- **Covered text is no visual candidate either.** The visual-contrast
  collector skips a candidate the stacks say is covered. Without that, text
  the element pass stood down on reached the sampled pass, because the visual
  fallback had skipped it only for carrying a finding already (run 17 below).
- **Fail safe.** The run has to lie wholly inside the viewport, every point
  has to be answered, and at least half the points have to find the text in
  the stack. Otherwise the verdict stands as before. A run part under a layer
  and part over the named surface also keeps its verdict. The SAFE_TAGS path
  asks before the page claims a colour pair, so the first uncovered link
  wearing the colour reports instead. Placeholder hits pass the same test.
- **text-occlusion ignores answers the capture disagrees with.** A decorated
  box counts as an occluder only where its captured rect holds the probe
  point. On co-trip.jp (80067, 80068, 80069) the Swiper track advanced between
  the capture and the answers, and the next slide's card, wearing the
  caption's own card classes, was named as the occluder. The engine already
  skipped ancestors; the card was not one. A box whose own text overflows its
  rect still counts, as text.

Replay: a recording made before this change answers only the points it asked.
The grid points of every text element text-occlusion probed are there, so
most above-the-fold text is decided on replay. Anything else (a one-letter
initial, text the occlusion collector skips) is unanswered, the verdict
stands, and the points count toward `unanswered_hit_tests`.

Goldens, recorded from the binary and read by hand:

- `detect-fixture-json-covered-text-contrast-html`,
  `detect-fixture-text-covered-text-contrast-html`: new. The static engine runs
  no hit tests and no script, so it reports the should-pass copy too:
  fourteen `low-contrast` plus one `flat-type-hierarchy`. In the should-flag
  column: `#aaaaaa` four times, `#475569` on the dot grid's `#0f172a`,
  `#52525b` on the grain section's `#18181b`, and `#9a9a9a` once for both
  links, since the static engine claims the pair on the covered one. From the
  should-pass copy: the two white initials, the two `#f0f0f0` lines, the white
  hero title on `#f6f7f8`, the banner copy (`#aaaaaa`) and the `#ffffff` on
  `#ff7145` CTA under the banner. The hierarchy snippet lists the h3 at 22px.
- `detect-dir-json-all-fixtures`, `detect-no-advisory-json` (620 to 635, 540
  to 555), `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`,
  `detect-no-advisory-text` (540 to 555 anti-patterns),
  `detect-scope-type`, `detect-scope-both` (159 to 160, 207 to 208, the
  `flat-type-hierarchy`): exactly the new fixture's findings, nothing removed.

The URL behavior is pinned by `crates/browser/tests/covered_text.rs`, and the
fixture joins `REPLAY_FIXTURES` in `crates/browser/tests/evidence.rs`. Scanned
live, the base binary (edfe602b) reports `low-contrast` on all eight
should-pass copies (the covered link and CTA, both initials, the photo and
dark-layer lines, the hero title over the photo and the banner copy) and
`text-occlusion` on both advancing captions, and misses
`#flag-uncovered-link`. The branch reports every should-flag case, the dot
grid and grain copy included, and none of the should-pass ones.

Corpus, replay of run 16 (344 captures): `low-contrast` 4,039 to 3,955 (84
removed, 0 added), `text-occlusion` 49 to 46 (3 removed, all pattern-absent).
Every removal was read against its capture's recorded stacks:

- 10 confirmed-harmful, all hrsimple.app text under its fixed consent banner:
  the nine covered findings named in issue 6 (85560 to 85562, 85616, 85618 to
  85620, 85929, 86059), and 85617, the mobile CTA under the same banner.
- 14 pattern-absent: donckelektro.nl hero copy over the hero photo (83479,
  83510, 83936, 84067), and yungching.com.tw key-visual titles over the
  slideshow images (81352 to 81355, 81396 to 81399, 81631, 81763).
- 60 unjudged. Read against their stacks:
  - donckelektro.nl's hero title over the hero photo (4);
  - yungching.com.tw's large key-visual line over the slideshow (2);
  - zigzag.kr's banner slide titles over their photos (6), and two prices
    under the white bottom app bar;
  - volkswagen-group.com's hero titles over a playing video (4), and the
    slider copy under the cookie modal (2);
  - thairath.co.th's copy under a full-viewport fixed video layer (4);
  - billia.app's section title under the fixed mobile bar (1);
  - bt.cn's tab label over the green gradient pill the walk read as grey (2).

  Not read one by one, all white text scored 1.0:1 on a named white:
  shipthatcode.com (10), ynet.co.il's consent-dialog link text (6), te.eg's
  cards (4), ktb.gov.tr's slider (4), aisupply.framer.website (3), att.com,
  nubank.com.br and samsung.com (2 each).

### Revised after review: texture is not a surface

At 27c60971 any hit-testable detached layer that painted a gradient or an
image set the verdict aside. A `position: absolute; inset: 0` dot grid, grid
lines, masked grid or grain tile under failing text therefore suppressed the
verdict even where the walk had named the right surface, the section's own
fill. That is the common shape of an AI-built landing page, and the corpus is
weighted toward it. The texture rules and the page-ground-or-differs test
above are the revision. Review pages served locally and scanned with release
binaries (`low-contrast` only):

| Page | base (edfe602b) | 27c60971 | revised |
|---|---|---|---|
| a-dot-grid: `#475569` over a 24px dot layer on `#0f172a` | 2.4:1 | none | 2.4:1 |
| l-dark-grid: `#ffffff26` 40px grid lines over `#020617` | 2.7:1 | none | 2.7:1 |
| n-masked-grid: masked light grid at `z-index: -1` on white | 2.5:1 | none | 2.5:1 |
| d-noise: grain tile at 0.2 on `#18181b` | 2.3:1 | none | 2.3:1 |
| o-link-dots: a link over dots | 1.7:1 | none | 1.7:1 |
| f-faint-image: `img` at 0.2 | 3.5:1 | pixel 2.0:1 | 3.5:1 |
| b-svg-pattern: SVG grid pattern, walk on the page ground | 2.5:1 | pixel 2.5:1 | pixel 2.5:1 |
| e-card-layer, j-dark-overlay-card: opaque panel, walk on the page ground | 2.1:1, 2.3:1 | none | none |
| c-blur-blob, g-svg-ring, i-control, k-ibelick-light-dots, m-dots-pe-none | as base | as base | as base |

Against 27c60971 the replay of run 16 returns ten findings and removes none
more:

- bt.cn's hero copy over a banner image at opacity 0.2 (80545, 80546, 80664,
  80665): texture by opacity, and the walk's `#fdfbfe` stands.
- donckelektro.nl's orange eyebrow (83229, 83279). It sits on the left of a
  `from-background` gradient whose opaque stops are the section's own
  `#f6f7f8`, so the walk named what it reads on. The pixel pass measured it at
  the same 2.5:1 live. The white hero title and subline over the photo stay
  removed.
- hrsimple.app's 4.1:1 subline (85736, 85928, 85980, 86058, confirmed-harmful).
  Two gradient tints lie above the 0.4 photo, and the walk named a gradient
  that they cannot be compared with, so the verdict stands. The pixel pass
  measured the subline at 4.2 to 4.3:1 live.

Violations drop from 14 to 10, and all ten are covered text. The review's live
scans were repeated with the revised binary: hrsimple.app 37 findings as at
27c60971 (base 40); usebidflow.com, overdrive.health, stroq.dev and
useautumn.com unchanged; co-trip.jp 69, differing only in the order of four
same-snippet findings that base reports too.

Live, the six sites of the issues recaptured with the engine at 27c60971, 34
captures each time, against run 16:

- **Run 17** exposed the visual-pass leak. hrsimple.app's stat labels under
  the consent banner left the element pass and came back as `browser contrast
  2.5:1 ... via analytic-gradient`. The covered-candidate gate above closes it.
- **Run 18**, with the gate. Replaying its captures with the base engine
  (edfe602b) isolates the engine from page drift. The branch drops 24
  element-pass `low-contrast` findings and adds none. hrsimple.app has 14:
  seven stat labels and three CTAs under the banner, and the hero subline
  over the 0.4 photo four times. donckelektro.nl has 10: hero copy over the
  hero photo. Every other site's element pass is identical to base on the
  same captures.
- **Pixel pass on dropped verdicts.** hrsimple.app's subline was measured by
  pixels (4.3:1 desktop, 4.2:1 mobile), and donckelektro.nl's orange eyebrow
  at 2.5:1 on its img underlay. The revision gives both their element
  verdicts back (above). In run 17 the pixel pass
  measured haraj.com.sa's circle initials at 1.9:1 where the element pass had
  scored them on the card.
- **text-occlusion:** co-trip.jp 3 to 0 in both runs, hrsimple.app unchanged.
- **Unchanged below the fold:** becomeautonomous.com (31 findings) and the
  yna.co.kr marquee copy. yna.co.kr and haraj.com.sa report fewer findings in
  run 18 than in run 17 on the same text volume. The base engine replays those
  captures to the branch's findings, so that drop is page drift.

### Known limits at merge

1. **Covered text hides real failures.** A consent banner that covers copy at
   capture hides a failure a visitor meets once they dismiss it. All ten
   confirmed-harmful removals in run 16 are this: hrsimple.app copy under its
   fixed cookie banner. A later fix could dismiss or hide known consent
   overlays before the rule pass, since the validity gate already recognizes
   OneTrust, Cookiebot and Usercentrics.
2. **Only the viewport.** Points outside it cannot be asked, so covered or
   layered text below the fold keeps its verdict: the yna.co.kr marquee copy
   (80094, y 949), becomeautonomous.com's tabs (88167, 88316) and zid.sa
   (82034, 82212) are unchanged.
3. **Hit testing skips `pointer-events: none`.** A layer that opts out of hit
   testing is invisible to the stacks, and text that does is silent.
4. **Pseudo-elements.** A `::before` or `::after` layer is answered as its
   originating element, whose own style is what is read.
5. **A transparent picture over text counts as covering it.** An `img` at full
   opacity is read as opaque, whatever its pixels.
6. **Pixel pass reach.** Its candidates skip links and spans and stop at twelve
   per page, so most text whose verdict is dropped here is not measured again.
   Skipping covered candidates frees slots for the next ones in document
   order, which can bring a new sampled verdict onto a page (hrsimple.app
   mobile gained `browser contrast 4.5:1` on a muted card line in run 18).
   Its samples still ignore unread layers beneath a candidate.
7. **FakeDom tests.** Two link-contrast unit tests declare the stacks a browser
   would answer, since FakeDom lists elements in document order. One of them
   now also asserts that a later photo at the text's layer, in the viewport,
   covers the link.
8. **Texture is read from style.** A hero photo faded out under a
   `mask-image`, or ghosted at 0.3 or less, counts as decoration and keeps
   the walk's verdict. A scrim whose stops are mostly transparent is read past
   to what lies under it. A remote image tiled at its own size is undecided.
   Captures older than the two mask columns read no mask.
9. **Opaque panels on the page ground.** Where the walk only reached the page
   ground, an opaque detached panel in another colour still sets the verdict
   aside, even where the text fails against the panel as well (the review's
   e-card-layer and j-dark-overlay-card), and the pixel pass does not measure
   either page's copy.
10. **Tints over a photo.** Where gradient tints lie above a photo and the walk
   named a gradient, the tints decide and the verdict stands, whatever the
   photo does to the surface (hrsimple.app's subline).

## Recorded 2026-09-13: evidence origin, scan identity and React invariants (corpus/fix-evidence-origin)

`corpus/fix-evidence-origin` fixes three things a corpus judge saw wrong in the
URL engine's evidence and one in its script-error snippets. None of them is in
a file scan, so no existing fixture's output moves; the goldens move only
because three fixtures were added.

- **The screenshot's document origin.** A document that scrolls from the right
  (agora.co.il sets `direction: rtl` on body) keeps its overflow at negative
  document x, and a beyond-viewport capture starts at the left edge of that
  overflow. The engine measures the furthest left the document scrolls
  (`GEOMETRY_JS`, by scrolling there and straight back) and records the
  image's left edge as `Screenshot::origin_x`: 0 for most pages, minus the
  overflow here. Past the 4,096px width cap the capture starts further right,
  so the viewport stays in the image. Stitched captures step their columns
  from the origin, and `needs_element_shot` compares image x. Pinned by
  `fullpage.rs` unit tests and `a_right_to_left_page_records_where_its_screenshot_starts`
  in `crates/browser/tests/fullpage_screenshot.rs`.
- **Rects from the element the scan flagged.** `capture_post_scan` resolved
  each flagged selector with `querySelector`. A generated selector names one
  element unless an id anchors it and the page repeats the id, and d3shop.ae
  has three inputs with one id, two collapsed, so finding 108250's rect and
  crop came from a hidden copy. The scan now carries each result's element in
  its capture, and for an id-anchored selector the capture matched more than
  once the evidence takes the same match (`[n, count]`, used while the page
  still has `count` matches, `querySelector` otherwise). Element shots resolve
  the same way. Pinned by `evidence_measures_the_element_the_scan_flagged_when_an_id_repeats`
  in `crates/browser/tests/evidence.rs`.
- **React invariants.** `Minified React error #418; visit <url> ...` reads
  `Minified React error #418: Hydration failed because the initial UI does not
  match what was rendered on the server.` for 14 common numbers
  (`REACT_INVARIANTS` in `cdp.rs`, specified in `CLI-CONTRACT.md`). Pinned by
  `a_react_invariant_reads_as_its_message` and
  `a_production_react_invariant_reads_as_its_message`.

New goldens, recorded from the binary and read by hand:
`detect-fixture-json-fullpage-screenshot-rtl-html` and the text form
(`low-contrast` on the faint copy, `overused-font` for Arial);
`detect-fixture-json-evidence-duplicate-id-html` and the text form (the static
engine measures no boxes, so it reports all six faint copies, collapsed ones
included, plus `overused-font`); `detect-fixture-json-script-error-react-html`
and the text form (nothing: the static engine runs no script).

Re-recorded sweeps, compared finding by finding against the previous goldens;
every added finding belongs to the two new fixtures that report, and nothing
was removed: `detect-dir-json-all-fixtures` 648 to 657 and
`detect-no-advisory-json` 568 to 577 (9 added each), `detect-scope-type` 160 to
162 and `detect-scope-both` 208 to 210 (the two `overused-font`). In the text forms
(`detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`,
`detect-no-advisory-text`) only the two fixture blocks were added and the
summary moved from 568 to 577 anti-patterns.

The corpus harness reads the origin behind `cfg(engine_screenshot_origin)`
(committed from the corpus side), and `judge/prepare.ts` and `judge/tiles.py`
place tiles by it, recovering it for older captures.

### Known limits

1. **The origin is measured by scrolling.** Only a page that scrolls from the
   right moves, sideways and back in one task, but a horizontal scroll handler
   on such a page could still react before the capture.
2. **The pixel contrast pass is unchanged.** Its clips floor document x at 0,
   and on a page that scrolls from the right a candidate in the overflow is
   likely measured at the wrong pixels. Not measured on the corpus.
3. **Identity reaches the evidence only.** The pixel pass and its scroll into
   view still resolve a repeated id with `querySelector`, so they can measure
   a collapsed copy.
4. **Visual-contrast results on a repeated id.** Their element is known only
   when every analysis on that selector gives the clip of one match; otherwise
   the evidence resolves the selector as before.
5. **React wording.** The table carries React 18's messages; React 19 words
   418 slightly differently, and the decoded snippet drops the decoder URL
   with its `args[]`. Unknown numbers keep React's text.
6. **Older captures.** `prepare.ts` recovers the origin from body or html
   `direction: rtl` and the image width; a vertical writing mode that scrolls
   from the right is not recovered, and the labeler reads only a recorded
   origin.
7. **CLI-CONTRACT.** It describes no evidence JSON, so the origin is documented
   on `Screenshot::origin_x` rather than there.

### Known limits at merge

`corpus/integration` merges this branch first of round 5, with no conflicts;
every golden it recorded replays as recorded.

1. **Pixel pass on a right-to-left overflow page.** The pixel contrast pass
   still floors clip x at 0, so a candidate in the negative-x overflow is
   measured at the wrong pixels (limit 2 above).
2. **Repeated ids outside the evidence.** The pixel pass and its scroll into
   view still resolve a repeated id with `querySelector` (limit 3 above).
3. **A clipped right-to-left root.** With `overflow-x: hidden` on both html and
   body and `direction: rtl`, the document does not scroll sideways, so the
   screenshot is only viewport-wide and overflow nobody can scroll to falls
   outside it.
4. **The probe scrolls in one task.** The origin probe scrolls sideways and
   back within one task (limit 1 above); a scroll handler that reads position
   synchronously can still see it.

## Recorded 2026-09-13: every rule that scores what a visitor sees asks the painted predicate (corpus/fix-painted-gate-coverage)

Corpus run 20 (cohort 2), `reports/observations-20.md` rows 18, 27 and 49. All
changes are in the URL engine. The goldens move only because
`painted-gate-coverage.html` joins the fixture directory.

- **Seven more rules skip what nobody sees.** `paint_gate` adds
  `bounce-easing`, `dark-glow` and `ai-color-palette` to the Box gate and
  `italic-serif-display` to the Text gate. `blinking-cursor` gets its own
  Toggle gate: the cursor's own opacity, and a `visibility: hidden` its parent
  does not share (with the `checkVisibility()` false that follows), do not hide
  it, so a cursor caught between blinks still reports; what its ancestors do,
  its clips and the document edges do hide it. `ai-color-palette` kept a model
  of its own that read `display` and `visibility` only; the shared predicate
  now decides, so a violet layer at `opacity: 0` no longer reports.
- **kicker-above-heading** asks the Text gate about the heading and about the
  label above it. A pair in a section at `hidden` (demotv.lol) or on an
  inactive slide (exxonmobil.com) is not on screen.
- **Page-level forms.** A `bounce-easing`, `dark-glow` or `pulsing-dot` form
  from the style text reports only when at least one element its selector
  matches is painted. The match is asked on the base predicate with no area
  test, because the selector may name a pseudo-element whose host has no box.
  A form that carries no selector (an inline `style` attribute) is unchanged.
- **text-occlusion** keeps `is_painted_for_occlusion` and adds the shared
  predicate for the covered text, for every hit-test answer that covers it,
  for the cards a headline overhangs and for inline leak hosts, so it only
  removes. The items of a closed `<details>` have boxes but are hidden on
  `::details-content`, which no ancestor style shows; `checkVisibility()`
  answers false for them. It also skips text under a `filter: blur()` of 4px
  or more on the element or an ancestor (a teaser behind a sign-in gate), and
  text with fewer than two characters on screen (a lone emoji is two UTF-16
  units). What covers the text is named for what it draws: `an SVG graphic`
  for an SVG element that is not text (it printed `overlapping text`), and `a
  bordered element` for a box counted through its borders with no opaque fill
  (it printed `an opaque element`). Counts and thresholds are unchanged.

Not changed, and why:

- **Row 49's rotating tagline** on agora.co.il (108272, 108321) is painted at
  capture, and the page image read at the RTL offset shows the header buttons'
  labels printed over it. The judges saw crops cut from the shifted RTL
  screenshot (observations-20 section 5). It still reports.
- **Covered, not hidden.** auradeballet.com's welcome dialog and install
  banner over dark-glow (104329, 104409, 104334, 104438), italic-serif-display
  (104328) and ai-color-palette (104698) need a cover test, not a paint test.
- **visiby.net's dark-glow form** (106376, 106967) comes from an inline
  `style` attribute and has no selector to test.

Goldens, recorded from the binary and read by hand:

- `detect-fixture-json-painted-gate-coverage-html`,
  `detect-fixture-text-painted-gate-coverage-html`: new, 17 findings. The
  static engine measures no boxes and reports what it can see in both columns:
  `dark-glow` 4 (both CTAs, the bar and the bar's style-text form),
  `bounce-easing` 3 (the visible row, the loader's `animation: pgc-bounce`, the
  unrevealed row's bezier), `kicker-above-heading` 3, `italic-serif-display` 2,
  `pulsing-dot` 2 (`.live-dot` and `.pass-rail .pgc-node::after`),
  `skipped-heading` 2 (the two overhang titles) and `tight-leading` 1.
- `detect-dir-json-all-fixtures` 648 to 665, `detect-no-advisory-json` 568 to
  582, `detect-scope-type` 160 to 168, `detect-scope-both` 208 to 216, and in
  the text forms (`detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures`, `detect-no-advisory-text`) only the new
  fixture's block and the summary counts (568 to 582 anti-patterns, 80 to 83
  advisory notes). Every added finding belongs to the new fixture. Nothing was
  removed.

The URL behavior is pinned by `crates/browser/tests/painted_gate_coverage.rs`,
and the fixture joins `REPLAY_FIXTURES` in `crates/browser/tests/evidence.rs`.
Scanned live, the base binary (5ce750e5) reports all 17 should-pass cases (two
cursors, two bounce-easing elements, the faded glow, the parked ramp, the
hidden italic heading, two kickers, four page forms and four text-occlusion
victims), and names the SVG and field occluders `overlapping text (rect)` and
`an opaque element`. The branch reports none of the should-pass cases, every
should-flag case (a cursor showing, one paused in the off phase of an opacity
blink and one of a visibility blink included), and the two new labels.

Corpus ratchets:

- **Cohort 2, run 20** (244 captures): 81 removed, 3 added, 18 violations.
  - ai-color-palette 11 (confirmed-harmful 8, real-harmless 2,
    pattern-absent 1): fabadda.com's game-card overlays, `absolute inset-0
    from-game-primary/60 ... opacity-0`, a hover layer at opacity 0 (103503,
    103504, 103872, 103873, 103876, 103877, 103879, 103880), the `-top-3` cyan
    badge on a card past the carousel clip (103498, 103874), and joongang.co.kr's
    slide parked past its track (108213).
  - blinking-cursor 2 (pattern-absent): copperhead.sh's caret (105195, 105283)
    sits 1px past the typing span that clips it.
  - bounce-easing 15 (pattern-absent 10, disputed 3, real-harmless 2). Among
    the 12 samples: centene.com's `cmp-loader` at `display: none` (5),
    progressive.com's closed flyout and the scroller inside it (108863,
    108864), visiby.net's unrevealed inbox row at opacity 0 (105794, 106275),
    and fabadda.com's `bounce-in` cards past the header's clip (103502,
    103875, 103878).
  - dark-glow 4 (pattern-absent): swipeloan.in's `on dark page` form, whose
    selector matches only the chat widget's channel at `display: none`.
  - kicker-above-heading 6 (confirmed-harmful): demotv.lol's `section[hidden]`
    pairs (106695, 106697, 107056, 107058) and exxonmobil.com's inactive hero
    slide (108901, 108927).
  - pulsing-dot 1 (confirmed-harmful): tryrote.com's stepper rail node, not
    drawn at 390px (104987).
  - text-occlusion 42 removed, 3 added (confirmed-harmful 3, pattern-absent
    37, real-harmless 2): demotv.lol's closed `<details>` menus and the
    headline overhanging their hidden dropdown, fabadda.com's emoji (103835),
    auradeballet.com's blurred teaser (104596, 104597), and the three
    v0-dashboard relabels below.
  - The 18 violations are not really wrong removals. 15 carry a label
    inherited from their cluster: both judges called the representatives
    pattern-absent where they were judged (104987, 106695 and 108901 are all
    `hidden-or-offscreen`), and the fabadda.com overlays share a cluster with
    violet surfaces a visitor sees. The other 3 are v0-dashboard-ui-redesign-nine
    (105020, 105022, 105025), which still report with the new labels (`a
    bordered element`, `an SVG graphic (path)`, `an SVG graphic (rect)`), the
    3 added.
- **Cohort 1, run 19** (342 captures): 159 removed, 0 added, 0 violations, all
  unjudged. ai-color-palette 49 (nubank.com.br headings on swiper slides past
  the clip, thairath.co.th's fixed player bar at opacity 0, sapo.vn's
  registration button in the off-canvas menu), bounce-easing 94 (samsung.com's
  `cm-loader` at `display: none` among the samples), dark-glow 4, pulsing-dot 6
  (albayan.ae's `.iz-news-hub-sticky-red::before`, whose host is at
  `display: none`) and text-occlusion 6 (bitroad.ai's closed mobile `<details>`
  menu).
- Crops opened: 103503, 103876, 103502, 105794, 108213, 105195, 103835, 104596,
  106708, 107069, 107071, 95151, 95902, 102927. Ten show nothing of the
  removed finding at rest: the game cards without their violet hover overlay,
  the empty inbox row, the sliver of a parked slide, the empty caret box, the
  blurred teaser, the headline with no card beside it, and page text where the
  closed menus' items would be. 103835 shows the lone emoji the finding scored
  as covered text. Three show state that changed after the capture: 103502 and
  95151, carousel slides the element shot scrolled into view, and 95902,
  thairath.co.th's bar, shown when the crop was taken while it sat at opacity 0
  in the scan snapshot.

The generated browser asset was regenerated with `cargo xtask bundle`.

### Known limits

1. **Hover and reveal layers.** A layer held at opacity 0 until hover or
   reveal no longer reports its palette, glow or motion, although a visitor
   meets it on interaction.
2. **Carousel slides past the clip.** A slide parked past its track loses its
   palette, glow, motion, kicker and italic findings, as its text findings
   already did. A crop that scrolls the track shows it.
3. **State at capture decides.** A bar a script shows and hides (thairath.co.th)
   is judged by the capture's opacity.
4. **Cover is not paint.** Content under an open dialog or a banner still
   reports.
5. **Page forms without a selector** (inline `style` attributes) still report,
   whether or not anything they style is painted.
6. **A cursor switched off for good.** A caret held at opacity 0 by a finished
   `forwards` animation, with nothing else hiding it, still reports: the
   capture records running animations, not finished fills.
7. **Characters on screen are approximated.** Combining marks, variation
   selectors, emoji modifiers and tags, zero-width joins and regional-indicator
   pairs count with the character before them; other sequences (Hangul jamo,
   Indic clusters) count per code point, which only keeps more text.
8. **Cluster keys move once** for text-occlusion findings covered by SVG shapes
   or border-only boxes, whose snippets change.

### Known limits at merge

`corpus/integration` merges this branch after `corpus/fix-evidence-origin`.
Only DELTAS.md and the seven sweep goldens conflicted; the source merged
cleanly and the regenerated browser asset matched the merged one. The sweeps
were re-recorded from the integrated binary and moved by exactly the branch's
delta (17 findings in the JSON sweep, 14 without advisories, 8 in each scope
sweep), the text summaries reading 577 to 591 here against 568 to 582 there.

1. **Past a horizontal scroller's visible edge.** A tile parked past a
   scroller's clip no longer reports its palette, glow, motion, kicker or
   italic findings (context.dev's carousel cards), consistent with the text
   rules. The eight fabadda.com ai-color-palette removals in run 20 (103503,
   103504, 103872, 103873, 103876, 103877, 103879, 103880), counted above as
   hover layers, are this case in the scan snapshot: their overlays compute
   opacity 1 and sit past the right edge of the hero header's
   `overflow: hidden` box (x 1767 in a header ending at 1256 on desktop; x 378,
   726 and 1074 in one ending at 378 on mobile). Their cluster's judged
   representatives, the hero eyebrow pill (103466) and the Arcade chip
   (104282), still report.
2. **Faded by its own animation.** An element an animation holds at opacity 0
   is judged by the moment of capture, so one caught faded out is silent and
   one caught faded in reports.

## Recorded 2026-09-14: low-contrast resolves the surfaces cohort 2 misread (corpus/fix-surface-resolution)

Corpus run 20, `observations-20.md` rows 17, 22, 38, 42 and 43, and the
low-contrast misses in `walkthroughs-20.md`. Fortune-stratum precision was 0.37
on thecignagroup.com, nike.com and exxonmobil.com, all surface misreads.

- **A `display: contents` box paints nothing (row 17).** The contrast walk,
  the gradient-stops walk and the visual pass's reasons walk read past it, in
  both engines. Framer writes its page root that way with `background-color:
  #000`, so dark copy on a white page scored `1.3:1 on #000000` and gold copy
  on white passed against black (dadastudio.framer.website, 61 findings, and
  the walkthrough's "Hear from our client" miss).
- **White on white (row 22).** The full pass in `check_colors` stands down on a
  surface in exactly the text's own colour, as the SAFE_TAGS path already did.
  `ColorOpts` gains `same_color_surface_is_unread`; the frozen call vectors
  leave it false and replay as recorded. The static engine sets it always. The
  URL engine sets it only where it cannot confirm the surface: the hit-test
  stacks are not `Consistent`, and either the walk reached the page ground or
  the structural layer climb finds something other than that ancestor's own
  fill under the text. White copy on a white card with nothing between keeps
  its `1.0:1` (ynet.co.il's slot captions), and so does a title whose stacks
  confirm it sits on its white header.
- **The layer check answers partly visible boxes (row 22).** `layers_at_text`
  asks the grid points inside the viewport and decides where at least half of
  the run's grid lies inside it, instead of requiring the whole box inside.
- **Open shadow trees are captured and walked (row 38).** `15-snapshot.js`
  records every open shadow tree after the light DOM (light ids do not move),
  with `sh` (the host of a top-level node), `as` (`assignedSlot`), `ts` (the
  slot a host's own text is assigned to) and `shadow: true` on the snapshot.
  Shadow elements are nobody's child, so document walks and selectors never
  reach them. `Dom` gains `flat_parent`, `text_slot` and
  `shadow_trees_recorded`, defaulting to the light tree; the contrast walk, the
  opacity fold and the layer check's host containment follow the flat tree,
  and a host whose own text is slotted takes its ink and font from the slot. A
  recording made before this capture, where the text belongs to a custom
  element and the walk ended on the page ground, prints no verdict.
- **The page's one report of a colour pair goes to a readable copy (row 42).**
  An element cut by the page's left or right edge claims its pair
  provisionally (`SafeTagTextSeen::keep_first_keyed_claiming`, `PairClaim`); the
  first later copy wholly on screen reports instead, and the driver withdraws
  the cut copy's finding. A cut copy nobody replaces stands.
- **Icon ligatures and close letters are not read (row 43).** A one-word
  lowercase ligature set in an icon font (`arrow_forward` in Material Symbols)
  and a Latin `x` in a control whose class, id, `aria-label` or `title` names a
  close or dismiss control count as glyph-only text, in both engines and in
  the visual collector.
- **Walkthrough misses.** A box under 10px wide is scored where its parent
  carries text of its own (joongang.co.kr's `1/8` counter total, 7.86px wide);
  a numeral alone in a circle and a decorative bit field stay marks. The
  gradient dedupe key carries a translucent ink's alpha, so `text-blue-100/70`
  and `/80` on one box are two pairs; the walkthrough blamed the inherited
  wrapper colour for veeza.ai's caption, and the replay shows the key. The
  visual pass records `backdrop filter under an opaque fill` for a box whose
  own fill is at least 95% opaque, a reason no pass refuses, so shadcn's
  frosted header is measured instead of blocked.
- **The sampled pass reads the median.** `sampled_verdict` takes the median of
  the per-point ratios, as the pixel pass reads its glyph cores; where the
  median is more than 3 times the 10th percentile the points read two surfaces
  and the 10th percentile stands, as before.

Not changed: row 11. Exion's pricing labels have no fill under them in the
snapshot, and fabadda.com's 90% badge gradient sits over a sibling photo whose
pixels the capture does not carry; compositing over white is the worst case,
so its 2.8:1 is a bound, and the verdict holds.

Goldens re-recorded from the binary and read finding by finding:

- `detect-fixture-json-covered-text-contrast-html`,
  `detect-fixture-text-covered-text-contrast-html`: 15 to 13, the two
  `1.0:1 (need 4.5:1) — text #ffffff on #ffffff` initials, should-pass copy the
  static engine scored against the page white.
- `detect-fixture-json-text-occlusion-html`, `detect-fixture-text-text-occlusion-html`:
  2 to 1, `1.0:1 (need 3:1) — text #ffffff on #ffffff`, the pass hero caption.
- `detect-fixture-json-gradient-surface-contrast-html`,
  `detect-fixture-text-gradient-surface-contrast-html`: 15 to 14,
  `1.0:1 (need 4.5:1) — text #0f172a on #0f172a (gradient on a.underline-link)`,
  the collapsed underline marked "URL engine".
- `detect-fixture-json-surface-resolution-contrast-html`,
  `detect-fixture-text-surface-resolution-contrast-html`: new, 11 low-contrast
  and 2 cramped-padding. The flag column's readable cases: `#c8c8c8`,
  `#d49522`, `#fdfdfd`, `#9ca3af on #ffffff` (the shadow band it cannot see),
  `#c9a3c8`, `#a4a7ae`, `#ffffff on #76b835`, `#999999`. The pass cases the
  header marks: `#f0f0f0` (the edge photo title), `#e5e7eb` (the slotted
  button label) and `#ffffff on #ffaa01` (the lone step numeral). Neither
  translucent ink on the blue band reports here (the static engine stands
  down on translucent text over a gradient), and `cramped-padding` reads the
  black contents root as a box with flush children.
- `detect-dir-json-all-fixtures`, `detect-no-advisory-json`: exactly the 4
  removed above and the new fixture's 13; `detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures`, `detect-no-advisory-text` the same, 568 to
  577 anti-patterns. `detect-scope-both` and `detect-scope-layout-text` gain
  the two cramped-padding findings.

`crates/html/tests/link_text_contrast.rs` pinned the `<p>` printing `#ffffff on
#ffffff`; it now pins that no path prints it and a `#fdfdfd` paragraph still
reports. The generated browser asset was regenerated with `cargo xtask bundle`.

The URL behavior is pinned by `crates/browser/tests/surface_resolution.rs`:
every should-flag case reports and no should-pass case does. Scanned with the
base binary (5ce750e5), the same fixture reports eight should-pass cases (the
edge photo title, the cut marquee copy, the contents-root copy, the photo
heading, the shadow dark band, the slotted button label, the icon ligature
and the close `x`), misses the gold eyebrow, the on-screen marquee copy, the
counter total, the 0.8 caption and the frosted tagline, and scores the light
shadow band on `#ffffff`.

Corpus:

- **Ratchet over run 20 (cohort 2, 244 captures).** low-contrast 1,411 to
  1,304: 117 removed (pattern-absent 111, real-harmless 5, confirmed-harmful
  1), 10 added. The removals are dadastudio.framer.website's black root (61),
  white on white on nike.com (27), thecignagroup.com (8), exxonmobil.com (6),
  simplybudget.framer.ai (5) and v0-mono-six.vercel.app (3), the three pair
  moves on v0-optimus-delta.vercel.app, agora.co.il's close `x` (2),
  climatempo.com.br's ligature and d3shop.ae's cut marquee copy. The one
  violation, 108253, is that copy ('IR' of 'HAIR'): its pair now reports on
  'Body'. The additions are those moves (3 on v0-optimus-delta, 1 on d3shop),
  the gold eyebrow (2), veeza.ai's caption (2) and joongang.co.kr's counter (2).
- **Ratchet over run 19 (cohort 1, 342 captures).** low-contrast 3,685 to
  3,643: 42 removed (unjudged 38, confirmed-harmful 4), 0 added. All are white
  on white: zid.sa, nubank.com.br and aajtak.in (8 each), context.dev (6),
  ynet.co.il and te.eg (4 each), sapo.vn and shipthatcode.com (2 each). The 4
  violations are ynet.co.il slot captions (94298, 94305, 94313, 94318) whose
  walk ended on the page ground; their crops show dark copy on white, so the
  `#ffffff` ink the snapshot recorded was not what painted, and they inherit
  the label of their cluster's representatives. An earlier revision guarded
  every same-hex surface and removed 8 ynet.co.il findings there, including
  the title confirmed by its stacks; the narrowed guard keeps those 4.
- **Live, run 24** (thecignagroup.com, nike.com, exxonmobil.com,
  dadastudio.framer.website, 24 captures) against run 20 on the same pages:
  low-contrast 139 to 26. Replaying run 24's captures with the base engine
  isolates the engine: 129 scan-path findings base, 17 branch, differing in
  exactly dadastudio's black root (and the added eyebrow), thecignagroup's
  promo bands, white on white on nike.com and exxonmobil.com. The new snapshots
  carry shadow trees (thecignagroup.com's home page: 60 shadow top-level
  nodes, 97 slotted children, one slotted host text), and every promo band
  finding is gone because the walk reads the band's own fill. The sampled
  verdicts on exxonmobil.com print their median (`1.7:1 median 2.6:1` is now
  `2.6:1 median 2.6:1`), and one subline at `3.6:1 median 8.3:1` passes.

### Known limits at merge

1. **Invisible text on the page ground.** Text really painted in its own
   colour on the page ground, below the fold or where hit tests do not find
   it, prints no verdict. On run 19 that is 4 ynet.co.il findings whose crops
   show legible copy.
2. **Closed shadow roots.** `el.shadowRoot` is null for them, so their fills
   are unread, as before; the old-capture fail-safe does not apply to new
   captures.
3. **Live probe.** `10-probe.js` implements no flat tree, so the live overlay
   walks the light DOM as before; only snapshot-based scans read shadow trees.
4. **Harness element counts.** The corpus inventory counts `snap.els`, which
   now includes shadow-tree elements on new captures.
5. **Icon fonts by name.** Only families whose first name is a known icon font
   or contains the word `icons` or `icon` are read as icon fonts; a ligature
   face with another name is scored.
6. **Provisional claims below the fold.** Only horizontal cuts are
   provisional; a copy parked above the page is not.
7. **Row 11 numbers.** Panels the snapshot does not describe, and translucent
   fills over photos, keep the numbers base prints.
8. **API.** `Dom` gains `flat_parent`, `text_slot`, `shadow_trees_recorded`;
   `SnapNode` gains `shadow_host`, `assigned_slot`, `text_slot`; `Snapshot`
   gains `shadow_trees_recorded`; `FakeDom` gains `add_shadow_child`,
   `set_assigned_slot`, `set_text_slot`, `shadow_trees_unrecorded`;
   `ColorOpts` gains `same_color_surface_is_unread`; `SafeTagTextSeen` gains
   `keep_first_keyed_claiming` and `take_superseded`; `check_colors_deduped_claiming`,
   `PairClaim`, `sampled_verdict`, `occlusion_grid_size` and the icon-font
   predicates are new.
9. **The sampled median can hide a partly unreadable line.** Where up to about
   4 of 9 samples fall over a light patch, the median still reads the passing
   surface and the line prints no verdict; only a spread past 3 times the 10th
   percentile keeps the low end.

At merge into `corpus/integration`, after `corpus/fix-evidence-origin` and
`corpus/fix-painted-gate-coverage`, the source merged cleanly beside
painted-gate-coverage's edits to `element_checks.rs`, `page_checks.rs` and
`driver.rs`. The generated browser asset conflicted and was regenerated with
`cargo xtask bundle`. The JS and Rust snapshot lists still match, 117 style
and 16 pseudo properties, each once. The branch's re-recorded fixture goldens
(covered-text, gradient-surface, text-occlusion, scope-layout-text) replay as
recorded. The six conflicted sweeps were re-recorded from the integrated
binary and moved by exactly the branch's delta (13 added and 4 removed in the
JSON sweeps, 2 added in `detect-scope-both`), the text summaries reading 591
to 600 here against 577 there.

## Recorded 2026-09-14: one report per declaration, and dark pages read off the page (corpus/fix-page-level-forms)

Corpus run 20, `reports/observations-20.md` rows 5, 9, 44 and 45, and two
misses from `reports/walkthroughs-20.md` (marquee travel inside `calc()`, the
property-name boundary). Several rules read one declaration twice: off an
element's computed style, off its class attribute, and off the stylesheet
text, which lands on `body`. In the URL engine the element forms run first
and read every element, so a stylesheet form now stands only where no element
form speaks for the same declaration.

URL engine only (`crates/core/src/browser/driver.rs`, which no golden runs):

- **A class form defers to its computed twin (row 5).** On one element,
  `bg-clip-text + bg-gradient (Tailwind)` and `animate-bounce (Tailwind)` are
  dropped once the computed form of the same rule reported it.
- **A page form defers to the element forms (rows 5 and 45).**
  - gradient-text: both page forms are dropped when any element carries
    gradient-text.
  - bounce-easing: a page form is dropped when an element finding reports the
    same declaration: an animation name, or an overshooting `cubic-bezier`
    compared by its numbers (`.34` equals `0.34`).
  - dark-glow: a page form is dropped when an element finding reports the
    same shadow property and colour, and when its selector names elements (no
    pseudo-element), because the element form read their computed shadows,
    size, opacity and surface and measured no glow: a later
    `text-shadow: none` (demotv.lol `.tv-hud`), a box with no area. A
    pseudo-element selector, and a form with no rule to name (a keyframe
    step, an inline `style` attribute), stand.
- **layout-transition's page form needs a painted element (row 9).** It stands
  only where no element form exists on the page and the first declaration's
  rule matches an element painted at capture (the Box gate) that computes one
  of the properties it names; for a pseudo-element selector a painted host is
  enough. A declaration with no rule to name is not reported from the text.
- **One strip, one marquee report (row 5).** Page forms whose selectors name
  the same elements (a base rule and a builder's more specific copy), or
  elements under one parent (the `text--clone` track looping beside the
  original), merge into the first.
- **Dark page from the page (row 44).** The painted root is the first of
  `html` and `body` with an opaque fill; a fill under an image, or no fill,
  is unread. A dark root settles the scan's "dark page"; otherwise the
  stylesheet decides the candidates as before, and a form that claims a dark
  page (radial-halo, an offset dark-glow) is decided against the surfaces
  under the elements it names (its own opaque fill or the walk behind it; for
  a form with no rule, the elements whose computed background or shadow
  paints its colour). A dark surface makes it stand on any root (a halo in a
  dark band of a light page), surfaces all read as light drop it, with no
  element the root decides, and an unread surface (an orb inside a gradient
  button) keeps it.
- **A glow has to lift its surface (row 45).** `glow_is_perceptible` gains a
  floor, `GLOW_MIN_LIFT` of 20 channel units: half the ink of each chromatic
  layer times its largest channel difference from the resolved surface,
  summed over the layers. It runs only where the engine measured the element
  and the walk resolved a fill (never from the gradient average, which
  ignores stop alpha), so the static engine, the text engine and the frozen
  `checkGlow` vectors are unchanged. `GlowOpts` gains `surface`.

All engines:

- **A property name has to start its own token.**
  `starts_css_property_token` (foundation `css/scan.rs`) refuses a match
  after a hyphen or an identifier character, and allows a clean vendor
  prefix. It gates the shadow declarations of the glow scan
  (`--bprogress-box-shadow` on hrsd.gov.sa, cisco.com's hover token), the
  dark-background declarations and root scopes (`--color-background: #0a0a0a`),
  the halo declarations, the `transition` declarations, and the layout
  properties inside them (`border-width`, `line-height`, `scroll-margin`),
  in the core pattern pass and the text engine's matchers.
- **Marquee travel inside `calc()`.** `TRANSLATE_X_PCT_RE` reads the first
  percentage inside a `calc()` (`translateX(calc(-100% - 32px))`).

Goldens recorded from the binary and read by hand:

- `detect-fixture-json-page-level-forms-html`,
  `detect-fixture-text-page-level-forms-html`: new. The static engine keeps
  base behavior, thirteen findings: gradient-text four times (the computed
  and class forms on the heading, and both page forms), dark-glow three times
  (the amber CTA, the avatar at 15% alpha, and `Colored box-shadow glow
  (#6366f1) on dark page` decided from the player box's `#000000`),
  `cubic-bezier(0.34, 1.56, 0.64, 1)`, `transition: height` for the collapsed
  drawer, the three ticker rule blocks, and the halo `on dark page`. The
  token shadow and the two hyphenated transitions report nothing, and neither
  does the painted link's width transition, which the static pattern pass
  leaves to the element rules.
- `detect-fixture-json-page-level-forms-dark-html`,
  `detect-fixture-text-page-level-forms-dark-html`: new, two findings, the
  orb's `Zero-offset box-shadow glow (#22c55e)` and the hero's
  `radial-gradient halo (#8fd8f2 → transparent) on dark page`. The two
  tokens report nothing; base reported them instead (`#ec4899`, `#fdba74`).
- `detect-dir-json-all-fixtures` (648 to 663), `detect-no-advisory-json` (568
  to 581), and the text forms `detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures`, `detect-no-advisory-text` (568 to 581
  anti-patterns, 80 to 82 advisory notes): exactly the fifteen findings
  above, nothing removed or rewritten. No other golden moved: no existing
  fixture holds a hyphenated property token or a `calc()` marquee.

The URL behavior is pinned by
`each_declaration_reports_once_where_it_paints`
(`crates/browser/tests/page_level_forms.rs`): the light fixture reports five
findings (the heading once, the button once, the CTA once, the link's
`transition: width` on body, the ticker once) where base (5ce750e5) reports
twelve, and the dark fixture reports the orb and the hero halo.

Corpus ratchets (scan path), base and candidate findings:

| Rule | Run 20 | Removed / added | Run 19 | Removed / added |
|---|---|---|---|---|
| gradient-text | 148 to 88 | 60 / 0 | 172 to 124 | 48 / 0 |
| bounce-easing | 128 to 102 | 26 / 0 | 167 to 146 | 21 / 0 |
| dark-glow | 91 to 46 | 45 / 0 | 182 to 127 | 55 / 0 |
| layout-transition | 223 to 110 | 113 / 0 | 296 to 175 | 121 / 0 |
| marquee | 20 to 16 | 6 / 2 | 16 to 18 | 0 / 2 |
| radial-halo | 6 to 2 | 4 / 0 | 12 to 7 | 5 / 0 |

Run 20 counts 62 violations, every one a duplicate whose twin still reports:
56 gradient-text class and body forms on fabadda.com and ai-pact.com, each
with the computed form on the same element or page, and six dark-glow body
forms with element findings of the same colour (zoptron.framer.ai 103795,
104125; codecanary.org 106627, 106950, 107027, 107172). Run 19 has no
judged removals. The two marquee additions per run are the `calc()` loops
(d3shop.ae's shipping ticker, context.dev's logo strip).

The generated browser asset was regenerated with `cargo xtask bundle`.

### Known limits at merge

1. **First hit only.** The page glow and halo forms still report the first
   qualifying declaration. Where the gate drops it, a later declaration is
   not considered, as base never reported it either.
2. **A judged glow under the lift floor.** kraflio.com's pricing card, a 30px
   halo at 15% alpha (lift 10), was labeled visible by both judges in run 2
   (11788, 12398); its run 19 findings (97012, 97151, 97512, 97603) are
   removed. The crop shows no light past the card's edge, and the card's
   call-to-action glow is still reported.
3. **Inline transitions.** A painted element whose own `style` attribute
   transitions a layout property the motion check skips by tag (onlinetest.tw's
   AdSense span) no longer reports from the text form.
4. **Dead declarations elsewhere keep base.** gradient-text and bounce-easing
   page forms with no element twin still report a declaration nothing paints;
   the painted gate for page forms belongs to `corpus/fix-painted-gate-coverage`,
   which edits the same driver pass.
5. **An unread surface keeps a dark claim.** A halo inside a gradient box on a
   light page (framai.framer.website's orbs) stands.
6. **Static engine unchanged.** It keeps the duplicates, the stylesheet's dark
   decision and no lift floor; it gains the token boundary and `calc()`.
7. **API.** `GlowOpts.surface`; `glow_is_perceptible` takes the summed lift;
   new `PatternContext`, `check_html_patterns_with`, `first_layout_transition`,
   `scan_css_text_for_glow_with`, `scan_css_text_for_radial_halo_with` and
   `starts_css_property_token`.
8. **The lift floor assumes half alpha at the box edge.** It drops faint but
   visible glows, a 60px violet at 0.15, and a glow with positive spread whose
   real lift is about 35. No corpus finding is lost to it.

At merge into `corpus/integration`, after `corpus/fix-evidence-origin`,
`corpus/fix-painted-gate-coverage` and `corpus/fix-surface-resolution`,
`driver.rs`, `css_scan.rs`, `rules.rs`, `element_checks.rs` and `types.rs`
merged cleanly. painted-gate-coverage's `page_form_painted` test runs while the
style-text patterns are collected, and this branch's one-report-per-declaration
pass runs afterwards on what is left, so both are kept. That settles item 4 for
bounce-easing, whose page forms now need a painted match; gradient-text page
forms with no element twin still report as on base. The generated browser
asset conflicted and was regenerated with `cargo xtask bundle`. The five
conflicted sweeps were re-recorded from the integrated binary and moved by
exactly the branch's delta (15 findings added in the JSON sweep, 13 without
advisories, nothing removed), the text summaries reading 600 to 613
anti-patterns and 83 to 85 advisory notes here against 568 to 581 and 80 to 82
there.

9. **An inline declaration whose element is not painted.** The page form
   defers to an element form of the same declaration, and in the integration
   that element form has already passed the painted gate. visiby.net declares
   `cubic-bezier(.34,1.56,.64,1)` in the `style` attribute of an inbox row held
   at opacity 0 until reveal. painted-gate-coverage drops the row's element
   form (105794, 106275), so no element twin is left, and the body form (105825,
   106315) stands, since a form from a `style` attribute carries no selector
   for the painted gate to test. Base reports the declaration twice, this
   branch alone once from the row, the integration once from body. That is the
   whole run 20 bounce-easing gap: 39 removed against the branches' 41.

## Recorded 2026-09-14: structure read from computed boxes instead of class names (corpus/fix-card-heuristics)

Corpus run 20 (cohort 2), observations-20 rows 7, 25, 28, 30, 36, 50, 51 and 57,
and walkthroughs-20 miss 1 (the label collectors). Most of the changes are in
the URL engine's rule pass, which no golden runs. The goldens move because the
rule fixtures gained cases, and because a few shared checks are measured by
both engines.

- **nested-cards (URL engine).** A card is read from its computed box: it
  paints an edge on at least three sides (a border at least half a pixel wide
  that draws, or an outer shadow that reaches a pixel past the box on that
  side; a zero-offset ring reaches all four) or a fill that differs from the
  surface under it by more than 3 on a channel once composited, and it is
  rounded or casts a shadow. No class name is read, so a `border-t` section, a
  `border-b-[4px]` strip and a footer rule are not cards. An inner candidate is
  skipped when it is a one-line label box (a pill whose rounding meets at its
  ends, or a box whose content holds one line of its own text), a field box
  whose children are all form controls, an inline run (`<mark>`), a frame
  around a picture, a video, a canvas or an iframe covering 60% of it, or a
  band along three edges of the card it sits in (a card's own header). A
  surface that cannot be read (an ancestor paints an image or a gradient, or a
  dark scheme with nothing painted) counts any fill that is not transparent, as
  before.
- **nested-cards (file engine).** A class counts as a four-sided border only as
  the `border`, `border-N` or `border-[Npx]` utility, with any variant prefix.
  `border-t`, `border-b-[4px]`, `border-black` and `border-dashed` no longer
  make a card.
- **clipped-overflow-container (both engines).** The viewport and decoration
  words are read as whole words of a class list or an id, split on every
  character that is not a letter or a digit and on camelCase:
  `kitify-text-marquee__text` holds `marquee`, `#hotStuffScroller` holds
  `scroller`. A positioned child whose own id or classes name a track word is
  skipped wherever it sits under the container.
- **buried-raster.** Both engines skip vector art: an `<img>` whose `src` is an
  SVG, or a background whose every `url()` is one. The URL engine also skips
  icon-sized boxes (48px or less on both axes), blurred placeholders
  (`filter: blur()` of 4px or more), and a frame under a painted raster sibling
  or a parent or grandparent painting a `url()` over half its box.
- **gray-on-color (both engines).** The channel spread a background needs rises
  with the square root of how far its luminance sits under 0.01, so `#04002d`
  and `#1e002f` read as black; `#001c47`, `#002733` and `#21254a` still read as
  colour. A utility behind a state variant (`hover:`, `focus:`, `group-*:`,
  `peer-*:`, `aria-*:`, `data-*:` and similar) is not the resting fill.
- **undersized-ui-text (both engines).** Text inside `sub` or `sup` is skipped,
  so a footnote link in a marker no longer reports.
- **flat-type-hierarchy (both engines).** An `h1` set at two sizes equally
  often takes its larger size; any other tied role stays out of the ladder.
- **oversized-h1 (URL engine).** A heading that starts below the first viewport
  is not reported, and characters are counted from the text that paints (a
  word rotator's hidden words are left out).
- **first-viewport-column-overflow (URL engine).** A `nav`, a `tablist` or
  `navigation` role and a sticky column are not measured, what sits inside an
  absolute or fixed layer does not count toward a column's content, and a
  column with nothing painted in its flow is skipped.
- **gpt-thin-border-wide-shadow (both engines).** A halo's blur is its radius
  less any negative spread, so `0 18px 40px -26px` and `0 30px 60px -40px` are
  tight lifts. The URL engine also drops a halo that does not show over the
  surface under the element and a hairline that does not show against the fill
  it rims (a channel delta under 8).
- **em-dash-overuse (URL engine).** `innerText` runs between tabs and line
  breaks whose whole text is a dash (the cells of a pricing matrix) are not
  counted.
- **icon-tile-stack, kicker-above-heading, numbered-section-labels.** The tile
  tint needs an alpha of 0.05, not above 0.1 (`bg-primary/10`), and a zero-blur
  ring counts as the tile's border (both engines). A kicker may be up to 0.45 of
  the heading's size, capped at 16px and never under the old 14px (13px for a
  numbered label) (both engines). The URL engine's collectors climb up to four
  wrappers while the heading leads its wrapper; past the first climb (the
  second, for numbered labels, which already read one) the label has to sit
  within 96px above or before the heading. The kicker and label type is read off
  the only text-bearing child when the label has no text of its own, and an
  icon tile resolves a `display: contents` wrapper and a paint-free container
  of the same size, and accepts a masked box as its icon.

Goldens, each re-recorded from the binary and reviewed line by line:

- `detect-fixture-json-buried-raster-html`, `detect-fixture-text-buried-raster-html`:
  5 to 9 counted. Added: the flag texture at opacity 0.04, and the file scan's
  reports of three URL-only pass cases (a 24px icon, a blurred placeholder
  canvas, a crossfade frame). The two SVG pass cases report in neither engine.
- `detect-fixture-json-clipped-overflow-container-html`, `detect-fixture-text-clipped-overflow-container-html`:
  12 to 13 advisory, the BEM tooltip flag case. The marquee, camelCase and
  nested slide pass cases report nothing.
- `detect-fixture-json-gpt-thin-border-wide-shadow-html`, `detect-fixture-text-gpt-thin-border-wide-shadow-html`:
  15 to 18 advisory. The flag panels now carry a -12px spread and still report
  at 60px; the file scan reports the new dark-surface pass row, which it cannot
  measure. The negative-spread pass row reports nothing.
- `detect-fixture-json-icon-tile-stack-html`, `detect-fixture-text-icon-tile-stack-html`:
  7 to 9, the tinted tile and the ring tile. The Framer and `display: contents`
  flag cases need the climb, which the file scan does not do.
- `detect-fixture-json-kicker-above-heading-html`, `detect-fixture-text-kicker-above-heading-html`:
  13 to 14, the 15px kicker over a 44px heading.
- `detect-fixture-json-numbered-section-labels-html`, `detect-fixture-text-numbered-section-labels-html`:
  4 to 5 advisory, the 15px index over a 44px heading, and every snippet's count
  moves from `(4 on page)` to `(5 on page)`.
- New cases: `detect-fixture-{json,text}-nested-cards-html` (3: the two flag
  cases and the lip pass case, which a file scan reads as a shadowed, rounded
  card), `-gray-on-color-html` (4 flag cases), `-footnote-markers-html` (1),
  `-flat-type-hierarchy-h1-tie-html` (none), `-oversized-h1-rendered-html` (2,
  both URL-only pass cases), `-first-viewport-column-overflow-outline-html` and
  `-tabs-html` (none; the rule is URL-only), `-em-dash-pricing-matrix-html` and
  `-em-dash-prose-with-matrix-html` (1 advisory each; the file scan counts every
  dash).
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`:
  648 to 672 findings (568 to 585 counted, 80 to 87 advisory), exactly the
  per-fixture changes above. `detect-no-advisory-json`, `detect-no-advisory-text`:
  568 to 585. `detect-scope-type`: 160 to 165. `detect-scope-both`: 208 to 219.
  `detect-scope-layout-text`: 32 to 37 counted, 16 to 17 advisory.

`crates/browser/tests/card_heuristics.rs` pins the URL engine on every flag and
pass case above. On the corpus replays (run 20, cohort 2, and run 19, cohort 1),
nested-cards goes from 172 to 113 and 214 to 167, gray-on-color from 32 to 0 and
65 to 35, buried-raster from 49 to 4 and 212 to 178, and the collectors add 38 and
37 icon tiles, 30 and 28 kickers, and 25 numbered labels on run 20.

### Known limits at merge

1. **Rings don't count as card edges.** An inset ring (`inset 0 0 0 1px`) or a
   sub-pixel ring is not a card edge, so a ring panel inside a ring card is no
   longer reported (context.dev, landio).
2. **One-line boxes are skipped at any width.** One-line bordered rows inside a
   card are lost: clipto's transcript rows, context.dev's URL rows and
   chorusai's model chips.
3. **Square boxes no longer count as cards.** A bordered box with no radius and
   no shadow, inside a square bordered section, is not reported.
4. **A gpt-thin-border golden flag case was edited.** The `0 30px 60px -40px`
   panels went from flag to pass per observations-10 row 22; the judges split
   on evergrovelabs.
5. **New nested-cards findings are unjudged.** A filled, rounded box with no
   edge is a card now, so tinted stat tiles, FAQ items in a panel, chat bubbles
   and a tab bar inside a card report where the class test missed them: 41
   added on run 20 and 91 on run 19. The crops opened read as nested surfaces,
   but none has been judged; chat bubbles are real, harmless nesting (taste
   call P12).
6. **Duplicate labels.** opentrailpaper's numbered "01" labels are now reported
   by both kicker-above-heading and numbered-section-labels.
7. **Thresholds and residual gaps.** gray-on-color's luminance knee is 0.01, a
   taste threshold that separates a near-black `#1e002f` from a dark navy
   `#001c47`, where the judges only split on `#04002d`. The four-wrapper climb
   and the 96px gap are fixed constants. The file engine cannot read boxes, so
   it keeps class-driven cards and reports the fixture's lip pass case. Buried
   rasters of 48px or less are skipped with the icons.
8. **Track words beyond the list.** `jswiper`, `scroll-news`, `deck` and `slide`
   are not viewport words, so joongang.co.kr's swipers, news.cn's news ticker and
   hrsd.gov.sa's hero deck still report.
9. **Renamed snippets count as ratchet violations.** The 6 numbered-label and 2
   of the 4 em-dash "confirmed-harmful removals" on run 20 are the same elements
   re-reported with a new count; the other 2 em-dash removals are visiby.net's
   pricing page, the row 57 target, whose prose holds 4 em dashes once the 78
   matrix cells are left out.

At merge into `corpus/integration`, after `corpus/fix-evidence-origin`,
`corpus/fix-painted-gate-coverage`, `corpus/fix-surface-resolution` and
`corpus/fix-page-level-forms`, only the `crate::color` imports of
`element_checks.rs` conflicted: `color_to_hex` from the integration and
`composite_color_over` from this branch, both kept. `page_checks.rs`,
`text_collectors.rs`, `rules.rs` and `adapters.rs` merged cleanly. This branch
adds no snapshot property, and the JS and Rust lists still match (117 style
properties, 16 pseudo properties, each once). The generated browser asset
conflicted and was regenerated with `cargo xtask bundle`. The eight conflicted
sweeps were re-recorded from the integrated binary and moved by exactly this
branch's delta, nothing else rewritten: `detect-dir-json-all-fixtures` 698 to
722, `detect-no-advisory-json` 613 to 630, the text summaries 613 to 630
anti-patterns and 85 to 92 advisory notes, `detect-scope-type` 170 to 175,
`detect-scope-both` 220 to 231, `detect-scope-layout-text` 34 to 39 counted
and 16 to 17 advisory.

The corpus ratchets equal the previous integration plus this branch, rule for
rule, with no interaction gap: run 20 counts 91 violations (81 plus 10), run 19
still 4. The only rule both sides move is kicker-above-heading, where the
earlier merges' 6 removals and this branch's 30 additions stack (64 to 88). Each
of the 10 new run 20 violations has a twin that still reports. The six numbered
labels on v0-optimus-delta.vercel.app and v0-compute-11.vercel.app re-report on
the same capture and heading with a new page count. visiby.net's home page
reports 25 em dashes where it reported 48 (105822, 106312). visiby.net/pricing
(106370, 106655) is row 57's target and goes silent, its prose holding 4 em
dashes. Both judges labeled 106370 itself pattern-absent; its confirmed-harmful
label is inherited from the site cluster, whose representative 105822 on the
home page still reports.

## Recorded 2026-09-13: geometry rules measure the text, not the box (corpus/fix-text-geometry)

Corpus run 20 (cohort 2), observations-20 rows 8, 29, 31, 33, 40, 41 and 56,
and walkthroughs-20 misses 2 and 4a. Every change is in the URL engine's rule
pass except the heading exemption, which both engines share. A new module,
`impeccable_core::browser::text_geometry`, holds what the rules read: the
extent of a block's text (the union of the Range client rects of its own text
and of its inline phrasing children), the line pitch of an inline run, the
x axis of `overflow`, and whether a clipping ancestor cuts a line. A Dom that
cannot measure text answers `None`, and each rule then keeps the box it read
before.

- **line-length** reads the text where it can. Text that renders as one line
  is not reported. Lines are counted from the line-height: a text rect spans
  one content area (taken as 1.2em) plus one pitch per extra line, so two
  lines at `line-height: 2.4` count as two (`normal` is taken as 1.2em).
  Wrapped lines that fill at least 80% of the box keep the box estimate,
  since the half-em advance runs 10 to 20% over a narrow sans; lines short of
  it (a float beside them, centred) are estimated from the widest line, and
  when all the block's text sits in those lines, from at least the average
  count per line. Lines ended by a `<br>` are read from the widest line up to
  90% fill. No line holds more characters than the block. Full-width CJK
  glyphs count a whole em each, weighted by how many the text holds, on every
  path. A `p`, `li`, `dd` or `blockquote` whose words sit wholly in inline
  children (`<i>`, `<b>`, `<span>`, links) is measured on the block;
  unmeasurable, it stays silent as before.
- **body-text-viewport-edge** measures the glyphs: wrapped lines that fill the
  content box sit on its edges (padding excluded), other text on its own
  extent, and a list item's start side reaches its content edge, where an
  inside marker paints. Text a horizontal scroller cuts (a track at
  `overflow-x: auto` or `scroll` whose `scrollWidth` runs past its box, the
  root and body excluded) is not reported: scrolling brings it into view. A
  box that only hides its overflow proves no track, so text an
  `overflow-hidden` section or card, or an `overflow-x-hidden` wrapper, cuts
  at the screen edge reports as it did before. Prose in inline children is
  measured as above. The printed distances are the measured ones.
- **text-overflow** reads a scroll region from `overflow-x` (the shorthand's
  first value when no longhand was recorded), so a Tailwind
  `overflow-x-hidden` wrapper no longer exempts every line under it. An
  overflow of 16px or more is reported only when painted content reaches past
  the box by 15px or more: the element's text, a descendant's text, a replaced
  element, or a descendant that paints a fill, image or border. Empty or
  absolutely positioned descendants with no text add nothing, a descendant
  that clips keeps its content in its own box, and text an overflow-hidden
  box pushes wholly outside itself (`text-indent: -9999px`) paints nothing.
  An element whose own text cannot be measured keeps the overflow, and so
  does one that carries generated content no text rect covers, on itself or
  a descendant: a `::before` or `::after` with text, or in flow an image or
  an empty box given a width. One positioned out of flow with no text (an
  arrow icon parked past a link) adds nothing, as a child with none adds
  nothing.
- **tight-leading** judges an inline run on the taller of its own line-height
  and its containing block's (an 11px run at `line-height: 11px` in a 14px
  block prints `1.27x`, not `1.00x`). The heading exemption now covers any box
  under a heading, not only inline tags, unless it is or sits inside a reading
  block (`p`, `li`, `td`, `th`, `dd`, `blockquote`, `figcaption`), so the
  fixture's paragraph nested in an `h3` still reports.
- **cramped-padding** gates an inline box on the height of one line fragment
  (its box divided by the lines its text spans), so a two-line highlight span
  is two 21px lines, not one 43px box.
- **edge-flush-cards** skips the root and body, reads `overflow-x`, and needs a
  row: two card-shaped boxes side by side in the scroller.

Fixtures: `line-length.html` is new (browser only; the static engine reports
none of it). `body-text-viewport-edge.html`, `text-overflow.html`,
`tight-leading.html`, `cramped-padding.html` and `edge-flush-cards.html` gain
flag and pass cases, pinned against a real browser by
`crates/browser/tests/text_geometry.rs`. After review, `flag-cut-by-wrapper`
(a paragraph an `overflow-x: hidden` wrapper cuts at the screen edge),
`flag-pseudo-suffix` (a spill from `::after` text) and `flag-tall-leading`
(two lines at `line-height: 2.4`) were added, and `pass-clipped-slide` now
sits in a track at `overflow-x: auto`. The static engine reports none of the
revised cases, so no golden changed with the revision.

Goldens recorded from the binary and read finding by finding:

- New: `detect-fixture-json-line-length-html`,
  `detect-fixture-text-line-length-html` (no findings, exit 0).
- `detect-fixture-json-tight-leading-html`,
  `detect-fixture-text-tight-leading-html`: 6 to 7. The added finding is
  `tight-leading` `line-height 0.95x` on the new inline run inside a 22px
  block, which the browser engine passes and the static engine, with no
  layout to find the block that sets the pitch, still reports (documented in
  the fixture, beside the single-line case). The new div inside a heading is
  exempt in both engines.
- `detect-dir-json-all-fixtures`, `detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json`,
  `detect-no-advisory-text`, `detect-scope-type`, `detect-scope-both`: the
  same one finding (568 to 569 counted). Nothing removed.

### Known limits at merge

1. **Centred lines near 90% fill.** Wrapped lines that fill 80% of their box or
   more keep line-length's box estimate (90% for lines ended by a `<br>`, and
   for body-text-viewport-edge's content edges). That brings back ai-pact.com's
   centred lines (105480, 105968: about 84 characters on lines at 90% of their
   box, printed `~88`), which a crop called right to drop. Measurement cannot
   tell them from a real long line at 88% fill: veeza.ai 106327, about 86
   characters in a narrow sans, which base reported and should.
2. **Carousels that only hide overflow report partial slides.** A track at
   `overflow: hidden` or `clip` cannot be told from a layout bug, so a slide it
   partly cuts reports as base did (tempra.framer.website, zoptron.framer.ai
   and exxonmobil.com on run 20). A scrolling track, `overflow-x: auto` with
   slides to scroll to (nike.com's `ul.slider`), still exempts what it cuts,
   and a slide wholly outside its track is still dropped by the paint gate.
   The review's seven confirmed-harmful findings, text cut at the screen edge
   by v0-optimus-delta.vercel.app's `overflow-hidden` section (104197, 104200,
   104202, 104206) and simplybudget.framer.ai's card (104222, 104431, 104577),
   report again.
3. **Text measured past a clipping card.** The rules measure the text wherever
   it runs, so a nowrap line a card cuts inside the viewport can add a finding
   the box never gave. Recall grows the same way: prose in inline children and
   pages under `overflow-x-hidden` wrappers now report. Run 20 adds centene.com
   and mckesson.com paragraphs and list items, swipeloan.in paragraphs with
   10px gutters, and text-overflow on v0-optimus-delta.vercel.app's stats; run
   19 adds 18 bt.cn footer lines past a 169px column. One of the additions is
   v0-compute-11.vercel.app's ASCII texture at 2% ink, clipped on purpose.
4. **Approximations.** text-overflow keeps a pseudo-element spill by parsing
   the `content` value (text, `url()`, empty, `none`) with the pseudo's
   `display`, `position` and `width`, not by measuring the generated box. The
   80% fill threshold is a band, not a measurement of the face's advance, and
   a line's content area and `line-height: normal` are both taken as 1.2em
   when counting lines. The static engine has no layout and keeps base's
   estimates: line-length and the geometry gates stay browser only, and it
   still reports the inline run and the single-line label in
   `tight-leading.html`.
5. **Heading exemption.** Content a heading may not hold still counts as
   heading text: a `div` of copy nested in an `h2` stays exempt from
   tight-leading. Only a reading block (`p`, `li`, `td`, `th`, `dd`,
   `blockquote`, `figcaption`), or a box inside one, reports under a heading.
6. **Harmful-cluster removals on cohort 2.** Distances and counts print what
   is measured, so a finding that stays can print a new number, and the
   ratchet counts it as one removal and one addition.
   - **line-length: 76.** The cluster key collapses a site's paragraphs into
     one cluster, so every member of a cluster with one harmful representative
     counts as confirmed harmful: the 76 come from 2 cluster labels spread over
     15 sites (glassbox.codecanary.org 16, progressive.com 15, ai-pact.com 8,
     demotv.lol 7, joongang.co.kr 8 and more). By the review, 75 are really not
     failures: 63 render one line, 6 are lines well short of their box (floats,
     `<br>`) and 5 are CJK or Hangul.
   - **body-text-viewport-edge: 13**, in 3 clusters (so-net.ne.jp 7,
     simplybudget.framer.ai 3, centene.com 2, v0-optimus-delta.vercel.app 1).
     Each still reports on the same element with a new measurement:
     v0-optimus-delta.vercel.app `right -61px` to `right -19px`,
     simplybudget.framer.ai `right -314px` to `right -216px`, so-net.ne.jp
     `right -306px` to `right -297px`, centene.com `right -95px` to
     `right -23px`.

At merge into `corpus/integration`, after `corpus/fix-evidence-origin`,
`corpus/fix-painted-gate-coverage`, `corpus/fix-surface-resolution`,
`corpus/fix-page-level-forms` and `corpus/fix-card-heuristics`, all source
merged without conflict: `element_checks.rs`, `page_checks.rs`, `quality.rs`,
`text_rules.rs`, `html/src/quality.rs` and `evidence_findings.rs` auto-merged,
and `driver.rs`, `painted.rs` and `text_collectors.rs` were not touched by this
branch. This branch adds no snapshot property; it reads `overflowX`,
`overflow` and the pseudo `content`, `display`, `position` and `width`, all
already recorded, and the JS and Rust lists still match (117 style
properties, 16 pseudo properties, each once). The generated browser asset
conflicted and was regenerated with `cargo xtask bundle`. The seven conflicted
sweeps were re-recorded from the integrated binary and moved by exactly this
branch's delta, the one `tight-leading` `line-height 0.95x` finding and nothing
else: `detect-dir-json-all-fixtures` 722 to 723, `detect-no-advisory-json` 630
to 631, the text and quiet summaries 630 to 631 anti-patterns,
`detect-scope-type` 175 to 176, `detect-scope-both` 231 to 232.

The horizontal-scroller tests agree. The painted predicate lets a scroller
bring content into its box when `overflow-x` is `auto` or `scroll` and its
`scrollWidth` runs past `clientWidth` by more than 1px, and
`scrolling_ancestor_cuts` counts exactly that box as a track. They differ only
where the capture recorded no scroll metric (the painted predicate answers
yes, the text rules no, so such text keeps reporting) and in how a legacy
shorthand is read (the painted predicate reads `overflow` only when both
longhands are empty, `overflow_x` reads the longhand and then the shorthand's
first value). The ratchets below show no interaction from either on the two
cohorts.

The corpus ratchets equal the previous integration plus this branch, rule for
rule, with no interaction gap, and the removed and added sets are the branch's
own: run 20 counts 180 violations (91 plus 89), run 19 still 4. The rules this
branch moves (line-length, body-text-viewport-edge, text-overflow,
tight-leading, cramped-padding, edge-flush-cards) are moved by no earlier
merge. Every new run 20 violation is one of the branch ratchet's 89 (76
line-length and 13 body-text-viewport-edge above).

## Recorded 2026-09-14: what is on screen (corpus/fix-on-screen)

Corpus run 25 (both cohorts), `reports/observations-25.md` issues 4, 13, 14, 20
and 24, section 5's "What is on screen" branch plus the line-length regression.
Every change is in the URL engine's rule pass; the goldens move only because
`on-screen.html` joins the fixture directory.

- **A visible-share floor for text measurements (issue 4).** The Text gate asks
  the painted predicate with a floor: a text measurement with less than a
  quarter of its width (`TEXT_MIN_VISIBLE_SHARE`) inside a clip that parks
  copies (`clip_outcome`, `parks_copies`) or before the start of the page
  (`outside_document`) is not painted for the text rules. A clip parks copies
  when it is narrower than the page (the smaller of `innerWidth` and the root's
  `clientWidth`), when it scrolls on x with content to scroll to, or when it
  holds a transformed track (`moves_a_track`). A box at least as wide as the
  viewport that only hides overflow is the page shell: text it cuts is a
  layout bug a visitor sees, so that cut leaves the share as it was and the
  gate decides as base did. The document floor likewise applies only on the
  scroll origin side (left, or right under `direction: rtl`); text past the
  page's far edge runs past a page that hides its overflow and keeps
  reporting. With no measured viewport the width test proves nothing. The
  share is taken of the text where it can be measured (the phrasing extent),
  else of the box; passing a horizontal
  scroller replaces both with the scroller's box, since scrolling brings
  anything in its range into it. Only the x axis is floored: a line-clamped
  standfirst or a collapsed "read more" box shows its first lines while the
  text rects of the lines it hides run past its bottom. A box that truncates
  its line with `text-overflow: ellipsis` shows the start of it and is not
  floored. The Box, Raster and Toggle gates and the base predicate keep the
  1px overlap test.
- **The pair dedupe claims for good only a copy shown across (issue 4).** The
  claim's `on_screen` also asks `text_shown_across`: no clipping ancestor and
  no document edge cuts the text on the x axis, within a pixel. A copy part
  way past a carousel clip claims the colour pair provisionally, as a copy cut
  by the page edge already did, and the first copy wholly in view reports
  instead. A copy under the floor never claims, because the claim's keep
  callback asks the Text gate.
- **Zero boxes (issue 14): unchanged from base.** A 0x0 box keeps `no_area`'s
  overflow test on the Text gate: it is not painted when it clips an axis or
  its scroll extent there is at most 1px, and a 0x0 anchor whose nowrap label
  overflows visibly (a map pin, a chart label) still reports. The branch first
  dropped every 0x0 box; review found that silenced real visible text, and
  keeping it only where the scroll extent is at most 1px on both axes is what
  `no_area` already does, so the extra test was removed.
- **body-text-viewport-edge needs text in the viewport (issue 14).** Text whose
  measured span lies wholly past either side of the viewport meets no edge a
  reader sees: a desktop column laid out past a phone viewport
  (people.com.cn at x 515 on 390px), a list parked 800px right (news.cn).
- **heading-rhythm behind the Text gate (issue 20).** A heading not painted for
  the text rules is neither reported nor counted toward the two-heading
  minimum (joongang.co.kr's tab slide parked past its track, whose twin met
  the minimum).
- **Transformed tracks (issue 24).** `scrolling_ancestor_cuts` also counts a box
  that hides or clips x overflow and holds, between it and the text, an
  element with a `transform` or `translate` (the identity matrix included)
  that lays out a row of at least two boxes side by side and whose
  `scrollWidth` runs past the clip's `clientWidth`: Framer tickers, Swiper and
  slick tracks. A section cutting a paragraph with no such row still reports.
- **line-length reads the font of the text runs (issue 13).** The characters a
  line holds are estimated at the font size of the runs that set the text
  (the block's own text and its inline phrasing children, weighted by
  characters), and a monospace face (`is_monospace_family`: a generic
  `monospace` or `ui-monospace`, a family with a `mono` word, or a known code
  face, read from the first family) advances 0.6em a glyph
  (`MONOSPACE_ADVANCE_EM`). Full-width CJK glyphs stay an em. With every run
  at the block's size in a proportional face the numbers are unchanged.

Fixtures and tests:

- `on-screen.html` is new: dates on a carousel track (a 6px sliver, one wholly
  in view, one mostly in view), dates past the page's left edge (4 of 70px,
  40 of 70px), one colour pair on a word part way past its clip and on a word
  wholly in view, a word with a sliver in its clip, a label its box truncates
  with an ellipsis, a 10px notice collapsed to 0x0 that hides its overflow, a
  map pin's nowrap 10px label on a 0x0 anchor (`#flag-zero-anchor`), a
  non-wrapping row's second column a 100vw page shell cuts with 60 of 280px in
  view (`#flag-shell-cut`), a 6,400px desktop column in the same shell
  (`#flag-shell-desktop`), a word 20px into a 100vw clip around a transformed
  track (`#pass-shell-track-sliver`), and three crowded headings, one on a
  slide parked past its track.
  `crates/browser/tests/on_screen.rs` pins the URL engine: low-contrast on the
  seven flag cases only, tiny-text on `#flag-zero-anchor` and not on
  `#pass-zero-box`, and the two crowded headings. Scanned with the base binary
  (05cbd66e) it reports all pass cases (`#pass-parked-date`,
  `#pass-edge-sliver`, `#pass-cut-copy` instead of `#flag-whole-copy`,
  `#pass-sliver-copy`, `#pass-shell-track-sliver`) and `Pass Parked Slide`
  with `(3 headings on page)`; the branch reports only the flag cases, with
  `(2 headings on page)`. dbeaa60d, the branch before review, dropped the
  three shell and pin flag cases.
- `body-text-viewport-edge.html` gains `pass-past-viewport`,
  `flag-runs-past`, `pass-ticker-first`, `pass-ticker-second`,
  `flag-transformed-wrapper` and `flag-shell-sliver` (a non-wrapping row's
  720px second column with 140px in view inside a 100vw wrapper that hides
  overflow); base reports the three pass cases (`right -2199px`,
  `left -300px`, `right -260px`), and dbeaa60d dropped `flag-shell-sliver`.
- `line-length.html` gains `flag-large-span` (`~100`, base `~150`),
  `flag-mono` (`~119`, base `~143`), `pass-large-span` (base `~102`) and
  `pass-mono` (base `~97`). `crates/browser/tests/text_geometry.rs` pins both.
- `on-screen.html` joins `REPLAY_FIXTURES` in `crates/browser/tests/evidence.rs`.
- Unit tests: the floor, the ellipsis exemption, the unfloored y axis, a
  scroller cell, the document floor on the origin side in both directions and
  not at the far edge, the page shell (a row's second column and a desktop
  column at 390px, a classic scrollbar, and the narrower clip, scroller and
  transformed track that still floor, plus an unmeasured viewport), the 0x0
  anchor whose label overflows and the 0x0 box with nothing past its edges,
  and `text_shown_across` (`painted.rs`); the run font and the transformed track
  (`text_geometry.rs`); the monospace families (`text_rules.rs`); the gated
  heading count (`page_checks.rs`).

Goldens, recorded from the binary and read by hand:

- `detect-fixture-json-on-screen-html`, `detect-fixture-text-on-screen-html`:
  new, 15 findings. The static engine has no layout: `low-contrast` 11 (all
  five dates, the colour pair once for its first copy `#pass-cut-copy`, the
  sliver word, the truncated label, the two shell cases and the track
  sliver), `tiny-text` for the 0x0 notice and the pin, and a
  `clipped-overflow-container` advisory for each word clip. No heading-rhythm.
- `detect-dir-json-all-fixtures` 723 to 738, `detect-no-advisory-json` 631 to
  644, `detect-scope-type` 176 to 178, `detect-scope-both` 232 to 236, and in
  the text forms (`detect-dir-text-all-fixtures`,
  `detect-dir-quiet-all-fixtures`, `detect-no-advisory-text`,
  `detect-scope-layout-text`) only the new fixture's block and the summary
  counts (631 to 644 anti-patterns, 92 to 94 advisory notes, 17 to 19 in the
  layout scope). Every added finding belongs to the new fixture; nothing was
  removed or rewritten. The static goldens of `body-text-viewport-edge.html`
  and `line-length.html` stay empty.

The generated browser asset was regenerated with `cargo xtask bundle`.

Corpus. Exact branch deltas from replaying every capture with the base and the
branch engine; a rename is the same element reported with a new number.

- **Run 25, both cohorts** (585 captures; `on-screen-25`, violations 56):
  - body-text-viewport-edge 482 to 453: 29 removed (pattern-absent 24,
    confirmed-harmful 3, unjudged 2): people.com.cn 18 and news.cn 3 wholly
    past the viewport, zoptron.framer.ai 4 and tempra.framer.website 2 ticker
    items, exxonmobil.com and aajtak.in 1 each.
  - heading-rhythm 4 to 2: joongang.co.kr's pair (pattern-absent 2).
  - line-length 1,142 to 1,056: 86 removed and 81 renamed (the ratchet counts
    167 removed, 81 added; confirmed-harmful 30, pattern-absent 78,
    real-harmless 36, unjudged 23). The removals are avikmukherjee.com's
    JetBrains Mono paragraphs (83), cvs.com's subheadline, bitroad.ai and
    sapo.vn's next Swiper slide past the viewport. The renames are pages whose
    text sits in runs at another size or in code: vibe-genomics.replit.app 46,
    cvs.com 8, ladepeche.fr 7 (`~95` to `~96`), bitroad.ai 5, centene.com 3
    (`~123` to `~96`), useautumn.com 3.
  - low-contrast 4,896 to 4,882: 15 removed, 1 added (confirmed-harmful 2,
    real-harmless 3, unjudged 10): overdrive.health's rotating word parked left
    of its clip and two tab buttons with 4 and 9% in view, vibe-genomics code
    chips cut by a table scroller, ynet.co.il's times and clones, zigzag.kr's
    rating 3px in view, fabadda.com's chip, useautumn.com, hungrygpu.com,
    context.dev. The addition is ynet.co.il's `span.authorField` pair moving to
    a copy wholly in view.
  - tight-leading 2 (ynet.co.il's slick clone, samsung.com's slide), tiny-text 2
    (att.com cells past a clip), undersized-ui-text 35 (confirmed-harmful 21,
    real-harmless 13, unjudged 1): otto.de's `+N` colour counts at the edge of
    the swatch scroller (9), visiby.net's chart labels cut to `T` (10),
    yna.co.kr's icon labels (13), zigzag.kr's ratings (2), adant.ai (1).
  - The 56 violations: 29 of the 30 line-length ones are renames that still
    report (vibe-genomics.replit.app 26, centene.com 3), and sapo.vn's slide is
    a copy 84 of 926px in view. The 21 undersized-ui-text ones are otto.de,
    visiby.net and zigzag.kr copies under a quarter in view, carrying their
    clusters' labels. body-text-viewport-edge 123513 to 123515 are news.cn
    items 800px past the viewport. low-contrast 110070 and 110493 are the
    zigzag.kr rating.
- **Run 20, cohort 2** (244 captures; `on-screen-20` against `integration-20`):
  violations 180 to 184. body-text-viewport-edge 10 removed (zoptron.framer.ai
  4, news.cn 3, tempra.framer.website 2, exxonmobil.com 1), line-length 9
  renamed (centene.com, fps-tester.com, codecanary.org, copperhead.sh),
  low-contrast 2 removed and 1 added (fabadda.com's chip past its carousel;
  climatempo.com.br's Taboola `span.branding` pair moving to another copy),
  undersized-ui-text 10 removed (visiby.net). The 4 new violations are the two
  codecanary.org renames (106977, 107004), 103501 and 108369.
- **Run 19, cohort 1** (342 captures; `on-screen-19` against `integration-19`):
  violations 4 to 9. body-text-viewport-edge 19 removed (people.com.cn 18,
  aajtak.in 1), line-length 86 removed and 67 renamed, low-contrast 12 removed,
  tight-leading 1, tiny-text 2, undersized-ui-text 25 removed. The 5 new
  violations are zigzag.kr's rating (93595, 93596, 93998, 93999) and
  overdrive.health's parked word (100733).
- Crops opened: 111833, 110071, 121181, 121574, 112118, 117134, 110549,
  110600, 109708, 123513, 123368, 114418, 116101, 116157, 119192, 119884,
  116470, 115801, 103501, 116724. The swatch count, the rating, the chart label
  and the two sliver tiles show nothing readable at the clip's edge; the ticker
  items are cut at the window edge by their moving track; the mono and
  span-sized paragraphs hold 73 and 67 characters a line; the people.com.cn,
  news.cn and joongang.co.kr crops are element shots scrolled into view of
  copies the phone viewport never shows. 116724's crop, cut after the scan,
  shows the word that rotated in; in the snapshot the removed copy is
  "Insurance Discovery", its text wholly left of the clip. 115801's code chip
  shows "`LD_LIBRA" at rest, under a quarter of its text.

Not changed, and why:

- **co-trip.jp 109346.** Replayed, the finding is the copy at x 470, wholly in
  view; the rect at x -66 was measured after the scan, once Swiper autoplay
  had moved the track. The snapshot's partial copy sits at x -38 with 46% in
  view and still reports, and these dates are divs scored by the full pass,
  not the pair dedupe.
- **thairath.co.th 112152 and 112153.** In the snapshot the consent paragraph
  and its link have boxes (390x72); the 0x0 rect was measured after the scan.
- **climatempo.com.br 123680.** The snapshot places the Taboola button inside
  every clip of its card.

### Revised at review: fail safe where the engine cannot tell

The review approved dbeaa60d with two open issues. Where the engine cannot
determine a fact, the gate now fails safe to base behaviour and keeps
reporting.

- **Zero boxes.** dbeaa60d dropped every 0x0 box on the Text gate. The review
  probe's `#pin`, a 0x0 anchor with a visible nowrap 10px label, lost its
  tiny-text at 390 and 1280. The Text gate is back on `no_area` (see above);
  no corpus finding depended on the extra test.
- **The floor and the page shell.** dbeaa60d floored any clip and both edges
  of the document, which silenced text that starts in view and is cut by a
  wrapper that hides overflow. On the review's `overflow-bug.html` at 390x844,
  `#plain-right` (50 of 280px in view) lost `body-text-viewport-edge
  right -250px`, and `#desktop` (a 1,700px column) lost low-contrast,
  line-length and body-text-viewport-edge. The floor now applies only to
  clips that park copies and to the document's origin side (see above).
- **Probes, base 05cbd66e / dbeaa60d / now.** `#pin` tiny-text: reported /
  none / reported, at 390 and 1280. `#plain-right` body-text-viewport-edge:
  reported / none / reported. `#desktop` low-contrast, line-length `~215` and
  body-text-viewport-edge `right -1330px`: reported / none / reported.
  `#thirty` reports on all three. `#mq-one` (a single-span marquee in a
  full-width strip at 390) comes back as base, because its strip is the page
  shell. `#row-right`, `#sc20` and `#cl20` stay as dbeaa60d (limits 3 and 4
  below). The rest of the review's probe pages (`partial`, `pair-floor`,
  `prose`, and `shares` at 1280) match dbeaa60d exactly.
- **Corpus.** `on-screen-25`, `on-screen-20` and `on-screen-19`, re-run on the
  revised engine: violations 56, 184 and 9, as dbeaa60d, with every rule's
  candidate, removed and added counts and its confirmed-harmful ids identical
  on all three runs (only the ratchet's random 12-item samples vary, on rules
  this change cannot reach as much as on those it can). The revision only lets
  more text through the gate, so a finding coming back would raise a
  candidate count and a pair moving would raise both removed and added: no
  removal came back. Every floor removal in the corpus is cut by a clip
  narrower than the page, a scroller, a transformed track, or the document's
  start: otto.de's `+N` counts in the swatch scroller (111833), visiby.net's
  chart labels (121574), zigzag.kr's rating (110070, 110071), sapo.vn's next
  Swiper slide (112118) and overdrive.health's parked word (100733) are still
  removed. No track condition on duplicate text was needed.
- **Goldens.** Re-recorded from the binary: the nine above whose counts moved
  (the two `on-screen-html` goldens, the three dir sweeps, `detect-scope-type`,
  `detect-scope-both` and the two no-advisory forms). Each gains only the
  fixture's four new static findings (three low-contrast pairs and the pin's
  tiny-text) and the summary count; nothing else moved.
- The generated browser asset was regenerated again with `cargo xtask bundle`.

### Known limits at merge

1. **The floor is a width share.** A copy with 25 to 99% of its text in view
   still reports (co-trip.jp's date at 46%), and a long code chip or table cell
   with a readable start under a quarter of its text at rest no longer does
   when a clip that parks copies cuts it.
2. **Only ellipsis clips are exempt.** A box narrower than the page that hides
   a nowrap line without `text-overflow: ellipsis` shows its start and is
   floored like a track.
3. **Scroller cells under a quarter in view** are floored even though
   scrolling brings them in (the review's `#sc20` and `#cl20`, vibe-genomics'
   code chip 115801). Base already refused cells wholly past a scroller.
4. **`moves_a_track` counts identity transforms.** Any transformed element
   holding a row of two side-by-side boxes wider than the clip counts as a
   track, the identity matrix included (Tailwind's `transform` class, reveal
   libraries), so a non-wrapping two-column row carrying such a transform is
   floored and exempt from body-text-viewport-edge (the review's `#row-right`);
   no confirmed-harmful finding was lost to it.
5. **A single-span marquee at phone width** in a strip narrower than the page
   is floored once under a quarter of it shows, which silences its
   low-contrast. In a full-width strip it reports as base, and a track of two
   copies still reports.
6. **The page shell is read from width alone.** A shell narrower than the
   viewport (a centred max-width wrapper at desktop widths) parks copies like
   a carousel, and a full-width carousel whose slides are positioned rather
   than a transformed row reports its parked copies as base did.
7. **Vertical cuts are not floored**, and neither is a fixed layer's content.
8. **The run font is a character-weighted average**, and monospace is read
   from the first family only, so a missing mono web font whose fallback is
   proportional still counts 0.6em, which fails toward fewer findings.
9. **Text past the viewport** reports nothing on body-text-viewport-edge; the
   overflow itself stays unreported (observations-25 issue 8, P20).
10. **Pairs move.** A colour pair first worn by a copy part way past its clip
    now reports on a later copy wholly in view, which the ratchet counts as a
    removal and an addition.
11. **API.** `TEXT_MIN_VISIBLE_SHARE`, `text_shown_across`,
    `phrasing_text_font`, `average_glyph_advance_em_at`,
    `is_monospace_family`, `MONOSPACE_ADVANCE_EM` and `PROPORTIONAL_ADVANCE_EM`
    are new; `scrolls_x` and `moves_a_track` are now `pub(crate)`.
## Recorded 2026-09-11: comp regions no longer include neighbouring pixels

The `comp-diff-no-spec` golden now measures the automatic bands at their actual
bounds rather than enlarging bands under 48px. Reviewed changes are confined to
regional scores and ink boxes: the second band's overall is 0.6755 (was 0.6951),
the fourth is 1.0 (was 0.9468), and narrow-band ink boxes use the corrected crop
coordinates. Whole-frame scores, verdicts, region definitions, exit status, and
stderr are unchanged. The golden was updated to enforce these exact results;
this is not an open-ended accepted delta. Frozen function call vectors remain
unchanged. The Rust narrow-region regression independently checks that changing
only neighbouring pixels leaves the measured crop identical.

## Recorded 2026-09-17: reference-bound typography reuse

- `comp-spec-regions`: the written spec adds `compSha256`, a SHA-256 of decoded dimensions and pixels. This binds retained typography to the exact reference when regions are remeasured. Structured comparison verified that only this field changed; stdout, stderr, exit status, regions, palettes and bounds are identical. The failing/passing regression separately verifies preservation and invalidation.

## Recorded 2026-09-17: exact reference bounds and non-destructive plate candidates

Reviewed the three CLI differences before updating their goldens:
`comp-spec-grid` appends coordinate guidance, `comp-spec-usage` explains grid,
normalized box and exact pixelBox units, and `build-phase-usage` advertises the
read-only candidate check. Only those stdout strings changed. Existing image
files, measurements, exit codes and frozen function vectors were not replaced.
The plate gate applies reference UI exclusions symmetrically after alignment;
regressions separately verify hidden-pixel invariance, visible missing-art
rejection, raw comp-copy rejection and unchanged candidate-check state.

## Recorded 2026-09-18: draft region authoring and source binding

`comp-spec-usage` now describes --auto as a draft writer. In
`comp-spec-regions`, the only file-content addition is regionsSource with the
input path and fixture-derived SHA-256; all existing fields and measurements
were compared unchanged. No frozen function vectors changed. New Rust
regressions verify automatic drafts do not overwrite specs or existing drafts,
cannot be submitted unchanged, and a rejected/missing region-source revision
cannot advance the build using the last successful measurements.

## Recorded 2026-09-18: read-only map inspection

`comp-spec-usage` adds one help line for --inspect-map, --out-dir and --json.
Only that stdout line was edited; existing measurements and frozen function
vectors remain unchanged. Rust regressions cover consolidated invalid-input
findings, fully masked references, container and child masks, preservation of
review group members, reference PNG provenance, HTML escaping, and refusal to
overwrite an existing report. Inspection does not change specs or build state.

## Recorded 2026-09-24: build session identity and nested italic headings

`build-phase-start-status`: the written state adds `"sessionId": "oracle-build"`
after `finish`, from the new `--session-id` start option. No other state field,
stdout, stderr or exit status changed. `build-phase-usage` advertises
`[--artifact <entry file>] [--session-id <id>]` on start and the new
`completion [--session-id <id>]` verb; only those usage strings changed.

The italic-serif-display correction adds exactly one finding per golden that
scans `tests/fixtures/antipatterns/italic-serif-display.html`: `italic serif h1
(fraunces) at 72px "Inline Em Inside Roman"`, a roman h1 whose visible text is
an `<em>` set in the same serif. The fixture moved this case from should-pass to
should-flag. `detect-fixture-{json,text}-italic-serif-display-html` go from 7 to
8 findings, and the aggregate corpus goldens (`detect-dir-*-all-fixtures`,
`detect-no-advisory-*`, `detect-scope-both`, `detect-scope-type`) go from 419 to
420. Every other finding, count, snippet and exit status is unchanged. Hidden
heading descendants and sans-serif or small italics stay exempt; Rust
regressions in `crates/html/tests/italic_heading.rs` pin both sides.

## Recorded 2026-09-25: plan and asset review (component review v3)

Three new cases, recorded from the binary and reviewed by hand; no existing
golden changed. `component-review-usage` is the usage refusal, which now leads
with `plan [--out .impeccable/review/components.json]`. `component-review-plan-missing-plates`
runs `plan` on the comp-basic spec before its one plate exists: exit 1, the
missing plate listed, nothing written. `component-review-plan` adds the plate
and pins the written v3 packet: `art` as an asset previewed by its plate, `top`
(non-container chrome) as a plan item with a `comp-crop` preview, `body` in
`codeRegions`, and `specSha256` of the fixture spec. Contract:
`docs/PLAN-REVIEW.md`. The build-phase plates next-step text gained one
sentence naming the review; no golden prints it.

## Recorded 2026-09-25: surface reading on code regions

`comp-spec-regions`: the written spec adds `"surface": {"flat": false, "rules": false}` to the two code regions (`top`, `body`). The raster region, every other field, measurement, stdout, stderr and exit status are unchanged, and no region in the fixture reads painted, so no `flags` entry or `FLAG` line appears. No frozen function vectors changed. Rust regressions cover the readings: a painted patch reads the same in tight and generous boxes, containers flag only on unmapped painted material, and grounds and rules separate from marks.

## Recorded 2026-09-29: visualize.md before decision comps

Agents wrote decision comp prompts before opening `reference/visualize.md`, which holds every comp-prompt rule, while they followed the engine's printed NEXT lines reliably. `serve-question` now prints a `NEXT read <skill>/reference/visualize.md now, before writing any decision comp prompt; ...` line when a round's comps are due, and `--wait` names landed decision comps that have no prompt sidecar.

- `question-wait-flip`: after the unchanged `BUILD PATH FLIPPED` line, one added `NEXT read <REPO>/skill/reference/visualize.md now, ...` line (a flip to comp is the moment a code-led round's comps start). Exit status and files are unchanged.

New cases, recorded from the binary and reviewed by hand: `question-wait-comp-sidecar-missing` (WAITING plus `COMP SIDECAR MISSING` naming only the landed comp without a sidecar), `question-wait-answer-comp-sidecar-missing` (the same line after the ANSWER block), `question-update-comps-next` (the NEXT line after `next round delivered`), `question-update-code-led-no-next` (a code-led round prints no NEXT line). `--start` is not in the corpus because it binds a port; Rust tests in `crates/context/src/serve_question.rs` cover its trigger.

Follow-up on the same branch: the NEXT line now also requires a declared comp that is not on disk yet (a comp-round page serves comps that already exist, so "before writing any decision comp prompt" was stale there), and a comp-round pick no longer prints the decision-round `CHOSEN COMP` line ("compositional option one ... adds two variations"), which predates this branch. It prints `APPROVED COMP: ...` instead, keyed on the comp sitting directly in `.impeccable/mocks/`. No existing golden changed. New cases: `question-update-comps-landed-no-next` (every declared comp exists, no NEXT line) and `question-wait-answer-comp-round` (`APPROVED COMP` in place of `CHOSEN COMP`).

Provenance by hand timestamp (same branch): deciding whether a comp is owed by existence alone misread files an earlier round left at reused slot paths. Every served hand now records `handAt` and `handDigest` in the state file, and a declared comp counts for the hand only when written at or after `handAt` (comp-round comps directly in `.impeccable/mocks/` excepted). A restart is `--start` with the same key and payload, so the dead-server message now names the key:

- `question-wait-no-server`, `question-wait-dead-pid`: `restart it with --start and the same payload` reads `restart it with --start --key k1 and the same payload`. Exit status and files are unchanged.

New case `question-wait-comp-stale`: `handAt` in 2100 makes the staged comp one an earlier round left, so WAITING is followed by `COMP STALE: ...` naming it, and no `COMP SIDECAR MISSING`. The other decision-comp goldens carry no `handAt`, which falls back to existence, so they did not move.

Hand file and content fingerprints (same branch, supersedes the `handAt` rule above): `--update` wrote the hand into `<key>.state.json`, which the server rewrites on heartbeat and claim, so an overlapping write could drop the hand; and flooring `handAt` to the second let an old image rewritten in the same second pass. Provenance now lives in `<key>.hand.json` (`digest`, `comps`, `pre` fingerprints), written atomically by `--start` and `--update` only, and a comp is this hand's when its bytes differ from the slot's `pre` fingerprint (or the slot had none).

- `question-wait-comp-stale`: the staged provenance moved from `handAt`/`handDigest` in the state file to a hand file whose `pre` fingerprint matches the staged `a.png`. Stdout and exit are unchanged; the snapshot now lists the hand file, and the state file no longer carries hand fields.
- `question-update-comps-next`: now snapshots `.impeccable/questions/k1.hand.json`, the new hand `--update` writes (`pre` is empty because neither slot holds a file). Stdout and exit are unchanged.

Generated slots and hand-write failures (same branch): a deterministic generator returns identical bytes for an unchanged prompt, so a re-roll regenerating into a reused slot failed the fingerprint rule forever. `impeccable generate-image` now marks a written `--out` that a recorded hand declares with a marker file in `<key>.generated/`, and such a slot counts as this hand's. A failed hand write now fails `--start` (before spawning) and `--update` (before delivering) with exit 1. No existing golden changed. New case `genimg-fake-marks-hand-slot`: the fake generator writes a declared slot and the snapshot shows the marker beside the untouched hand file. The failure path is covered by Rust tests, not the oracle, because its stderr carries the OS error text, which differs per platform. Follow-up: markers name their hand by `hand` (the hand's per-hand `id`, falling back to `digest` for a hand file without one) instead of `digest`, so `genimg-fake-marks-hand-slot` now shows `"hand":"0123456789abcdef"` where it showed `"digest"`; a new hand prunes other hands' markers only after its own write succeeds, and `--stop` and a closing answer remove the marker folder. `question-update-comps-next` now snapshots the hand file with its per-hand `id`, which mixes the clock and the pid, so that case masks it as `<HAND_ID>` with a case-scoped normalizer.

## Recorded 2026-09-30: hand-tagged generated markers (#886)

`--update` wrote the new hand and then pruned `<key>.generated/` by reading each marker's `hand` and deleting the file, so a parallel `impeccable generate-image` for the new hand that replaced a reused slot's marker between the read and the delete lost its marker, and a byte-identical regeneration then read as stale. Markers now carry the hand in their name, `<16 hex of the slot>-<hand id>.json`, so one hand's markers never share a path with another's, and the prune decides from the name alone: it deletes only names tagged with another hand. Legacy untagged `<16 hex>.json` markers are still read and pruned by their content `hand` (or `digest`).

- `genimg-fake-marks-hand-slot`: the marker file is now `k1.generated/bc804e5cae3cb360-0123456789abcdef.json` where it was `k1.generated/bc804e5cae3cb360.json`. Its content, stdout, stderr and exit are unchanged.

The interleaving itself is covered by Rust tests in `crates/context/src/serve_question.rs`, not the oracle, since it needs two writers.

## Recorded 2026-09-30: sidecar name spelled out

The decision-comp directives said a comp's prompt goes in `<comp>.json`, which an agent read as the comp's name without its extension (`assigned.json`). The engine checks the image's full file name plus `.json` (`a.png.json`), so the wording now says so: `its sidecar, the image's full file name plus .json (a.png gets a.png.json)`. Only that phrase moved in each golden; exit status, files and every other line are unchanged.

- `question-update-comps-next`, `question-wait-flip`: the `NEXT read ... visualize.md` line ends with the new phrase in place of `(<comp>.json)`.
- `question-wait-comp-sidecar-missing`, `question-wait-answer-comp-sidecar-missing`: `COMP SIDECAR MISSING` names the sidecar the same way, followed by `as {"prompt": "..."} (generate-image writes it itself, a harness image tool does not)`.
- `question-wait-answer-comp-round`: `APPROVED COMP` says `Set "approved": true in its prompt sidecar, the image's full file name plus .json (a.png gets a.png.json)`.
Concept-seed richness instruction scoped by mode (fix/operate-directions): the seed's RICHNESS text told every run to commit an interface-language source "across navigation, content, controls, and states", which contradicted the Operate rule in new-work.md and visualize.md at fusion time. It now allows that on Persuade and Experience surfaces, while Operate and Read take the source's type, density, palette, material accents and one signature move and keep the platform's standard navigation and controls. The 12 seed goldens that print the instruction change in that one sentence only; reviewed by hand.

## Recorded 2026-09-30: Operate deals from the graphic tier

`select_approved_challengers` now takes per-mode tier quotas (`TIER_QUOTAS`), in parity with impeccable-site's roll API (renaissance-geek-inc/impeccable-site#77). Operate draws five graphic worlds and one interaction world and no atmosphere, because instrument and atmosphere worlds dealt to working screens became costumes of the tool. For a mode with quotas, a tier the mode filter empties hands its picks to graphic instead of refilling from worlds closed to the mode. Every other mode keeps two per tier with the same salts, so no persuade, read, experience or unscoped roll moved; `crates/context/src/roll_selection.rs` tests replay 136 rolls recorded from the JS to prove it.

- `seed-direction-local-count-5` (`--mode operate`): challengers 3 to 6 were relay desk, dial cabinet (interaction), pine gallery, dusk quarry (atmosphere); they are now placard row, crest register, stamp folio (graphic) and relay desk (interaction). Header, assigned index and every other line are unchanged.
- `seed-surface-local`, `seed-surface-local-default-scope` (`--mode operate`, surface scope): the atmosphere pair frost arcade and salt terrace and the second interaction pick, signal tower, are gone; folio stand and banner press join placard row and poster wall, with meter row as the one interaction pick. The fixture catalog holds only four graphic duals, so this surface roll deals five challengers where it dealt six: a quota tier reuses its pool when it cannot fill its quota but never borrows another strength. The live catalog holds 16 graphic duals open to operate.
- `seed-direction-local-operate` (new): an operate direction roll on `oracle-key-1`, five graphic and one interaction, no atmosphere.

## Recorded 2026-10-01: MODE RULES printed by concept-seed

Mode-specific rules for directions and comps moved out of the shared reference files into `skill/reference/mode-persuade.md` (persuade and experience), `mode-operate.md` and `mode-read.md`. With `--mode`, `concept-seed` now prints the bodies of the mode file's `## Directions` and `## Comps` sections inside a `MODE RULES (<mode>, from <path>). ...` block, so the agent gets them in output it already reads instead of a file it can skip. The block sits right after the richness instruction on a full roll and after the authority instruction on a degraded one, and prints on every round, re-rolls and both registers included. An unreadable file or a missing section prints one `MODE RULES unavailable: read <path> before writing directions or comps.` line and the roll still succeeds. The richness instruction lost its Persuade/Experience versus Operate/Read sentence, which now lives in the mode files, and reads `Keep a literal carrier only when it becomes functional.` where it read `Otherwise keep ...`.

Seed cases now set `IMPECCABLE_SKILL_DIR` to `tests/fixtures/mode-rules-skill` (placeholder rule text), so these goldens do not move whenever the real mode prose changes. Nothing else concept-seed prints reads the skill dir while `IMPECCABLE_CATALOG_DIR` is pinned.

- Every seed golden that prints the richness instruction (`seed-direction-local`, `-reroll`, `-reroll-bolder`, `-reroll-safer`, `-unscoped`, `-count-5`, `-operate`, `seed-direction-env-key`, `seed-surface-local`, `-default-scope`, `-grain-flow`, `-compositions`, `-card-base`): the richness sentence change above. Those that pass `--mode` also gain the MODE RULES block after it, naming the fixture file for their mode (`seed-surface-local-card-base` prints `experience` with `mode-persuade.md`). The unscoped and safer cases without `--mode` print no block.
- `seed-degraded-direction`, `seed-degraded-surface`: the MODE RULES block after the authority instruction (persuade and operate). Degraded output carries no richness instruction, so nothing else moved.
- `question-update-comps-next`, `question-wait-flip`: the NEXT line reads `it and the MODE RULES block concept-seed printed for this surface govern every card's image` where it read `its comp rules govern every card's image`.

Exit status, stderr and files are unchanged everywhere. New cases: `seed-mode-rules-persuade`, `-experience` (reads `mode-persuade.md`), `-operate`, `-read` (one per mode file, `oracle-key-5`), and `seed-mode-rules-missing-file`, `-missing-section` against `tests/fixtures/mode-rules-skill-partial` (no `mode-read.md`; a `mode-operate.md` without `## Comps`), which print the unavailable line.

## Recorded 2026-10-02: responsive gate names displaced regions, prints crops, escalates

New cases, recorded from the engine and reviewed by hand (no JS golden ever covered the responsive gate's printed output).

- `build-phase-responsive-displaced`: a sign-off line pushed 40px below the first viewport by a growing column reads `displaced, not missing` with the offset and the visible share, the `LOOK FIRST` crop list and the displaced remedy line print, and the third failed `advance` leads with the three-attempt route to the first-viewport review.
- `build-phase-responsive-missing`: the same region absent from the capture still reads `at desktop width, region sign-off is missing`, now with its repair crop listed.

## Recorded 2026-10-02: near-black ink is not gray

`is_gray_ink` counted any low-saturation ink over lightness 0.2 as gray, so
`#393939` on a yellow card and `#413c38` on a green button, which read at 6 to
8:1, reported as gray on colour. On a corpus of real sites those findings were
judged harmless. The floor is now `GRAY_INK_MIN_LIGHTNESS` = 0.3: every
Tailwind neutral at `-700` and darker sits under it, every `-600` and lighter
over it. The Tailwind class paths (the DOM class check and the source-text
matcher) skip `text-{gray,slate,zinc,neutral,stone}-N` for N of 700 and up the
same way. No existing fixture finding moved; the goldens below change only
because of the new `gray-on-color.html` fixture.

- New cases `detect-fixture-json-gray-on-color-html` and `detect-fixture-text-gray-on-color-html`: the fixture's five should-flag rows report (`#d1d5db` on `#1e3a8a` and on `#115e59`, `text-gray-400 on bg-blue-600`, `#4b5563` on `#fcd34d`, `text-gray-600 on bg-amber-400`); its five should-pass rows do not (`#e5e7eb` on `#1e3a8a`, `#393939` on `#ffc224`, `#413c38` on `#38e07b`, `text-gray-800` on `bg-yellow-400`, `#4b5563` on the neutral `#f3f4f6`). The released 0.1.11 engine also reports the three near-black rows (`#393939`, `#413c38` and `text-gray-800`).
- The sweeps `detect-dir-text-all-fixtures`, `detect-dir-quiet-all-fixtures`, `detect-no-advisory-json` and `detect-no-advisory-text` gain the same five findings (437 to 442 counted). Nothing else moved. `detect-dir-json-all-fixtures` was left unrecorded: it is already an accepted delta and a fresh recording also carries unrelated drift.

### Known limits

1. **Near-black ink that is genuinely too dark for its fill** (`#363637` on a
   mid blue) now reports only as `low-contrast`, which is the rule that owns
   that failure.
2. **The class path reads the shade number, not the colour,** so a project
   that redefines `gray-700` lighter than 0.3 is still skipped.
## Recorded 2026-10-04: a native ship binds every captured input

A comp-led run recorded `finish --disposition ship`, then fixed padding findings the Stop hook reported after it, and stopped: the shipped page no longer matched the final native capture. Native `finish --disposition ship` now records `finish.captureInputs`, the `{path, sha256}` rows of the manifest the final responsive capture bound, and `build-phase completion` reports `changed-after-finish` when the entry or any of those files changed, listing them in `changedSinceFinish`. The NEXT lines after review and after a recorded ship now say that a later edit, a hook finding's fix included, needs ship recorded again, and what ship does with the current files under each capture policy. No existing golden printed these lines, so none moved.

New cases, recorded from the binary and reviewed by hand:

- `build-phase-shipped-then-edited`: `status` over a native ship that still covers the page prints the `Finish is recorded for the current entry. Any later edit ...` NEXT; after a font the page loads changes (entry bytes unchanged), `status` prints `A file the final check bound changed after finish (fonts/face.ttf) ...` and `completion` reports `changed-after-finish` with `changedSinceFinish: ["fonts/face.ttf"]`.
- `build-phase-review-next-native`: the review-phase NEXT under native capture, ending `Make every fix before ship: ship re-captures the current files natively and must pass the responsive gate again, and any edit after it, a fix for a hook finding included, needs ship recorded again.`

## Recorded 2026-10-04: a component treatment is clothes

The challenger instruction in direction-scope concept-seed output says what counts as a challenger's clothes. It read `A donation transfers ambition and system discipline, never the challenger's clothes; one world owns the page.` and now reads `A donation transfers ambition and system discipline, never the challenger's clothes. A component treatment, such as a button's shadow or a display face, is clothes, not discipline; one world owns the page.` The lines after it rewrap; their words are unchanged. A gallery run had filed a declined challenger's hard offset shadow on the primary button as a discipline raise.

- `seed-direction-local`, `-reroll`, `-unscoped`, `-count-5`, `-operate`, `seed-direction-env-key`, `seed-mode-rules-persuade`, `-experience`, `-missing-file`, `-missing-section`: that sentence only, reviewed by hand. Exit status, stderr and files are unchanged.

## Recorded 2026-10-04: the approved comp is a fixed reference

A comp-led run composited its generated plates into the approved comp, copied those crops in as the plates, and re-ran `comp-spec` so the spec's `compSha256` followed the edit; the plates gate then measured the work against itself. The engine now keeps a copy of the approved comp (`.impeccable/build/approved-comp.<ext>` plus `approved-comp.json`) when a comp is approved, and every entry point that measures against the comp refuses, before measuring, while its pixels differ from that record. `build-phase restore-comp` puts the copy back.

- `build-phase-usage`: the usage line ends `| finish --disposition <word> | restore-comp`. Nothing else in the case moved.

New case, recorded from the binary and reviewed by hand:

- `build-phase-approved-comp-edited`: `start --comp` and `comp-spec --regions` keep the copy (the snapshotted `approved-comp.json` holds the same pixel hash the spec records); after `build.png` is copied over the comp, `advance` fails the spec gate with the single `the approved comp comp.png has changed since approval: expected pixel sha256 <approved>, found <current>. ...` reason, and a re-run of `comp-spec --regions` and a `comp-diff` against the spec's comp exit 2 with the same message on stderr; `restore-comp` prints `RESTORED comp.png from .impeccable/build/approved-comp.png ...` and names `.impeccable/build/edited-comp-<hash>.png`; the next `advance` measures again and fails on the spec gate's own type reading.

## Recorded 2026-10-05: concept-seed prints the decision round

Codex-harness runs read new-work.md through a shell that cut the middle of the file, losing exactly the decision-page and build-path paragraphs; the run then presented the direction through the structured question tool and asked the retired build-path question. Every successful `concept-seed` roll (direction or surface, full or degraded, every re-roll and both registers) now ends with a `PRESENTATION (the decision round, condensed from new-work.md; ...)` block of five lines after the restated line, in working order: how to serve the hand (`serve-question --start` on the first round; on a re-roll, `--update --key <same key>` while a page is open and `--start` when none opened yet), which cards declare comps (direction: canon included, declined excepted; degraded direction: one text-only card, except the safer register's full lineup; surface: comps or wireframes, no pick or canon; code-led: comp paths as a flip reserve), holding `--wait` (after the last comp lands, or right after serving on a code-led round or a single degraded card) and through a shell that hands back a session, the build path, and when the structured tool is the fallback. The build-path line names the recorded default and its file, resolved like `context`'s `BUILD_PATH_DEFAULT` (`.impeccable/config.local.json` over `config.json`), or says none is recorded.

- Every seed golden that prints a roll (`seed-direction-local`, `-reroll`, `-reroll-bolder`, `-reroll-safer`, `-unscoped`, `-count-5`, `-operate`, `seed-direction-env-key`, `seed-mode-rules-*`, `seed-surface-local`, `-default-scope`, `-grain-flow`, `-compositions`, `-card-base`, `seed-degraded-direction`, `-surface`, `-safer`, `-bolder`): the block appended after the last line, reviewed by hand. Everything before it is byte-identical; exit status, stderr and files are unchanged. Validation errors, the PRODUCT.md gate and the telemetry pings print no block.

New cases, recorded from the binary and reviewed by hand: `seed-presentation-build-path-code` (direction roll with `.impeccable/config.json` `code`: the code-led flip-reserve comp line and `recorded default code (from .impeccable/config.json)`) and `seed-presentation-build-path-local` (surface roll where `config.local.json` `code` beats `config.json` `comp`: the code-led wireframe line and `(from .impeccable/config.local.json)`).

## Recorded 2026-10-05: decision comps get their render checks once per round

A Gemini run wrote decision comp prompts that inventoried every region and never opened the rendered comps, so visualize.md's post-render checks never ran. `serve-question --wait` now prints `NEXT open each decision comp once and run ${visualize path}'s render checks (shipped screen, one dominant move); regenerate any that fail before the user answers.` as the last line of a WAITING return, once per hand, when a decision comp of this hand has landed with its sidecar. It records the hand's id in `<key>.render-check` so later polls of the same hand stay quiet.

- `question-wait-comp-sidecar-missing`: b landed with its sidecar, so the new NEXT line follows `COMP SIDECAR MISSING`, and the files now include `.impeccable/questions/k1.render-check` with the empty id (the case has no hand file). Exit status and stderr are unchanged.

New case, recorded from the binary and reviewed by hand:

- `question-wait-render-check-once`: `k1.render-check` already holds the hand's id `h1`, so a poll with a landed, sidecar-carrying decision comp prints only the WAITING line and leaves the marker as it was.

## Recorded 2026-10-05: the chosen decision comp is option one of the comp round

The skill says the direction round's chosen decision comp enters the comp round as compositional option one and is never regenerated, but the comps gate counted only files directly in `.impeccable/mocks/`, while decision comps live in `.impeccable/mocks/decision/`. `build-phase start --direction <key> --decision-comp <png>` now records that comp as `decisionComp` in the state; the comps gate counts it first where it stands, never any other decision comp, and an approval on it closes the gate with the decision path as the approved comp (record, kept copy and `restore-comp` bind to it). `serve-question --wait` treats a pick of that recorded comp during the open comps phase as the approval (`APPROVED COMP`), and the `CHOSEN COMP` line names the flag.

- `build-phase-usage`: the usage line now reads `start --comp <png> | --direction <key> [--decision-comp <png>] [--breakpoint WxH] ...`. Nothing else in the case moved.
- `question-wait-answer-ready`, `question-wait-answer-comp-sidecar-missing`: the `CHOSEN COMP` comp-led clause reads `On a comp-led build pass it to build-phase start as --decision-comp <that path>, and the comp round adds two variations beside it where it stands;`. Exit status, stderr and files are unchanged.

New cases, recorded from the binary and reviewed by hand:

- `build-phase-decision-comp-option-one`: `--decision-comp` with `--comp` and with a missing file exit 1; `start --direction seed --decision-comp` prints the option-one NEXT; the first `advance` fails with `1 comp (the chosen decision comp ... as option one, the others directly under .impeccable/mocks)` and no approval, although an unrelated decision comp carries `"approved": true`; after two comps land in `.impeccable/mocks/` and the decision comp's sidecar is approved, `advance` closes on it (`3 comps, 1 approved`), and the snapshotted state and `approved-comp.json` name the decision path.
- `question-wait-answer-decision-comp-in-round`: with a build state in the `comps` phase recording `decisionComp` and a page that also serves a comp directly in `.impeccable/mocks/`, a pick of that path prints `APPROVED COMP`, not `CHOSEN COMP`.

## Recorded 2026-10-05: surface rounds owe their decision comps too

`serve-question --start` and `--update` refused only comp-led direction rounds with missing decision comps, because a surface round's payload carries no canon and looks like the comp round. Every successful `concept-seed` roll now writes `.impeccable/questions/roll.json` (`scope`, `key`, `reroll`, `at`), and while the latest roll is a surface roll under an hour old that no decision page has taken, a comp-led surface hand that leaves a dealt card without a comp is refused with the same message helper (exit 1, before any state is written). The degraded roll's approval guidance said `nothing is written`; it now names the record.

- `seed-degraded-direction`, `seed-degraded-surface`, `seed-degraded-bolder`: in the no-network paragraph, `and\nnothing is written.` became `and\nthe only file written is the local roll record .impeccable/questions/roll.json,\nwhich serve-question reads.`. Nothing else moved; exit status and stderr are unchanged, and the cases snapshot no files.

New cases, recorded from the binary and reviewed by hand:

- `question-start-surface-missing-comps`: a degraded surface roll, then `--start` of a comp-led three-card surface hand with no comps exits 1 naming `ledger, rail, field`; no hand is recorded and `roll.json` stays.
- `question-update-surface-missing-comps`: a surface re-roll, then `--update` of the same hand with only `ledger` declared exits 1 naming `rail, field`; nothing is delivered and `roll.json` stays.
- `question-update-surface-shape-after-direction-roll`: after a direction roll the same comp-less hand is delivered, and the page takes the roll (`roll.json` is gone).
