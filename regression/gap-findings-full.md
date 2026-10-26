# PTUI Full E2E — Gap Findings (Run 23)

- Generated: 2026-10-26T00:11:21.931211Z
- Project: PTUI (id 29, slug `ptui-spec`) — permanent prod project
- Release version id: 16  |  Audit branch: main
- Run: id 23 "PTUI Full E2E", state **completed**, completed 2026-10-26T00:10:06.370Z
- Items: 33 total | done 33 | failed 0 | failed_perm 0 | cancelled 0
- Avg score (after): **30.52** | min 8.0 | max 62.0

## Score Distribution

| Bucket | Count |
|---|---|
| 0-19 | 8 |
| 20-39 | 14 |
| 40-59 | 10 |
| 60-79 | 1 |

## Verdicts

- **major_gaps**: 27
- **needs_refinement**: 6

## Tool Issue: UTF-8 audit skips

10 items returned `Audit skipped: invalid byte sequence in UTF-8`. Each still carries a score (prior/heuristic retained — not re-run per run policy). This is a CLI/audit-pipeline encoding bug to investigate, not a spec gap.

- F-2: Audit skipped: invalid byte sequence in UTF-8
- F-13: Audit skipped: invalid byte sequence in UTF-8
- F-15: Audit skipped: invalid byte sequence in UTF-8
- F-18: Audit skipped: invalid byte sequence in UTF-8
- F-22: Audit skipped: invalid byte sequence in UTF-8
- F-25: Audit skipped: invalid byte sequence in UTF-8
- T-28: Audit skipped: invalid byte sequence in UTF-8
- T-30: Audit skipped: invalid byte sequence in UTF-8
- T-32: Audit skipped: invalid byte sequence in UTF-8
- T-33: Audit skipped: invalid byte sequence in UTF-8

## Per-Item Gap Findings

### F-1 — Image preview
- Score: **42.0** | Verdict: major_gaps
- Top gaps:
  - F-1: Enumerated list of supported image formats with pass/fail for each.
  - F-1: Maximum acceptable latency from file selection to preview render.
  - F-1: Behavior when no preview is available (unsupported format, binary, permission denied) — error message, placeholder, or silent blank.

### F-2 — ANSI terminal graphics rendering
- Score: **18.0** | Verdict: major_gaps | ⚠ Audit skipped: invalid byte sequence in UTF-8
- Top gaps:
  - Definition of which specific ANSI/escape-code rendering technique is used (sixel, half-block Unicode, true-color ANSI blocks, etc.).
  - List of supported input image formats and behavior for unsupported formats.
  - Specified output dimensions (columns × rows, or pixel bounds) for the rendered preview.

### F-3 — Graphical protocol support (kitty, iTerm2)
- Score: **18.0** | Verdict: major_gaps
- Top gaps:
  - Explicit list of target terminals that must pass acceptance (e.g., kitty terminal, iTerm2, Ghostty — with minimum versions).
  - Acceptance criterion for kitty graphics protocol: which version/revision of the protocol specification must be supported (e.g., direct/chunked transfer, Unicode placeholders, etc.).
  - Acceptance criterion for iTerm2 inline image protocol: which version and required header fields (width, height, preserveAspectRatio, inline=1) must be emitted correctly.

### F-4 — Picture-to-text converters (chafa, jp2a)
- Score: **28.0** | Verdict: major_gaps
- Top gaps:
  - No acceptance criteria of any kind are defined.
  - No input specification: what image formats are accepted (JPEG, PNG, GIF, SVG, WebP)?
  - No output specification: what is the expected output type (ANSI escape string, plain ASCII, Unicode block characters)?

### F-5 — Switch converters with TAB
- Score: **28.0** | Verdict: major_gaps
- Top gaps:
  - Definition of the ordered list of converters that participate in the TAB cycle.
  - Behavior when only one converter is available (TAB pressed with no alternative): should it be a no-op, show a message, or still re-render?
  - Behavior when no converter is available for the current file type.

