# Pomoterra technical guide

[← Back to the project overview](../README.md)

A compact Pomodoro timer written in Rust with Timer, Config, and History tabs. Its UI is `src/lib.rs`, and `nsh.json` names the root component (`"root": "App"`).

## Timer

The Timer view has a blue countdown ring, a session note, and start/pause, reset,
skip, announcement, and full-screen controls. Focus sessions default to 25 minutes;
breaks last 5 minutes, with a 15-minute break after every fourth focus session.
The next session waits for you to press Start. Reset restarts the current phase.

Click the three dots to edit or save a session note. The speaker control toggles
an audible, two-note completion chime and screen-reader announcements for both
focus sessions and breaks. Unmuting plays a preview; Reset and Skip do not ring.
The chime works independently of system notification banners, including in GPUI
builds. Playback uses Rodio on the default audio output device, with no external
players or temporary sound files. The built-in chime is generated in memory and
decoded through the same pipeline that can later support user-selected sounds.
WAV, MP3, Ogg Vorbis, and FLAC decoding are enabled; choosing and saving custom
sounds is not implemented yet. Linux builds require ALSA development files
(`libasound2-dev` on Ubuntu/Debian, `alsa-lib-devel` on Fedora), and runtime
playback requires a working default audio device. No sound-file download is needed.
The speaker setting, notes, and timer countdown state are kept in memory for the
current app run. Completed focus sessions save their notes to SQLite.

## Background operation and tray

The default native-backend build has the tray integration described below.
**GPUI release builds currently do not install these native-backend hooks:** closing
the GPUI window exits the app, and native minimum-window-size enforcement is not
installed. Keep the GPUI window open for timers and scheduled breaks to continue.

On Linux, Windows, and macOS, closing the native-backend window hides it to the system tray
(macOS menu bar) instead of
stopping the app. An active timer and scheduled breaks continue working while
hidden; restoring the window preserves the current timer, settings, and note.
The next session still waits for Start, just as it does with the window visible.

The tray uses a theme-aware countdown icon and shows the selected profile,
current Focus/Break phase, running/paused state, and remaining / total phase time
(for example, **24:12 / 25:00**). Its **Switch profile** submenu lists the quick
profiles chosen under **Settings → Quick switch configuration profiles**, with
the active profile checked. Switching uses the same behavior as the Timer tab:
it saves the old profile's rules and starts a fresh, paused focus cycle for the
new profile. Selecting the already-active profile leaves the timer unchanged.
If no shortcuts are selected, the submenu points to Settings.

The menu also includes a **Play / Stop** action: Play starts or resumes the
current countdown, while Stop pauses it without resetting the remaining time.
The label follows the timer state, including changes made in the app window.
**Restore the app** and **Quit** actions remain available.
On Linux, clicking the icon also restores the window. On Windows and
macOS, clicking opens its menu. **Quit** exits completely; Linux also supports
Ctrl+Q and macOS retains its normal Cmd+Q application shortcut. On Linux,
Ctrl+W hides the window; Windows Alt+F4 and macOS's close button do the same.
This integration is installed by the Rust root in both the direct desktop
launcher and NSH-generated desktop runners. Restoring keeps the same engine
and timer state rather than mounting a new app.

On Linux, a StatusNotifier-compatible desktop tray and a session D-Bus connection are
required (for example, KDE Plasma or GNOME with an AppIndicator extension).
If tray registration fails, closing the window exits normally. If the tray host
disappears while the app is running, the window is restored so it stays reachable.
No AppIndicator/GTK 3 system library is required. Windows uses a native
notification-area icon; macOS uses a native status item. Platform-specific
objects stay on their operating system's UI thread. If native tray creation
fails, normal close-to-exit behavior is retained.

Mobile and web retain their existing UI and lifecycle without desktop tray
dependencies. A desktop tray does not provide mobile background execution or
keep a closed browser tab running.

Desktop CI checks compilation and unit tests on Linux, Windows, and macOS.
Native interaction on Windows/macOS still needs validation on those systems;
Linux cross-compilation alone cannot verify their desktop behavior.

