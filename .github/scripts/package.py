#!/usr/bin/env python3
"""Version validation and native desktop packaging; uses Python's standard library."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import struct
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
SEMVER = re.compile(
    r"^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)"
    r"(?:-((?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
    r"(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*))?$"
)


def metadata(root, tag=None):
    cargo = tomllib.loads((root / "Cargo.toml").read_text())
    app = json.loads((root / "nsh.json").read_text())
    version = cargo["package"]["version"]
    match = SEMVER.fullmatch(version)
    if not match:
        raise ValueError("Package version must be SemVer without build metadata")
    if app["version"] != version:
        raise ValueError("Cargo.toml and nsh.json versions must match")
    if tag is not None and tag != f"v{version}":
        raise ValueError(f"Release tag must match the package version: v{version}")
    return {"version": version, "prerelease": str(bool(match[4])).lower(), "id": app["id"]}


def verify_binary(path, platform, arch):
    data = path.read_bytes()
    if platform == "windows":
        if len(data) < 64 or data[:2] != b"MZ":
            raise ValueError("Missing MZ header")
        pe = struct.unpack_from("<I", data, 0x3C)[0]
        if pe < 64 or pe + 94 > len(data) or data[pe:pe + 4] != b"PE\0\0":
            raise ValueError("Invalid PE header")
        if (struct.unpack_from("<H", data, pe + 4)[0] != 0x8664
                or struct.unpack_from("<H", data, pe + 24)[0] != 0x20B
                or struct.unpack_from("<H", data, pe + 92)[0] != 2):
            raise ValueError("Expected an x64 PE32+ Windows GUI executable")
    elif platform == "linux":
        machine = 62 if arch == "x64" else 183
        if (len(data) < 64 or data[:6] != b"\x7fELF\x02\x01"
                or struct.unpack_from("<H", data, 18)[0] != machine):
            raise ValueError(f"Expected a 64-bit little-endian ELF executable for {arch}")
    elif platform == "macos":
        cpu = 0x01000007 if arch == "x64" else 0x0100000C
        if (len(data) < 32 or data[:4] != b"\xcf\xfa\xed\xfe"
                or struct.unpack_from("<I", data, 4)[0] != cpu):
            raise ValueError(f"Expected a 64-bit Mach-O executable for {arch}")
    else:
        raise ValueError(f"Unsupported platform: {platform}")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def make_zip(directory, output, executable):
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(directory.rglob("*")):
            if path.is_file():
                archive.write(path, path.relative_to(directory.parent))
    with zipfile.ZipFile(output) as archive:
        archived = archive.read(str(executable.relative_to(directory.parent)).replace(os.sep, "/"))
    if hashlib.sha256(archived).hexdigest() != digest(executable):
        raise ValueError("Archived executable differs from the staged executable")


def make_tar(directory, output, executable):
    with tarfile.open(output, "w:gz") as archive:
        archive.add(directory, arcname=directory.name)
    with tarfile.open(output) as archive:
        archived = archive.extractfile(executable.relative_to(directory.parent).as_posix()).read()
    if hashlib.sha256(archived).hexdigest() != digest(executable):
        raise ValueError("Archived executable differs from the staged executable")


def make_deb(root, work, source, output, version, arch):
    package = work / "deb"
    binary = package / "usr/bin/pomoterra"
    binary.parent.mkdir(parents=True)
    shutil.copy2(source, binary)
    binary.chmod(0o755)
    docs = package / "usr/share/doc/pomoterra"
    docs.mkdir(parents=True)
    shutil.copy2(root / "THIRD_PARTY_NOTICES.md", docs)
    desktop = package / "usr/share/applications/pomoterra.desktop"
    desktop.parent.mkdir(parents=True)
    desktop.write_text(
        "[Desktop Entry]\nType=Application\nName=Pomoterra\n"
        "Comment=Pomodoro focus timer\nExec=pomoterra\nTerminal=false\n"
        "Categories=Utility;Clock;\nStartupWMClass=com.deltawenny.pomoterra\n"
    )
    # Let the native runner resolve exact shared-library package requirements.
    debian = work / "debian"
    debian.mkdir()
    (debian / "control").write_text(
        "Source: pomoterra\nSection: utils\nPriority: optional\n"
        "Maintainer: Pomoterra contributors <noreply@users.noreply.github.com>\n\n"
        "Package: pomoterra\nArchitecture: any\nDescription: Pomodoro focus timer\n"
    )
    result = subprocess.check_output(
        ["dpkg-shlibdeps", "-O", "-e", str(binary)], cwd=work, text=True
    )
    dependencies = next(line.split("=", 1)[1] for line in result.splitlines()
                        if line.startswith("shlibs:Depends="))
    control = package / "DEBIAN"
    control.mkdir()
    # Debian orders ~ prereleases before their corresponding stable version.
    deb_version = version.replace("-", "~", 1)
    (control / "control").write_text(
        f"Package: pomoterra\nVersion: {deb_version}\n"
        f"Architecture: {'amd64' if arch == 'x64' else 'arm64'}\n"
        "Maintainer: Pomoterra contributors <noreply@users.noreply.github.com>\n"
        f"Depends: {dependencies}\nSection: utils\nPriority: optional\n"
        "Description: Pomodoro focus timer with profiles and session history\n"
    )
    subprocess.run(["dpkg-deb", "--root-owner-group", "--build", str(package), str(output)], check=True)
    extracted = work / "verify-deb"
    subprocess.run(["dpkg-deb", "--extract", str(output), str(extracted)], check=True)
    if digest(extracted / "usr/bin/pomoterra") != digest(source):
        raise ValueError("Debian package executable differs from the build output")


def package(root, target, platform, arch):
    info = metadata(root)
    version = info["version"]
    name = f"pomoterra-{version}-{platform}-{arch}"
    source = root / "target" / target / "release" / (
        "nsh-app-pomoterra.exe" if platform == "windows" else "nsh-app-pomoterra"
    )
    verify_binary(source, platform, arch)
    dist = root / "dist"
    dist.mkdir(exist_ok=True)
    outputs = []
    with tempfile.TemporaryDirectory() as temporary:
        work = Path(temporary)
        stage = work / name
        stage.mkdir()
        shutil.copy2(root / "THIRD_PARTY_NOTICES.md", stage)
        if platform == "macos":
            app = stage / "Pomoterra.app"
            contents = app / "Contents"
            (contents / "MacOS").mkdir(parents=True)
            (contents / "Resources").mkdir()
            executable = contents / "MacOS/pomoterra"
            shutil.copy2(source, executable)
            executable.chmod(0o755)
            shutil.copy2(root / "THIRD_PARTY_NOTICES.md", contents / "Resources")
            with (contents / "Info.plist").open("wb") as handle:
                plistlib.dump({
                    "CFBundleName": "Pomoterra", "CFBundleDisplayName": "Pomoterra",
                    "CFBundleIdentifier": info["id"], "CFBundleExecutable": "pomoterra",
                    "CFBundlePackageType": "APPL", "CFBundleShortVersionString": version.split("-")[0],
                    "CFBundleVersion": str(json.loads((root / "nsh.json").read_text())["build"]),
                    "LSMinimumSystemVersion": "11.0", "NSHighResolutionCapable": True,
                }, handle)
            subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
            subprocess.run(["codesign", "--verify", "--strict", str(app)], check=True)
            archive = dist / f"{name}.zip"
            make_zip(stage, archive, executable)
            outputs.append(archive)
            (stage / "Applications").symlink_to("/Applications")
            image = dist / f"{name}.dmg"
            subprocess.run(["hdiutil", "create", "-volname", "Pomoterra", "-srcfolder",
                            str(stage), "-ov", "-format", "UDZO", str(image)], check=True)
            subprocess.run(["hdiutil", "verify", str(image)], check=True)
            outputs.append(image)
        else:
            executable = stage / ("pomoterra.exe" if platform == "windows" else "pomoterra")
            shutil.copy2(source, executable)
            if digest(source) != digest(executable):
                raise ValueError("Staged executable differs from the build output")
            if platform == "windows":
                archive = dist / f"{name}.zip"
                make_zip(stage, archive, executable)
            else:
                executable.chmod(0o755)
                archive = dist / f"{name}.tar.gz"
                make_tar(stage, archive, executable)
                deb = dist / f"{name}.deb"
                make_deb(root, work, source, deb, version, arch)
                outputs.append(deb)
            outputs.append(archive)
        for path in outputs:
            (dist / f"{path.name}.sha256").write_text(f"{digest(path)}  {path.name}\n")
            print(f"Packaged {path.name}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    meta = commands.add_parser("metadata")
    meta.add_argument("--tag")
    pack = commands.add_parser("package")
    pack.add_argument("--target", required=True)
    pack.add_argument("--platform", choices=["linux", "windows", "macos"], required=True)
    pack.add_argument("--arch", choices=["x64", "arm64"], required=True)
    args = parser.parse_args()
    if args.command == "metadata":
        for key, value in metadata(ROOT, args.tag).items():
            print(f"{key}={value}")
    else:
        package(ROOT, args.target, args.platform, args.arch)


if __name__ == "__main__":
    main()