### F-6 — Caching of rendered images
- Score: **18.0** | Verdict: major_gaps
- Top gaps:
  - Cache hit rate or measurable performance improvement threshold (e.g., 'cache hit must serve result in < X ms')
  - Cache key definition: exact fields that uniquely identify a cached entry (file path, mtime/hash, terminal size, converter, converter version, settings)
  - Cache invalidation rules: when is a cached entry considered stale or invalid?

### F-7 — Text file preview
- Score: **28.0** | Verdict: major_gaps
- Top gaps:
  - Explicit list of supported file types/extensions that trigger text preview mode
  - Detection mechanism: how is a file identified as 'text' vs. 'binary' (extension, MIME type, magic bytes)?
  - Maximum file size limit: what happens when a very large text file is opened (e.g., 1 GB log file)?

### F-8 — File browser navigation
- Score: **42.0** | Verdict: major_gaps
- Top gaps:
  - Arrow key navigation: exactly one item moves per keypress in the expected direction.
  - j/k navigation: identical behavior to down/up arrow keys confirmed by test.
  - Page Up/Down and Ctrl+F/Ctrl+B: the number of items skipped per keypress is defined and consistent.

### F-9 — Directory traversal
- Score: **22.0** | Verdict: major_gaps
- Top gaps:
  - Given a directory entry is highlighted, when Enter is pressed, the file list must update to show the contents of that directory.
  - Given the user is at the filesystem root, when Backspace is pressed, the action must be a no-op (or display a specific feedback) without crashing or navigating further.
  - Given the user navigates into a subdirectory and then presses Backspace, the cursor/selection must land on the directory entry that was just entered (return-to-origin behavior).

### F-10 — Scrollable file lists
- Score: **18.0** | Verdict: major_gaps
- Top gaps:
  - Given a directory with N files where N exceeds the visible area, the list MUST be scrollable (define N or tie it to viewport size).
  - The currently selected/highlighted item MUST always be fully visible within the viewport after any navigation action.
  - Specific key bindings for scroll actions must be listed (e.g., ↑/↓, j/k, Page Up/Down, Home/End, mouse wheel).

### F-11 — Re-read file list (r)
- Score: **22.0** | Verdict: major_gaps
- Top gaps:
  - No acceptance criteria are defined at all — the spec contains only a single-sentence description.
  - Expected behaviour when the currently selected file has been deleted between reads.
  - Expected behaviour when the directory itself no longer exists or is inaccessible.

### F-12 — Open in system file browser (o)
- Score: **22.0** | Verdict: major_gaps
- Top gaps:
  - Expected behavior on each supported platform (macOS: open Finder at path; Windows: open Explorer at path; Linux: attempt known browsers or show error).
  - Behavior when no system file browser is detected or available — error message text, fallback action, or silent no-op must be specified.
  - Definition of what 'current location' resolves to in all states: file selected, directory selected, no item selected, empty list.

### F-13 — Slideshow mode
- Score: **28.0** | Verdict: major_gaps | ⚠ Audit skipped: invalid byte sequence in UTF-8
- Top gaps:
  - Acceptance criterion for initiating slideshow: preconditions (minimum number of images, application state) must be defined
  - Acceptance criterion for auto-advance: slide must advance automatically after exactly slideshow_delay_ms milliseconds without user input
  - Acceptance criterion for manual advance: pressing configured keys must immediately display the next/previous slide and optionally reset the timer

### F-14 — Star ratings
- Score: **62.0** | Verdict: needs_refinement
- Top gaps:
  - F-14: No acceptance criterion for what happens when rating is applied to multiple selected files simultaneously — is bulk rating supported?
  - F-14: No criterion for persistence across app restarts when using sidecar mode (covered implicitly by F-15 but not linked explicitly).
  - F-14: No criterion for the display of a rating of 0 (cleared) — is the star/number hidden entirely, or shown as empty stars or '0'?

### F-15 — XMP sidecar storage
- Score: **31.0** | Verdict: major_gaps | ⚠ Audit skipped: invalid byte sequence in UTF-8
- Top gaps:
  - Definition of the valid rating value domain (e.g., integers 0–5) and how out-of-range values are handled.
  - Exact sidecar file structure: which XMP packet wrapper format (read-only vs. read-write), required namespaces, and mandatory/optional metadata fields beyond xmp:Rating.
  - Criterion for what constitutes 'interoperability' with each named application (e.g., 'digiKam 7.x must read a sidecar written by this app and display the correct star rating without import errors').

