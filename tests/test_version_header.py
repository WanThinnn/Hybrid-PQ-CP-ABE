"""Changing VERSION regenerates the public SDK header without a clean build."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
MAKE = shutil.which('make')


@unittest.skipUnless(MAKE, 'GNU Make required')
class VersionHeaderTests(unittest.TestCase):
    def test_version_changes_regenerate_the_header(self):
        with tempfile.TemporaryDirectory(prefix='version header ') as directory:
            root = Path(directory)
            shutil.copy2(ROOT / 'Makefile', root / 'Makefile')
            (root / 'scripts').mkdir()
            shutil.copy2(ROOT / 'scripts/windows-shell.sh', root / 'scripts/windows-shell.sh')
            shutil.copy2(ROOT / 'scripts/generate-version-header.py', root / 'scripts/generate-version-header.py')
            header = root / 'include/hybrid_pq_cp_abe/version.h'
            header.parent.mkdir(parents=True)
            for version in ('6.0.0', '6.1.2-test'):
                (root / 'VERSION').write_text(version + '\n')
                if header.exists():
                    os.utime(header, (1, 1))
                result = subprocess.run([MAKE, 'include/hybrid_pq_cp_abe/version.h'], cwd=root,
                                        capture_output=True, text=True, timeout=30)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn(f'#define HCPABE_LIB_VERSION "{version}"', header.read_text())

    def test_environment_switches_version_files_without_timestamp_rebuilds(self):
        with tempfile.TemporaryDirectory(prefix='version selection ') as directory:
            root = Path(directory)
            (root / 'scripts').mkdir()
            for name in ('Makefile', 'Makefile.wasm', 'scripts/windows-shell.sh',
                         'scripts/generate-version-header.py'):
                shutil.copy2(ROOT / name, root / name)
            (root / 'VERSION').write_text('6.1.0\n')
            (root / 'VERSION-DEV').write_text('6.2.0-alpha\n')
            # Selecting a file older than the generated header must still work.
            os.utime(root / 'VERSION-DEV', (1, 1))
            header = root / 'include/hybrid_pq_cp_abe/version.h'
            for makefile in ('Makefile', 'Makefile.wasm'):
                if makefile.endswith('.wasm') and os.name == 'nt':
                    continue  # WASM's /bin/bash path is checked on Unix/WSL.
                for source, expected in [('VERSION', '6.1.0'),
                                         ('VERSION-DEV', '6.2.0-alpha'),
                                         ('VERSION', '6.1.0')]:
                    env = dict(os.environ, VERSION_FILE=source)
                    command = [MAKE, '-f', makefile, 'include/hybrid_pq_cp_abe/version.h']
                    result = subprocess.run(command, cwd=root, env=env,
                                            capture_output=True, text=True, timeout=30)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertIn(f'"{expected}"', header.read_text())
                    before = header.stat().st_mtime_ns
                    result = subprocess.run(command, cwd=root, env=env,
                                            capture_output=True, text=True, timeout=30)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertEqual(before, header.stat().st_mtime_ns)


if __name__ == '__main__':
    unittest.main()
