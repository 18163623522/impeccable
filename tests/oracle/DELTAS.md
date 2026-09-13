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
