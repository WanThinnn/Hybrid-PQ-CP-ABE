"""Synthetic cache regressions; run during final security acceptance."""

import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / 'scripts/native-dependency-cache.py'
spec = importlib.util.spec_from_file_location('native_cache', SCRIPT)
cache = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cache)


class NativeDependencyCacheTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.base = self.root / 'build/deps/linux'
        self.patch_root = patch.object(cache, 'ROOT', self.root)
        self.patch_root.start()
        self.addCleanup(self.patch_root.stop)

    def write(self, name, data=b'abc'):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return path

    def invoke(self, action, name='rabe', context='input-v1'):
        with patch('sys.argv', [str(SCRIPT), action, 'linux', name, context]):
            cache.main()

    def stage_rabe(self):
        for name in ['lib/rabe/librabe_ffi.a', 'lib/rabe/librabe_ffi.so',
                     'include/rabe/rabe.h']:
            self.write('build/deps/linux/' + name)

    def test_cache_hit_preserves_record_timestamp(self):
        self.stage_rabe()
        self.invoke('record')
        path = cache.record_path(self.base, 'rabe')
        before = path.stat().st_mtime_ns
        self.invoke('check')
        self.invoke('record')
        self.assertEqual(before, path.stat().st_mtime_ns)

    def test_changed_input_rejects_cache_and_ready_marker(self):
        self.stage_rabe()
        self.invoke('record')
        ready = self.write('build/deps/linux/lib/.ready')
        with self.assertRaises(SystemExit):
            self.invoke('check', context='input-v2')
        self.assertFalse(ready.exists())

    def test_missing_corrupt_and_added_outputs_reject_cache(self):
        for change in ['missing', 'corrupt', 'added']:
            with self.subTest(change=change):
                self.stage_rabe()
                self.invoke('record')
                library = self.base / 'lib/rabe/librabe_ffi.a'
                if change == 'missing':
                    library.unlink()
                elif change == 'corrupt':
                    library.write_bytes(b'xyz')  # Same size, changed content.
                else:
                    self.write('build/deps/linux/lib/rabe/unexpected.so')
                with self.assertRaises(SystemExit):
                    self.invoke('check')

    def test_incomplete_outputs_cannot_be_recorded_or_marked_ready(self):
        self.write('build/deps/linux/lib/rabe/librabe_ffi.a')
        with self.assertRaises(FileNotFoundError):
            self.invoke('record')
        ready = self.write('build/deps/linux/lib/.ready')
        with self.assertRaises(FileNotFoundError):
            self.invoke('ready', name='all')
        self.assertFalse(ready.exists())

    def test_pqc_underskrift_ffi_static_shared_header_and_tampering(self):
        archive = self.write('build/deps/linux/lib/pqc-underskrift/libpdf_signature_ffi.a')
        self.write('build/deps/linux/lib/pqc-underskrift/libpdf_signature_ffi.so')
        self.write('build/deps/linux/include/pqc-underskrift/pdf_signature_ffi.h')
        self.invoke('record', name='pqc-underskrift')
        self.invoke('check', name='pqc-underskrift')
        archive.write_bytes(b'modified archive')
        with self.assertRaises(SystemExit):
            self.invoke('check', name='pqc-underskrift')

    def test_ready_requires_all_six_dependencies_and_keeps_timestamp(self):
        libraries = {'cryptopp': 'libcryptopp', 'liboqs': 'liboqs',
                     'rabe': 'librabe_ffi', 'tkn20': 'libcpabe_tkn20',
                     'covercrypt': 'libcovercrypt_ffi'}
        headers = {'cryptopp': 'aes.h', 'liboqs': 'oqs/oqs.h', 'rabe': 'rabe.h',
                   'tkn20': 'cpabe_tkn20.h', 'covercrypt': 'covercrypt_ffi.h'}
        for name, library in libraries.items():
            for extension in ('a', 'so'):
                self.write(f'build/deps/linux/lib/{name}/{library}.{extension}')
            self.write(f'build/deps/linux/include/{name}/{headers[name]}')
            self.invoke('record', name=name)
        self.write('build/deps/linux/lib/pqc-underskrift/libpdf_signature_ffi.a')
        self.write('build/deps/linux/lib/pqc-underskrift/libpdf_signature_ffi.so')
        self.write('build/deps/linux/include/pqc-underskrift/pdf_signature_ffi.h')
        self.invoke('record', name='pqc-underskrift')
        self.invoke('ready', name='all')
        ready = self.base / 'lib/.ready'
        before = ready.stat().st_mtime_ns
        self.invoke('ready', name='all')
        self.assertEqual(before, ready.stat().st_mtime_ns)
        self.assertEqual(len(json.loads(ready.read_text())['dependencies']), 6)
        (self.base / 'lib/tkn20/libcpabe_tkn20.so').unlink()
        with self.assertRaises(FileNotFoundError):
            self.invoke('ready', name='all')
        self.assertFalse(ready.exists())

    def test_content_flags_toolchain_and_rename_invalidate_fingerprint(self):
        source = self.write('vendor/librabe-ffi/src/lib.rs')
        self.write('vendor/upstreams.json')
        for name in ['build-deps.sh', 'build-deps.ps1', 'native-dependency-cache.py',
                     'linux-hardening.py',
                     'cryptopp-msvc.props', 'cryptopp-dll.props', 'normalize-msvc-library.ps1']:
            self.write('scripts/' + name)
        identity = ['compiler-v1']
        go_env = json.dumps({'CC': 'cc', 'CXX': 'c++'})

        def fingerprint():
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                self.invoke('fingerprint')
            return output.getvalue()

        with patch.object(cache, 'tool', side_effect=lambda command: [command, *identity]), \
                patch.object(cache.subprocess, 'check_output', return_value=go_env), \
                patch.dict(os.environ, {'CARGO_HOME': str(self.root / '.cargo')}, clear=True):
            original = fingerprint()
            self.assertEqual(original, fingerprint())
            self.write('vendor/librabe-ffi/adhoc.cpp.copied', b'generated marker')
            self.assertEqual(original, fingerprint())
            old_stat = source.stat()
            source.write_bytes(b'xyz')
            os.utime(source, ns=(old_stat.st_atime_ns, old_stat.st_mtime_ns))
            self.assertNotEqual(original, fingerprint())
            source.write_bytes(b'abc')
            self.assertEqual(original, fingerprint())
            with patch.dict(os.environ, {'RUSTFLAGS': '-C opt-level=2'}):
                self.assertNotEqual(original, fingerprint())
            identity[:] = ['compiler-v2']
            self.assertNotEqual(original, fingerprint())
            identity[:] = ['compiler-v1']
            source.rename(source.with_name('renamed.rs'))
            self.assertNotEqual(original, fingerprint())


if __name__ == '__main__':
    unittest.main()
