"""Small post-link checks; the full cryptographic regression suite runs separately."""

import argparse
import ctypes as c
import json
import os
from pathlib import Path
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('library', type=Path)
    parser.add_argument('cli', type=Path)
    parser.add_argument('--version-file', type=Path, default=Path('VERSION'))
    args = parser.parse_args()
    path = args.library.resolve()
    dll_directory = os.add_dll_directory(str(path.parent)) if os.name == 'nt' else None
    try:
        lib = c.CDLL(str(path))
        root = Path(__file__).resolve().parents[1]
        header = root / 'include/hybrid_pq_cp_abe/hybrid-pq-cp-abe.h'
        names = re.findall(r'^\s*LIB_API[ \t]+[^;{}]*?\b(\w+)\s*\(',
                           header.read_text(encoding='utf-8'), re.MULTILINE)
        for name in names:
            getattr(lib, name)
        lib.getVersion.restype = c.c_char_p
        version = lib.getVersion().decode()
        expected = args.version_file.read_text(encoding='utf-8').strip()
        if version != expected:
            raise RuntimeError(f'Native version {version} != {expected}')
        output = c.POINTER(c.c_void_p)
        length = c.POINTER(c.c_size_t)
        setup = lib.hybrid_cpabe_setupBuffer_with_scheme
        setup.argtypes = [output, length, output, length, c.c_int]
        setup.restype = c.c_int
        covercrypt = lib.hybrid_cpabe_covercrypt_setupBuffer_from_json
        covercrypt.argtypes = [c.c_char_p, c.c_int, output, length, output, length]
        covercrypt.restype = c.c_int
        lib.freeBuffer.argtypes = [c.c_void_p]
        lib.freeBuffer.restype = None
        checked = []
        for name, scheme in [('AC17', 0), ('TKN20', 1), ('CoverCrypt', 2)]:
            pk, msk = c.c_void_p(), c.c_void_p()
            pk_len, msk_len = c.c_size_t(), c.c_size_t()
            try:
                if scheme == 2:
                    status = covercrypt(b'{"dimensions":[{"name":"D","attributes":["yes","no"]}]}',
                                        0, c.byref(pk), c.byref(pk_len), c.byref(msk), c.byref(msk_len))
                else:
                    status = setup(c.byref(pk), c.byref(pk_len), c.byref(msk), c.byref(msk_len), scheme)
                if status != 0 or not pk.value or not msk.value or not pk_len.value or not msk_len.value:
                    raise RuntimeError(f'{name} buffer setup failed: {status}')
                checked.append(name)
            finally:
                lib.freeBuffer(pk)
                lib.freeBuffer(msk)
        for option in ('--version', '--help'):
            result = subprocess.run([str(args.cli.resolve()), option], capture_output=True,
                                    text=True, timeout=30)
            if result.returncode:
                raise RuntimeError(f'CLI {option} failed: {result.stdout}\n{result.stderr}')
            if option == '--version' and expected not in result.stdout + result.stderr:
                raise RuntimeError('CLI version mismatch')
        print(json.dumps({'version': version, 'public_c_functions': len(names),
                          'buffer_setup': checked, 'cli': ['--version', '--help']}))
    finally:
        if dll_directory:
            dll_directory.close()


if __name__ == '__main__':
    main()