## Config

The Config tab adjusts focus time (1–120 minutes), short and long breaks
(1–60 minutes), and session rounds (1–10). Sliders support dragging and keyboard
arrows. Changing a rule resets the timer to the first focus session. Restore
Default resets the rules to 25/5/15 minutes and four rounds, and clears special breaks.

Four optional special breaks each accept a local 24-hour start time (`HH:MM`)
and a duration of 1–180 minutes. Incomplete or invalid entries have a red border
and do not run. A matching time interrupts an active focus session once per day;
after the break, the remaining focus time is ready to resume. Schedules only
run while the app is open. Rules and schedules are saved automatically per profile in SQLite.

The profile picker supports searching, adding, renaming, and deleting profiles.
Switching profiles starts a fresh focus cycle with that profile's rules. Deleting
requires confirmation and keeps all recorded history; at least one profile must remain.
Choose a profile color from the palette or enter a six-digit hex color. Enable
Payable to set a rate per hour and currency. Rates accept up to two decimal places.
Amounts use integer hundredths of the selected currency; different currencies are
reported separately, without conversion.

## History and storage

History records each naturally completed focus period once, including its profile,
start and finish times, focused duration, note, color, and billing rate. Pauses and
scheduled breaks do not add billable time. Reset and Skip discard the current
unfinished period. Changing timer rules or switching profiles resets the current
cycle. A scheduled break preserves the interrupted focus period until it finishes.
Sessions reference profiles by ID. Renaming a profile or changing its color updates
all its history immediately, including charts and filters. Deleted profiles are
archived so their history remains available. Billing rates and currencies remain
snapshots of each completed session, so pricing changes do not alter past earnings.
Toasty creates the database schema on first startup. Legacy development schemas
are not migrated; use a fresh database when changing an incompatible model.

The yearly contribution calendar shows daily session counts, using profile colors
and stronger shades for busier days. Scroll horizontally to explore the year.
Today, Week (Monday onward), Month, Year, and All filters use local calendar dates;
the calendar always shows the last 365 days. Filter by profile, including deleted
profiles with history, and reverse the session date order. The profile bar chart
shows focus duration for the selected period, alongside session and earnings totals.

To inspect the History UI without waiting for real sessions, click **Show sample
history** in the History tab. This previews over a thousand sample sessions across
400 days, with multiple profile colors, paid and unpaid work, USD/EUR earnings,
and varied notes. All period filters, profile filters, charts, and date ordering
work on the preview. **Show real history** switches back and resets the profile
filter. Sample data stays in memory only: it never changes saved profiles or
sessions, and restarting the app returns to real history.

