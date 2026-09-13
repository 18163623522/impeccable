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
