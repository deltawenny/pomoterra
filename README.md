# pomoterra

A compact Pomodoro timer written in Rust with Timer and Config tabs. Its UI is `src/lib.rs`, and `nsh.json` names the root component (`"root": "App"`).

## Timer

The Timer view has a blue countdown ring, a session note, and start/pause, reset,
skip, announcement, and full-screen controls. Focus sessions default to 25 minutes;
breaks last 5 minutes, with a 15-minute break after every fourth focus session.
The next session waits for you to press Start. Reset restarts the current phase.

Click the three dots to edit or save a session note. The speaker control toggles
screen-reader completion announcements (not an audio alarm). Notes and timer
state are kept in memory for the current app run.

## Config

The Config tab adjusts focus time (1–120 minutes), short and long breaks
(1–60 minutes), and session rounds (1–10). Sliders support dragging and keyboard
arrows. Changing a rule resets the timer to the first focus session. Restore
Default resets the rules to 25/5/15 minutes and four rounds, and clears special breaks.

Four optional special breaks each accept a local 24-hour start time (`HH:MM`)
and a duration of 1–180 minutes. Incomplete or invalid entries have a red border
and do not run. A matching time interrupts an active focus session once per day;
after the break, the remaining focus time is ready to resume. Schedules only
run while the app is open. Rules and schedules are kept in memory for this app run.

Timer-state tests live in `src/store.rs`; `App.test.tsx` tests the UI controls.

```sh
nsh run                      # this machine's backend; --target ios, android, web, macos, linux or windows
nsh check                    # cargo check of the crate, then type checks App.test.tsx
nsh test                     # cargo test, then App.test.tsx: the root tested from TypeScript, headless; --target to run it on a platform
nsh test nsh.json --trace    # headless: what the root mounts, and the time of each commit
nsh package --target web     # or ios, android, macos, linux, windows
```

`nsh run` and `nsh package` mount the root on every platform, the browser included, without QuickJS: the app is Rust
and native code from start to end (in the browser, with NSH's page script). Rust edits rebuild and restart the app.

```sh
cargo run
```

This starts the root directly on the machine's native desktop backend: GTK 4 on Linux, AppKit on macOS, Win32 on
Windows. Install the backend's development libraries before building.

The `nsh` commands build the app's runners from the NSH crates `Cargo.toml` depends on: crates.io's at the version of
the `nsh` you run (keep the `nsh-*` dependencies at that version when you upgrade `nsh`), or an NSH checkout's by path.

## Icons

Use the [Lucide catalog](https://lucide.dev/icons/) through the pinned
[lucide-icons crate](https://docs.rs/lucide-icons/1.70.0/lucide_icons/).
All UI icons go through [`src/icons.rs`](src/icons.rs), which reexports the
crate's typed catalog and renders decorative glyphs in a fixed square.
Choose a catalog variant and pass its color and size in logical pixels to the
shared renderer; existing examples are in [`src/lib.rs`](src/lib.rs).
Keep the action's accessible label on the enclosing button. The renderer hides
its glyph from assistive technologies so screen readers announce the action.

To add an icon, select its matching Rust variant from the crate and use the
shared renderer. Avoid inline SVG icon paths, Unicode literals, or a second
icon library. The countdown ring remains custom SVG because it displays timer
progress rather than a catalog icon.

This crate supplies font glyphs rather than SVG paths. The renderer uses the
bundled Lucide font, with size and color as its styling options. The direct
desktop launcher registers the dependency's font bytes. [`build.rs`](build.rs)
exposes those same bytes as an NSH font asset for generated runners, TSX entries,
and packaging; no font needs installing on the user's machine. Keep the normal
and build dependency versions identical when updating Lucide, then check icon
appearance and run the existing UI tests.

Lucide icons are ISC licensed, with Feather-derived portions under MIT; the
Rust crate is MIT licensed. Attribution and license texts are included in
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md). Include these notices when
redistributing the app and bundled font.

## Adding a TSX entry

TSX screens can use the Rust components and modules, with hot reloading for the TSX. The crate's set,
`#[native_module("pomoterra/native")]` in `src/lib.rs`, already exports `App`. Add an `App.tsx`:

```tsx
import { mount } from "nsh";
import { App } from "pomoterra/native";

mount(App);
```

then run it with `nsh run App.tsx` (`tsconfig.json` is there for it and the test): a named TSX entry
runs on QuickJS, while `nsh run` alone keeps running the Rust root. The first `nsh build App.tsx` writes the TypeScript
declarations of `pomoterra/native` into `.nsh/types`. Register more with `registry.register::<…>()` (components),
`registry.module::<…>()` and `registry.class::<…>()`.

NSH-generated files stay in `.nsh/`: runner sources in `.nsh/rust/` and default packages in `.nsh/out/`.
Cargo keeps its normal `target/` directory and respects your existing `CARGO_TARGET_DIR`.
