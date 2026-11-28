"""Regression checks for the internal FFI module layout."""
from pathlib import Path
import re
import unittest

ROOT = Path(__file__).resolve().parents[1]
FFI = ROOT / "bindings" / "ffi"


class FfiLayoutTests(unittest.TestCase):
    def test_internal_ffi_modules_live_under_one_parent(self):
        for name in ("covercrypt", "tkn20", "pdf-signature"):
            self.assertTrue((FFI / name).is_dir(), name)
            self.assertFalse((ROOT / "bindings" / name).exists(), name)
        self.assertTrue((FFI / "README.md").is_file())
        for name in ("python", "wasm", "npm"):
            self.assertTrue((ROOT / "bindings" / name).is_dir(), name)

    def test_rust_crates_resolve_vendored_dependencies(self):
        for crate, vendor in (
            ("covercrypt", "cover_crypt"),
            ("pdf-signature", "pqc-underskrift"),
        ):
            manifest = (FFI / crate / "Cargo.toml").read_text(encoding="utf-8")
            match = re.search(r'path\s*=\s*"([^"]*vendor/[^"]+)"', manifest)
            self.assertIsNotNone(match, crate)
            self.assertEqual((FFI / crate / match.group(1)).resolve(),
                             (ROOT / "vendor" / vendor).resolve())

    def test_go_circl_replace_path_resolves_to_vendor(self):
        go_mod = (FFI / "tkn20" / "go.mod").read_text(encoding="utf-8")
        match = re.search(
            r"replace\s+github\.com/cloudflare/circl\s*=>\s*(\S+)", go_mod
        )
        self.assertIsNotNone(match)
        self.assertEqual((FFI / "tkn20" / match.group(1)).resolve(),
                         (ROOT / "vendor" / "circl").resolve())


if __name__ == "__main__":
    unittest.main()
