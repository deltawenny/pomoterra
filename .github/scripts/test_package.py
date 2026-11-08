"""Fast packaging tests without native build tools or GUI execution."""

import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("packaging", Path(__file__).with_name("package.py"))
packaging = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(packaging)


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def write_metadata(self, version="1.2.3", app_version=None):
        (self.root / "Cargo.toml").write_text(f'[package]\nversion = "{version}"\n')
        (self.root / "nsh.json").write_text(json.dumps({
            "version": app_version or version, "id": "com.deltawenny.pomoterra",
        }))

    def test_stable_and_prerelease_metadata(self):
        for version, prerelease in [("1.2.3", "false"), ("1.2.3-rc.1", "true")]:
            with self.subTest(version=version):
                self.write_metadata(version)
                result = packaging.metadata(self.root, f"v{version}")
                self.assertEqual(result["prerelease"], prerelease)

    def test_version_and_tag_mismatches_fail(self):
        self.write_metadata(app_version="1.2.4")
        with self.assertRaises(ValueError):
            packaging.metadata(self.root)
        self.write_metadata()
        for tag in ["1.2.3", "v1.2.4", "v1.2.3;echo unsafe"]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                packaging.metadata(self.root, tag)

    def test_invalid_versions_fail(self):
        for version in ["01.2.3", "1.2", "1.2.3-01", "1.2.3+build", "1.2.3-"]:
            self.write_metadata(version)
            with self.subTest(version=version), self.assertRaises(ValueError):
                packaging.metadata(self.root)

    def test_native_headers_and_wrong_architectures(self):
        binary = self.root / "binary"
        for platform, arch, machine in [
            ("linux", "x64", 62), ("linux", "arm64", 183),
            ("macos", "x64", 0x01000007), ("macos", "arm64", 0x0100000C),
        ]:
            data = bytearray(64)
            if platform == "linux":
                data[:6] = b"\x7fELF\x02\x01"
                struct.pack_into("<H", data, 18, machine)
            else:
                data[:4] = b"\xcf\xfa\xed\xfe"
                struct.pack_into("<I", data, 4, machine)
            binary.write_bytes(data)
            with self.subTest(platform=platform, arch=arch):
                packaging.verify_binary(binary, platform, arch)
                with self.assertRaises(ValueError):
                    packaging.verify_binary(binary, platform, "arm64" if arch == "x64" else "x64")

    def test_windows_gui_header_validation(self):
        binary = self.root / "pomoterra.exe"
        data = bytearray(256)
        data[:2] = b"MZ"
        struct.pack_into("<I", data, 0x3C, 64)
        data[64:68] = b"PE\0\0"
        struct.pack_into("<H", data, 68, 0x8664)
        struct.pack_into("<H", data, 88, 0x20B)
        struct.pack_into("<H", data, 156, 2)
        binary.write_bytes(data)
        packaging.verify_binary(binary, "windows", "x64")
        struct.pack_into("<H", data, 156, 3)
        binary.write_bytes(data)
        with self.assertRaises(ValueError):
            packaging.verify_binary(binary, "windows", "x64")
        for invalid in [b"", b"MZ", b"not executable"]:
            binary.write_bytes(invalid)
            with self.assertRaises(ValueError):
                packaging.verify_binary(binary, "windows", "x64")

    def test_linux_desktop_icon_matches_application_id(self):
        self.write_metadata()
        icons = self.root / "assets/icons"
        icons.mkdir(parents=True)
        (icons / "pomoterra.png").write_bytes(b"png")
        logo = self.root / ".github/assets/logo.svg"
        logo.parent.mkdir(parents=True)
        logo.write_text("<svg/>")
        destination = self.root / "share"
        packaging.install_linux_desktop(self.root, destination)
        app_id = "com.deltawenny.pomoterra"
        desktop = (destination / f"applications/{app_id}.desktop").read_text()
        self.assertIn(f"Icon={app_id}\n", desktop)
        self.assertIn(f"StartupWMClass={app_id}\n", desktop)
        self.assertEqual(
            (destination / f"icons/hicolor/scalable/apps/{app_id}.svg").read_text(),
            "<svg/>",
        )
        self.assertEqual(
            (destination / f"icons/hicolor/256x256/apps/{app_id}.png").read_bytes(), b"png",
        )

    def test_checked_in_icon_formats(self):
        icons = packaging.ROOT / "assets/icons"
        png = (icons / "pomoterra.png").read_bytes()
        self.assertEqual(png[:8], b"\x89PNG\r\n\x1a\n")
        self.assertEqual(struct.unpack_from(">II", png, 16), (256, 256))
        self.assertEqual(len((icons / "pomoterra.rgba").read_bytes()), 64 * 64 * 4)
        ico = (icons / "pomoterra.ico").read_bytes()
        self.assertEqual(struct.unpack_from("<HHH", ico), (0, 1, 7))
        icns = (icons / "pomoterra.icns").read_bytes()
        self.assertEqual(icns[:4], b"icns")
        self.assertEqual(struct.unpack_from(">I", icns, 4)[0], len(icns))
        offset = 8
        sizes = []
        while offset < len(icns):
            size = struct.unpack_from(">I", icns, offset + 4)[0]
            self.assertGreater(size, 8)
            self.assertEqual(icns[offset + 8:offset + 16], b"\x89PNG\r\n\x1a\n")
            sizes.append(struct.unpack_from(">II", icns, offset + 24))
            offset += size
        self.assertEqual(offset, len(icns))
        self.assertEqual(sizes, [(s, s) for s in (16, 32, 64, 128, 256, 512, 1024)])

    def test_archives_preserve_binary(self):
        stage = self.root / "pomoterra-test"
        stage.mkdir()
        binary = stage / "pomoterra"
        binary.write_bytes(b"test executable\x00\xff")
        binary.chmod(0o755)
        (stage / "THIRD_PARTY_NOTICES.md").write_text("License notices")
        packaging.make_zip(stage, self.root / "test.zip", binary)
        packaging.make_tar(stage, self.root / "test.tar.gz", binary)


if __name__ == "__main__":
    unittest.main()