### F-16 — Sidecar naming conventions
- Score: **41.0** | Verdict: major_gaps
- Top gaps:
  - Explicit algorithm for detecting which convention a folder uses (e.g., first file found, majority count, presence of any file of each type).
  - Behavior when a folder contains a mix of both conventions — read strategy and write strategy must each be specified.
  - Definition of 'orphaned sidecar' with exact matching rules (base name comparison, extension case sensitivity, multi-extension stacking like .jpg.xmp vs .jpeg.xmp).

### F-17 — Non-destructive sidecar updates
- Score: **32.0** | Verdict: major_gaps
- Top gaps:
  - Given an existing sidecar with darktable edit history, when xmp:Rating is updated, then the darktable history blocks must be byte-for-byte identical to their pre-update state.
  - Given an existing sidecar that does not contain an xmp:Rating node, when a rating is written, then xmp:Rating is inserted and all pre-existing fields remain unchanged.
  - Given an existing sidecar that does not contain an xmp:MetadataDate node, when a rating is written, then xmp:MetadataDate is inserted with the current UTC timestamp.

### F-18 — Private rating fallback
- Score: **22.0** | Verdict: major_gaps | ⚠ Audit skipped: invalid byte sequence in UTF-8
- Top gaps:
  - No acceptance criteria of any kind are defined for this requirement.
  - No definition of the state.json schema or the shape of a private rating entry.
  - No specification of how 'ask' prompt is presented to the user (modal, toast, inline, etc.) and what the exact options are.

### F-19 — Rating follows file operations
- Score: **28.0** | Verdict: major_gaps
- Top gaps:
  - Given a file with an associated XMP sidecar, when the file is deleted, then the sidecar file is also deleted — and vice versa is NOT true (deleting the sidecar alone does not delete the file).
  - Given a file with a sidecar, when the file is copied to a new location, then a copy of the sidecar appears at the new location with a correctly derived name (per F-16).
  - Given a file with a sidecar, when the file is moved, then the sidecar is moved (not copied) and the original sidecar no longer exists at the source location.

### F-20 — Sort file list
- Score: **52.0** | Verdict: needs_refinement
- Top gaps:
  - Explicit acceptance criterion defining the exact toggle behavior: press once → ascending, press again → descending, press again → ??? (reset or stay descending).
  - Acceptance criterion for what constitutes 'unrated': null value, zero value, or absent field, and whether unrated files always trail or lead the rated ones.
  - Criterion specifying which date field drives date sort, with a fallback if that field is absent.

### F-21 — File operations
- Score: **42.0** | Verdict: major_gaps
- Top gaps:
  - F-21: No acceptance criteria defined at the parent level; the parent spec adds no value beyond listing children and should at minimum define the shared interaction model (e.g., keybinding conflicts, behavior when no file is selected).
  - F-22: What happens when deletion fails (permission denied, file locked)? Expected error message or behavior not specified.
  - F-22: No definition of 'selected file' — is exactly one file always selected when this key is pressed, or must the spec handle multi-selection or no-selection states?

### F-22 — Delete file (x)
- Score: **18.0** | Verdict: major_gaps | ⚠ Audit skipped: invalid byte sequence in UTF-8
- Top gaps:
  - Confirmation dialog: is the user prompted before deletion, or is the action immediate and irreversible?
  - Success state: what feedback is given to the user after successful deletion (e.g., UI update, status message, cursor moves to next file)?
  - Failure/error handling: what happens if the file is locked, in use, or the user lacks permissions to delete it?

### F-23 — Copy / move file (c / m)
- Score: **47.0** | Verdict: major_gaps
- Top gaps:
  - Acceptance criterion for successful copy: source file exists unchanged at source, identical file exists at destination.
  - Acceptance criterion for successful move: file no longer exists at source, identical file exists at destination.
  - Acceptance criterion for the shortcut list: exact list contents, ordering, and numbering are verified at display time.

