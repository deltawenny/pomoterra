#!/usr/bin/env python3
"""Compose real screenshots into a layered README preview using ImageMagick 7."""

import argparse
import json
import os
from collections.abc import Sequence
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
from typing import TypeAlias

ROOT: Path = Path(__file__).resolve().parents[2]

ScreenshotPath: TypeAlias = str | Path
Card: TypeAlias = tuple[float, int, int, Path]
RowLayout: TypeAlias = tuple[int, list[Card]]


def positive(value: str) -> int:
    number = int(value)
    if number < 1:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return number


def run(*args: str | Path) -> None:
    subprocess.run(["magick", *map(str, args)], check=True, capture_output=True, text=True)


def generate(
    rows: Sequence[Sequence[ScreenshotPath]],
    output: Path,
    height: int = 440,
    overlap: float = 0.42,
    padding: int = 30,
    gap: int = 45,
) -> None:
    """Input order is left to right; the middle screenshot is in front."""
    if not rows or any(not row for row in rows):
        raise ValueError("each row must contain at least one screenshot")
    if height < 1 or padding < 1 or gap < 1 or not 0 <= overlap <= 0.75:
        raise ValueError("invalid layout dimensions or overlap")
    if not shutil.which("magick"):
        raise ValueError("ImageMagick 7 is required: install it and put 'magick' on PATH")
    for row in rows:
        for source in row:
            if not Path(source).is_file():
                raise ValueError(f"screenshot not found: {source}")
    if output.suffix.lower() != ".png":
        raise ValueError("output must have a .png extension")
    if output.resolve() in {Path(p).resolve() for row in rows for p in row}:
        raise ValueError("output must not overwrite an input screenshot")

    with tempfile.TemporaryDirectory(prefix="pomoterra-preview-") as directory:
        temp = Path(directory)
        layouts: list[RowLayout] = []
        for row_index, row in enumerate(rows):
            center = (len(row) - 1) / 2
            cards: list[Card] = []
            x = 0
            row_width = 0
            for index, source in enumerate(row):
                distance = abs(index - center)
                card_height = round(height * max(0.72, 1 - 0.10 * distance))
                card = temp / f"card-{row_index}-{index}.png"
                # [0] selects one frame; auto-orient respects screenshot metadata.
                run(f"{Path(source).resolve()}[0]", "-auto-orient", "-resize", f"x{card_height}", card)
                result = subprocess.run(
                    ["magick", "identify", "-format", "%w %h", str(card)],
                    check=True, capture_output=True, text=True,
                )
                width, actual_height = map(int, result.stdout.split())
                # Round only the promotional card, leaving the real app untouched.
                radius = max(1, round(actual_height * 0.02))
                run(card, "-alpha", "set", "(", "-size", f"{width}x{actual_height}",
                    "xc:none", "-fill", "white", "-draw",
                    f"roundrectangle 0,0 {width - 1},{actual_height - 1} {radius},{radius}",
                    ")", "-compose", "DstIn", "-composite", card)
                shadow = temp / f"shadow-{row_index}-{index}.png"
                run(card, "(", "+clone", "-background", "#16354a", "-shadow", "18x12+0+12", ")",
                    "+swap", "-background", "none", "-layers", "merge", "+repage", shadow)
                cards.append((distance, x, (height - actual_height) // 2, shadow))
                row_width = x + width
                x += round(width * (1 - overlap))
            layouts.append((row_width, cards))

        canvas_width = max(width for width, _ in layouts) + padding * 2
        # Shadow blur extends beyond each card; padding keeps it inside the canvas.
        row_height = height + 60
        canvas_height = padding * 2 + len(rows) * row_height + (len(rows) - 1) * gap
        canvas = temp / "canvas.png"
        run("-size", f"{canvas_width}x{canvas_height}", "xc:none", "-type", "TrueColorMatte", f"PNG32:{canvas}")
        for row_index, (row_width, cards) in enumerate(layouts):
            left = padding + (canvas_width - padding * 2 - row_width) // 2
            top = padding + row_index * (row_height + gap)
            # Furthest cards first, middle last, producing a symmetric fan.
            for _, x, y, card in sorted(cards, key=lambda c: c[0], reverse=True):
                run(canvas, card, "-geometry", f"+{left + x}+{top + y}", "-composite", canvas)
        output.parent.mkdir(parents=True, exist_ok=True)
        run(canvas, "-strip", "-define", "png:exclude-chunks=date,time", output)


def update_readme(output: Path) -> None:
    relative = output.resolve().relative_to(ROOT).as_posix()
    readme = ROOT / "README.md"
    text = readme.read_text()
    text = text.replace('src=".github/assets/preview.svg"', f'src="{relative}"')
    text = text.replace(
        "Illustrated overview of Pomoterra's Config, Timer, and History views; not an application screenshot",
        "Pomoterra application screenshots",
    )
    text = text.replace("  <br>\n  <sub>Illustrated overview — not an application screenshot.</sub>\n", "")
    readme.write_text(text)


def capture(directory: Path) -> list[Path]:
    """Capture the native app inside the private Xvfb display, never the user's desktop."""
    environment = os.environ.copy()
    environment.update({
        "GDK_BACKEND": "x11", "GSK_RENDERER": "cairo", "GTK_A11Y": "none",
        "POMOTERRA_DB_PATH": ":memory:", "GDK_SCALE": "1", "GDK_DPI_SCALE": "1",
    })
    environment.pop("WAYLAND_DISPLAY", None)
    metadata = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"], cwd=ROOT, text=True,
    )
    binary = Path(json.loads(metadata)["target_directory"]) / "debug/nsh-app-pomoterra"
    with (directory / "app.log").open("w+") as log:
        app = subprocess.Popen([str(binary)], cwd=ROOT, env=environment, stdout=log, stderr=log)
        try:
            window = ""
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                if app.poll() is not None:
                    log.seek(0)
                    raise ValueError(f"app exited before capture: {log.read()}")
                result = subprocess.run(
                    ["xdotool", "search", "--onlyvisible", "--pid", str(app.pid), "--name", "Pomoterra"],
                    capture_output=True, text=True,
                )
                if result.returncode == 0 and result.stdout.strip():
                    window = result.stdout.splitlines()[-1]
                    break
                time.sleep(0.1)
            if not window:
                raise ValueError("timed out waiting for the Pomoterra window")
            subprocess.run(["xdotool", "windowsize", "--sync", window, "400", "650"], check=True)
            time.sleep(1)
            images: list[Path] = []
            for name, x in (("config", 115), ("timer", 200), ("history", 285)):
                subprocess.run(
                    ["xdotool", "mousemove", "--window", window, str(x), "28", "click", "1"], check=True,
                )
                time.sleep(0.5)
                if name == "history":
                    # The built-in sample toggle is below the History heading.
                    subprocess.run(
                        ["xdotool", "mousemove", "--window", window, "85", "130", "click", "1"], check=True,
                    )
                    time.sleep(0.5)
                image = directory / f"{name}.png"
                subprocess.run(["magick", "import", "-window", window, str(image)], check=True)
                images.append(image)
            return images
        finally:
            app.terminate()
            try:
                app.wait(timeout=5)
            except subprocess.TimeoutExpired:
                app.kill()
                app.wait()


def automatic_preview() -> None:
    """Build and re-enter in an isolated display/session for automatic capture."""
    if not sys.platform.startswith("linux"):
        raise ValueError("automatic capture currently supports Linux; use --row for existing screenshots elsewhere")
    tools = ("cargo", "magick", "xvfb-run", "Xvfb", "xauth", "xdotool", "dbus-run-session")
    missing = [tool for tool in tools if not shutil.which(tool)]
    if missing:
        raise ValueError(f"install required capture tools: {', '.join(missing)}")
    print("Building Pomoterra…", flush=True)
    subprocess.run(["cargo", "build", "--locked", "--no-default-features", "--features", "native"], cwd=ROOT, check=True)
    subprocess.run([
        "xvfb-run", "--auto-servernum", "--server-args=-screen 0 1280x1024x24 -nolisten tcp",
        "dbus-run-session", "--", sys.executable, str(Path(__file__).resolve()), "--capture-worker",
    ], cwd=ROOT, check=True)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--capture-worker", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--row", nargs="+", action="append", type=Path,
                        help="screenshots in left-to-right order; repeat for another row")
    parser.add_argument("--output", type=Path, default=ROOT / ".github/assets/preview.png")
    parser.add_argument("--height", type=positive, default=440, help="front screenshot height (default: 440)")
    parser.add_argument("--overlap", type=float, default=0.42, help="overlap fraction, 0–0.75 (default: 0.42)")
    parser.add_argument("--padding", type=positive, default=30)
    parser.add_argument("--gap", type=positive, default=45, help="space between rows")
    parser.add_argument("--update-readme", action="store_true",
                        help="replace the README illustration with the generated screenshot preview")
    args = parser.parse_args(argv)
    try:
        if args.capture_worker:
            output = ROOT / ".github/assets/preview.png"
            with tempfile.TemporaryDirectory(prefix="pomoterra-capture-") as directory:
                images = capture(Path(directory))
                generate([images], output)
            update_readme(output)
            print(f"Updated {output} and README.md")
            return 0
        if not args.row:
            automatic_preview()
            return 0
        if args.update_readme:
            args.output.resolve().relative_to(ROOT)
        generate(args.row, args.output, args.height, args.overlap, args.padding, args.gap)
        if args.update_readme:
            update_readme(args.output)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        detail = error.stderr if isinstance(error, subprocess.CalledProcessError) else str(error)
        parser.exit(1, f"Preview generation failed: {detail}\n")
    print(f"Generated {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
