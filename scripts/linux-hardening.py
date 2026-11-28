#!/usr/bin/env python3
"""Probe Linux C/C++ hardening against the selected compiler and libc."""

import argparse
import json
from pathlib import Path
import shlex
import subprocess
import tempfile


def probe(compiler, language):
    command = shlex.split(compiler)
    with tempfile.TemporaryDirectory(prefix='hcpabe-hardening-') as directory:
        output = str(Path(directory) / 'probe')

        def accepts(flags, source='int main(void) { return 0; }\n', link=False):
            result = subprocess.run(command + ['-x', language, '-O2', '-Werror'] + flags +
                                    ([] if link else ['-c']) + ['-', '-o', output],
                                    input=source, text=True, capture_output=True)
            return result.returncode == 0

        flags = ['-fstack-protector-strong']
        if not accepts(flags):
            raise RuntimeError(f'{compiler}: -fstack-protector-strong is required')
        if accepts(flags + ['-fstack-clash-protection']):
            flags.append('-fstack-clash-protection')
        fortify = 0
        for level in (3, 2):
            source = ('#include <features.h>\n'
                      f'#if !defined(__GLIBC__) || __USE_FORTIFY_LEVEL != {level}\n'
                      '#error unsupported FORTIFY level\n#endif\nint main(void) { return 0; }\n')
            candidate = ['-U_FORTIFY_SOURCE', f'-D_FORTIFY_SOURCE={level}']
            if accepts(flags + candidate, source):
                flags += candidate
                fortify = level
                break
        linker = ['-Wl,-z,noexecstack', '-Wl,-z,relro', '-Wl,-z,now']
        if not accepts(flags + ['-fPIE', '-pie'] + linker, link=True):
            raise RuntimeError(f'{compiler}: Linux PIE/RELRO/NOW/NX link probe failed')
        return {'compiler': compiler, 'language': language, 'cflags': flags,
                'fortify': fortify, 'ldflags': linker, 'cli_cflags': ['-fPIE'],
                'cli_ldflags': ['-pie']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', required=True)
    parser.add_argument('--language', choices=['c', 'c++'], default='c++')
    parser.add_argument('--format', choices=['flags', 'json'], default='json')
    args = parser.parse_args()
    result = probe(args.compiler, args.language)
    print(' '.join(result['cflags']) if args.format == 'flags' else json.dumps(result))


if __name__ == '__main__':
    main()