### F-24 — Save picture to ASCII (i)
- Score: **8.0** | Verdict: major_gaps
- Top gaps:
  - Definition of supported input image formats
  - Expected output format and file extension
  - Output file naming convention

### F-25 — Multilingual support
- Score: **18.0** | Verdict: major_gaps | ⚠ Audit skipped: invalid byte sequence in UTF-8
- Top gaps:
  - Definition of coverage: percentage or explicit list of UI surfaces that must be translated (all user-visible strings vs. a subset).
  - Fallback chain: explicit behavior when a translation key is absent for the active locale.
  - Locale detection priority order: config key vs. browser Accept-Language header vs. user profile preference vs. OS locale.

### F-26 — Dynamic window resizing
- Score: **22.0** | Verdict: major_gaps
- Top gaps:
  - Definition of minimum allowable window width and height after resizing.
  - Definition of maximum allowable window width and height (e.g., cannot exceed terminal dimensions).
  - Resize step size: how many rows/columns change per keypress of '[' or ']'.

### F-27 — Help and quit controls
- Score: **22.0** | Verdict: major_gaps
- Top gaps:
  - Definition of the help content: what information is displayed, in what format, and how is it dismissed?
  - Expected exit code(s) on quit
  - Whether a confirmation prompt is shown before quitting (e.g., 'Unsaved changes — are you sure?')

### T-28 — Configuration file
- Score: **42.0** | Verdict: needs_refinement | ⚠ Audit skipped: invalid byte sequence in UTF-8
- Top gaps:
  - No schema or structural definition for ptui.json (keys, types, constraints, required vs optional fields).
  - No description of runtime config loading: how the application reads and applies config values.
  - No error/fallback handling for directory creation failures, JSON parse errors, or missing required keys.

### T-29 — Partial config with defaults
- Score: **22.0** | Verdict: major_gaps
- Top gaps:
  - No technical implementation detail is provided — the spec reads as a one-line concept statement, not an actionable engineering specification.
  - Default values are not enumerated or referenced; there is no canonical source of truth for what the defaults are.
  - The mechanism for merging partial user config with defaults is unspecified (e.g., deep merge vs shallow merge, precedence order).

### T-30 — Live config reload
- Score: **8.0** | Verdict: major_gaps | ⚠ Audit skipped: invalid byte sequence in UTF-8
- Top gaps:
  - No detection mechanism specified (file watcher, polling, OS signal, etc.)
  - No description of which configuration keys or sections support live reload vs. require restart
  - No validation or error handling strategy for invalid/malformed config during reload

### T-31 — Config parse-error resilience
- Score: **52.0** | Verdict: needs_refinement
- Top gaps:
  - No technical mechanism described to enforce 'leave file untouched' (e.g., parse-before-write, no auto-save on error, file locking strategy).
  - No definition of what 'defaults' are: hard-coded values, a bundled default config file, or last-known-good state.
  - No specification of whether the user receives a secondary notification (e.g., status bar indicator, modal on startup) that defaults are active, beyond the messages pane entry.

### T-32 — Runtime requirements
- Score: **52.0** | Verdict: needs_refinement | ⚠ Audit skipped: invalid byte sequence in UTF-8
- Top gaps:
  - No minimum version constraints specified for any dependency (chafa, ImageMagick/identify, jp2a, nasm).
  - No clear separation between runtime dependencies and build-time dependencies; nasm is a build-time tool but is listed alongside runtime tools.
  - No description of runtime detection behavior: what happens if a mandatory dependency (chafa, ImageMagick) is missing at runtime?

### T-33 — Installation channels
- Score: **52.0** | Verdict: needs_refinement | ⚠ Audit skipped: invalid byte sequence in UTF-8
- Top gaps:
  - No minimum Rust toolchain version or edition specified for source builds.
  - No system-level prerequisites listed for either Arch Linux or macOS (e.g. libjpeg-turbo, Xcode CLT).
  - No explanation of what the 'fast-jpeg' feature flag does, when it should be enabled, or what dependencies it requires.
