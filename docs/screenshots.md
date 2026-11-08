# Refresh the README preview

From the repository root, run:

```sh
python3 .github/scripts/generate_preview.py
```

That's it. The command builds the current app, launches it in a private virtual display, captures Config, Timer, and History, composes the layered preview, and updates the image referenced by the README.

The generated image is `.github/assets/preview.png`. Commit it together with the README change.

## Requirements

Automatic capture currently supports Linux and requires:

- The project's Rust toolchain and native GTK development dependencies.
- Python 3.10 or newer, without extra Python packages.
- ImageMagick 7, Xvfb, xauth, xdotool, and dbus-run-session on PATH.

On Ubuntu, the capture utilities can be installed with:

```sh
sudo apt-get install xvfb xauth xdotool dbus-x11
```

Install [ImageMagick 7](https://imagemagick.org/script/download.php) separately if your distribution only supplies ImageMagick 6.

## What gets captured

The script starts a separate native-backend build at a fixed window size in Xvfb, even if your normal desktop uses Wayland. It doesn't capture or click your normal desktop. It uses an in-memory database, so personal profiles, notes, and saved sessions are not read or changed.

The History capture enables the app's built-in **sample history** preview. The screenshot retains its sample-data notice. These are real rendered app screenshots, not drawn mockups. The app process, virtual display, D-Bus session, and temporary captures are cleaned up afterward.

Capture uses fixed tab/button coordinates for the current layout; update them in the script if the navigation layout changes. It waits for the window to appear and allows rendering time before each capture.

## Optional manual composition

For screenshots from another operating system, or a different layout, the advanced options remain available:

```sh
python3 .github/scripts/generate_preview.py \
  --row dist/screenshots/config.png dist/screenshots/timer.png dist/screenshots/history.png \
  --update-readme
```

Repeat the row option for multiple rows. The middle screenshot is largest and in front. Inputs are never overwritten. See all options with:

```sh
python3 .github/scripts/generate_preview.py --help
```

## Tests

```sh
python3 -m unittest discover -s .github/scripts -p 'test_*.py' -v
```

Composition tests use temporary colored fixtures, not promotional assets. Workflow tests mock launching and capture; run the no-argument command to verify native rendering end to end.