Database reads and writes use Toasty 0.11 with its SQLite driver. Typed table
mappings live in [`src/persistence/models.rs`](../src/persistence/models.rs), and
[`src/persistence.rs`](../src/persistence.rs) keeps the UI-facing storage API
synchronous using a dedicated Tokio runtime. Toasty owns schema creation and all
database access; there is no direct SQLite connection or legacy migration layer.
Existing incompatible development databases must be removed manually or replaced
by choosing a new database location.
Building requires Rust 1.95 or newer (Toasty's minimum supported version).

SQLite is bundled with the app. The database is stored in the operating system's
local application data directory under `pomoterra/pomoterra.sqlite3`:

- Linux: `$XDG_DATA_HOME/pomoterra/pomoterra.sqlite3`, normally `~/.local/share/pomoterra/pomoterra.sqlite3`.
- Windows: `%LOCALAPPDATA%\\pomoterra\\pomoterra.sqlite3`.
- macOS: `~/Library/Application Support/pomoterra/pomoterra.sqlite3`.

Set `POMOTERRA_DB_PATH` to use another location, or `:memory:` for an isolated test
run. Storage failures appear in the app; failed session writes remain queued in
memory and retry while the app is open. Quit only after resolving any storage
error to retain those pending records. Profiles and history survive restarts;
unfinished countdowns and draft notes do not. The SQLite backend requires a native
filesystem; browser persistence is not supported by this backend.

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

## Desktop releases

Push a version tag to run [Desktop release](../.github/workflows/release.yml).
It tests and builds a **GPUI host** on each operating system, creates packages,
and publishes a GitHub Release with generated notes only after **all five targets
succeed**. No personal access token or release secret is required: publishing uses
GitHub Actions' built-in repository token. Actions must be enabled in the repository.

| System | Architecture | Downloads |
| --- | --- | --- |
| Linux (Ubuntu 24.04 build baseline) | x64 and ARM64 | Portable tar archive and Debian package |
| Windows | x64 | Portable ZIP |
| macOS 14+ | Intel and Apple Silicon | App bundle ZIP and drag-to-Applications DMG |

Every filename includes the version, system, and architecture. Each download has
an individual SHA-256 checksum; the release also includes a combined checksum
manifest. Packages include the bundled Lucide font and third-party notices, and
use the existing per-user database location.

### GPUI host and local builds

The [GPUI integration guide](https://nsh.dev/docs/gpui/) describes embedding a Rust
root in a GPUI host. [src/main.rs](../src/main.rs) does this using NSH's GPUI
launcher; the release workflow selects it with:

```sh
cargo run --no-default-features --features gpui
cargo build --release --locked --no-default-features --features gpui
```

Plain `cargo run` retains the existing native launcher and its tray integration.
`nsh package` still builds NSH's OS-native runners, not this GPUI host. The release
workflow deliberately builds our executable and packages it with the existing
[packaging helper](../.github/scripts/package.py). Desktop CI checks both backends.
GPUI uses Metal on macOS, DirectX on Windows, and Vulkan on Linux. Its
runtime-shaders feature compiles Metal shaders at startup, so CI does not need to
download Xcode's Metal Toolchain. Web views, video, and the JavaScript runtime are
not enabled in this Rust-only GPUI host.

Linux GPUI build prerequisites (Ubuntu 24.04):

```sh
sudo apt-get install clang libclang-dev pkg-config libfontconfig-dev libfreetype-dev \
  libwayland-dev libx11-xcb-dev libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev \
  libzstd-dev libvulkan-dev
```

### Publish a version

1. Set the same version in [Cargo.toml](../Cargo.toml) and [nsh.json](../nsh.json),
   update the root package version in [Cargo.lock](../Cargo.lock) by running Cargo,
   and increment the app build number in [nsh.json](../nsh.json).
2. Commit and push those changes.
3. Tag that commit and push the tag (for example, after setting version 0.1.3):

```sh
git tag v0.1.3
git push origin v0.1.3
```

The tag must exactly match both package versions. Stable tags use
`vMAJOR.MINOR.PATCH`; prerelease tags such as `v0.2.0-rc.1` create GitHub
prereleases without marking them latest. Build-metadata suffixes are not supported.
Validation runs before the build matrix, so mismatched versions fail early.

You can also select **Actions → Desktop release → Run workflow**. A branch run
only uploads workflow artifacts (retained for 30 days); selecting a matching
version tag also publishes the release. Rerunning the same tag replaces its
matching assets. New releases remain drafts until asset upload completes.
Publishing a release manually does not trigger another build; push a tag or use
the manual workflow instead. ARM Linux runners require repository access to
GitHub's hosted ARM runners (available for public repositories).

### Install and verify

- **Windows:** extract the ZIP before launching the app.
- **macOS:** open the architecture-specific DMG and drag Pomoterra into
  Applications, or extract the app bundle ZIP. Bundles are ad-hoc signed for
  integrity, **not Developer ID signed or notarized**. Gatekeeper may block them;
  follow your organization's policy for unnotarized applications. Apple Silicon
  and Intel downloads are separate, not universal binaries.
- **Linux:** install the Debian package with your distribution's package installer
  so linked-library dependencies are resolved, or extract the portable archive.
  GPUI also requires a working Vulkan driver, Wayland or X11, and libxkbcommon.
  Portable does not mean statically linked. Packages are built on Ubuntu 24.04
  and are not guaranteed to run on older distributions. GPUI releases currently
  have no close-to-tray behavior.

To verify downloads on Linux, place the release files and checksum manifest in
one directory and run:

```sh
sha256sum --check SHA256SUMS
```

Packaging checks native executable architecture (and Windows GUI subsystem),
verifies archived executable hashes, extracts and checks Debian packages, and
verifies macOS bundle signatures and disk images. These are structural checks,
not native GUI smoke tests. Windows packages are unsigned; macOS packages are
not notarized. App stores, Homebrew, Winget, APT repositories, Flatpak, mobile,
and browser releases are **not** published by this workflow; these require
separate distribution metadata, infrastructure, or signing credentials.

The [packaging helper](../.github/scripts/package.py) uses Python 3.11+ and the
standard library. Run its local tests with:

```sh
python3 -m unittest discover -s .github/scripts -p 'test_*.py' -v
python3 .github/scripts/package.py metadata --tag v0.1.3
```

Use the actual current version for the metadata check. Native package generation
also requires the platform's build tools (Debian packaging tools on Linux and
Apple command-line tools on macOS).

## Icons

### Application icon

The app uses the original [Pomoterra logo](../.github/assets/logo.svg) for its
application identity and native tray. Generated platform assets live in
[`assets/icons`](../assets/icons). Windows builds embed the multi-resolution icon
in the executable; macOS release bundles declare their icon for Finder and the
Dock. Native Linux builds embed the logo as a GTK icon resource, so the window
and X11 taskbar can use it even when running an uninstalled development binary.
Building this backend requires `glib-compile-resources` from the GLib development
tools. Linux Debian packages also install an app-ID-matched desktop entry and
hicolor icons for launchers and Wayland desktop integration. These changes apply
after rebuilding or installing a new package; desktop shells may cache an older icon.

Portable Linux archives include a `share` directory. For launcher and taskbar
integration, put the extracted executable on your PATH and copy the contents of
that directory into your user data directory (normally `.local/share` under your
home directory). A raw development binary does not install desktop integration.
macOS development binaries likewise need the packaged app bundle for the Dock icon.

After changing the source logo, regenerate and commit the derived assets with
`python3 .github/scripts/generate_icons.py`. Regeneration requires `rsvg-convert`
and ImageMagick; ordinary builds and packaging use the checked-in assets and do
not need either image tool. NSH-generated runners are separate from this project's
Rust executable and release packaging.

### UI icons

Use the [Lucide catalog](https://lucide.dev/icons/) through the pinned
[lucide-icons crate](https://docs.rs/lucide-icons/1.70.0/lucide_icons/).
All UI icons go through [`src/icons.rs`](../src/icons.rs), which reexports the
crate's typed catalog and renders decorative glyphs in a fixed square.
Choose a catalog variant and pass its color and size in logical pixels to the
shared renderer; existing examples are in [`src/lib.rs`](../src/lib.rs).
Keep the action's accessible label on the enclosing button. The renderer hides
its glyph from assistive technologies so screen readers announce the action.

To add an icon, select its matching Rust variant from the crate and use the
shared renderer. Avoid inline SVG icon paths, Unicode literals, or a second
icon library. The countdown ring remains custom SVG because it displays timer
progress rather than a catalog icon.

This crate supplies font glyphs rather than SVG paths. The renderer uses the
bundled Lucide font, with size and color as its styling options. The direct
desktop launcher registers the dependency's font bytes. [`build.rs`](../build.rs)
exposes those same bytes as an NSH font asset for generated runners, TSX entries,
and packaging; no font needs installing on the user's machine. Keep the normal
and build dependency versions identical when updating Lucide, then check icon
appearance and run the existing UI tests.

Lucide icons are ISC licensed, with Feather-derived portions under MIT; the
Rust crate is MIT licensed. Attribution and license texts are included in
[`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md). Include these notices when
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
