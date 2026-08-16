from __future__ import annotations

import struct
import sys
import unittest
from pathlib import Path
from unittest import mock


RELEASE_SCRIPTS = Path(__file__).resolve().parents[1] / "release"
sys.path.insert(0, str(RELEASE_SCRIPTS))

import package_native  # noqa: E402


class PackageNativeTests(unittest.TestCase):
    def test_release_tag_is_normalized(self) -> None:
        self.assertEqual(package_native.validate_version("v2.0.0"), "2.0.0")
        self.assertEqual(
            package_native.validate_version("2.0.0-preview.1"),
            "2.0.0-preview.1",
        )

    def test_invalid_or_mismatched_version_is_rejected(self) -> None:
        with self.assertRaises(ValueError):
            package_native.validate_version("release-2")
        with self.assertRaisesRegex(ValueError, "does not match"):
            package_native.validate_version("v2.0.1", "2.0.0")

    def test_all_release_targets_have_expected_artifact_names(self) -> None:
        expected = {
            "aarch64-apple-darwin": {
                "NyaTerm_2.0.0_macos_arm64.dmg",
                "NyaTerm_2.0.0_macos_arm64.app.tar.gz",
            },
            "x86_64-apple-darwin": {
                "NyaTerm_2.0.0_macos_x64.dmg",
                "NyaTerm_2.0.0_macos_x64.app.tar.gz",
            },
            "aarch64-unknown-linux-gnu": {
                "NyaTerm_2.0.0_linux_arm64.AppImage",
                "NyaTerm_2.0.0_linux_arm64.deb",
                "NyaTerm_2.0.0_linux_arm64.rpm",
            },
            "x86_64-unknown-linux-gnu": {
                "NyaTerm_2.0.0_linux_x64.AppImage",
                "NyaTerm_2.0.0_linux_x64.deb",
                "NyaTerm_2.0.0_linux_x64.rpm",
            },
            "aarch64-pc-windows-msvc": {
                "NyaTerm_2.0.0_windows_arm64_portable.zip",
                "NyaTerm_2.0.0_windows_arm64-setup.exe",
            },
            "x86_64-pc-windows-msvc": {
                "NyaTerm_2.0.0_windows_x64_portable.zip",
                "NyaTerm_2.0.0_windows_x64-setup.exe",
            },
        }
        for target, names in expected.items():
            with self.subTest(target=target):
                self.assertEqual(package_native.artifact_names(target, "v2.0.0"), names)

    def test_release_binary_always_uses_explicit_target_directory(self) -> None:
        with mock.patch.dict("os.environ", {}, clear=True):
            linux = package_native.release_binary_path("x86_64-unknown-linux-gnu")
            windows = package_native.release_binary_path("aarch64-pc-windows-msvc")
        self.assertEqual(
            linux.relative_to(package_native.ROOT_DIR).as_posix(),
            "target/x86_64-unknown-linux-gnu/release/nyaterm",
        )
        self.assertEqual(
            windows.relative_to(package_native.ROOT_DIR).as_posix(),
            "target/aarch64-pc-windows-msvc/release/nyaterm.exe",
        )

    def test_release_binary_respects_absolute_cargo_target_dir(self) -> None:
        with mock.patch.dict("os.environ", {"CARGO_TARGET_DIR": "/cache/cargo"}):
            path = package_native.release_binary_path("x86_64-unknown-linux-gnu")
        self.assertEqual(
            path.as_posix(), "/cache/cargo/x86_64-unknown-linux-gnu/release/nyaterm"
        )

    def test_platform_package_versions_are_normalized(self) -> None:
        self.assertEqual(package_native.windows_numeric_version("2.4.6-beta.1"), "2.4.6.0")
        self.assertEqual(package_native.linux_rpm_version("2.4.6"), ("2.4.6", "1"))
        self.assertEqual(
            package_native.linux_rpm_version("2.4.6-beta.1"),
            ("2.4.6", "0.beta.1"),
        )

    def test_dpkg_dependency_output_is_parsed(self) -> None:
        output = "ignored=value\nshlibs:Depends=libc6 (>= 2.34), libx11-6\n"
        self.assertEqual(
            package_native.parse_dpkg_dependencies(output),
            "libc6 (>= 2.34), libx11-6",
        )
        with self.assertRaises(RuntimeError):
            package_native.parse_dpkg_dependencies("shlibs:Depends=\n")

    def test_native_icon_resources_have_expected_formats_and_sizes(self) -> None:
        expected_png_sizes = {
            "32x32.png": (32, 32),
            "64x64.png": (64, 64),
            "128x128.png": (128, 128),
            "256x256.png": (256, 256),
            "512x512.png": (512, 512),
        }
        for name, expected_size in expected_png_sizes.items():
            with self.subTest(name=name):
                data = (package_native.ICON_DIR / name).read_bytes()
                self.assertEqual(data[:8], b"\x89PNG\r\n\x1a\n")
                self.assertEqual(struct.unpack(">II", data[16:24]), expected_size)

        self.assertEqual(
            (package_native.ICON_DIR / "icon.icns").read_bytes()[:4], b"icns"
        )
        self.assertEqual(
            (package_native.ICON_DIR / "icon.ico").read_bytes()[:4], b"\0\0\1\0"
        )


if __name__ == "__main__":
    unittest.main()
