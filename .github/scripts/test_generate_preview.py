"""Tests use synthetic temporary fixtures, never promotional screenshots."""

from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import generate_preview as preview


class PreviewTests(unittest.TestCase):
    def test_missing_imagemagick(self) -> None:
        with patch.object(shutil, "which", return_value=None):
            with self.assertRaisesRegex(ValueError, "ImageMagick 7"):
                preview.generate([[Path("missing.png")]], Path("out.png"))

    def test_invalid_layout(self) -> None:
        cases: tuple[tuple[list[list[preview.ScreenshotPath]], float], ...] = (
            ([], 0.4), ([[]], 0.4), ([["unused"]], 0.9),
        )
        for rows, overlap in cases:
            with self.subTest(rows=rows, overlap=overlap), self.assertRaises(ValueError):
                preview.generate(rows, Path("out.png"), overlap=overlap)

    @unittest.skipUnless(shutil.which("magick"), "ImageMagick 7 required")
    def test_composition(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            images: list[Path] = []
            for index, color in enumerate(("red", "green", "blue")):
                image = root / f"fixture {index}.png"
                preview.run("-size", "100x160", f"xc:{color}", image)
                images.append(image)
            original = [p.read_bytes() for p in images]
            for rows in ([images], [images, images[:1]]):
                output = root / "result.png"
                preview.generate(rows, output, height=160)
                result = subprocess.check_output(
                    ["magick", "identify", "-format", "%m %w %h", str(output)], text=True,
                ).split()
                self.assertEqual(result[0], "PNG")
                self.assertGreater(int(result[1]), 200)
                self.assertEqual(int(result[2]), 60 + len(rows) * 220 + (len(rows) - 1) * 45)
            self.assertEqual(original, [p.read_bytes() for p in images])
            with self.assertRaisesRegex(ValueError, "overwrite"):
                preview.generate([images], images[0])
            with self.assertRaisesRegex(ValueError, "not found"):
                preview.generate([[root / "missing.png"]], root / "out.png")
            with self.assertRaisesRegex(ValueError, ".png extension"):
                preview.generate([images], root / "out.jpg")

    def test_default_command_runs_automatic_workflow(self) -> None:
        with patch.object(preview, "automatic_preview") as automatic:
            self.assertEqual(preview.main([]), 0)
        automatic.assert_called_once_with()

    def test_automatic_workflow_checks_tools_before_build(self) -> None:
        with patch.object(preview.sys, "platform", "linux"), patch.object(shutil, "which", return_value=None):
            with self.assertRaisesRegex(ValueError, "install required capture tools"):
                preview.automatic_preview()

    def test_automatic_workflow_builds_then_uses_private_display(self) -> None:
        with patch.object(preview.sys, "platform", "linux"), \
             patch.object(shutil, "which", return_value="tool"), \
             patch.object(subprocess, "run") as run:
            preview.automatic_preview()
        self.assertEqual(run.call_count, 2)
        self.assertEqual(run.call_args_list[0].args[0][:2], ["cargo", "build"])
        self.assertEqual(run.call_args_list[1].args[0][0], "xvfb-run")
        self.assertIn("--capture-worker", run.call_args_list[1].args[0])

    def test_update_readme(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            readme = root / "README.md"
            readme.write_text(
                '<img src=".github/assets/preview.svg" alt="Illustrated overview of Pomoterra\'s Config, Timer, and History views; not an application screenshot">\n'
                '  <br>\n  <sub>Illustrated overview — not an application screenshot.</sub>\n'
            )
            with patch.object(preview, "ROOT", root):
                preview.update_readme(root / ".github/assets/preview.png")
                first = readme.read_text()
                preview.update_readme(root / ".github/assets/preview.png")
            self.assertEqual(first, readme.read_text())
            self.assertIn('src=".github/assets/preview.png"', first)
            self.assertIn("Pomoterra application screenshots", first)
            self.assertNotIn("<sub>", first)


if __name__ == "__main__":
    unittest.main()
