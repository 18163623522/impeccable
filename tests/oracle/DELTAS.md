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
