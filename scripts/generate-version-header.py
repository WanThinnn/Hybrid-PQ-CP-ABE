#!/usr/bin/env python3
"""Embed the selected build version, preserving timestamps when unchanged."""

import argparse
from pathlib import Path
import re
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    version = args.source.read_text(encoding='utf-8').strip()
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9.+_-]*', version):
        parser.error(f'Invalid or empty version in {args.source}')
    contents = (
        '// Generated build version; do not edit.\n'
        '#ifndef HYBRID_CP_ABE_VERSION_H\n'
        '#define HYBRID_CP_ABE_VERSION_H\n'
        f'#define HCPABE_LIB_VERSION "{version}"\n'
        '#endif\n'
    )
    if args.output.is_file() and args.output.read_text(encoding='utf-8') == contents:
        return
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', newline='\n',
                                     dir=args.output.parent, delete=False) as stream:
        temporary = Path(stream.name)
        stream.write(contents)
    try:
        temporary.replace(args.output)
    finally:
        temporary.unlink(missing_ok=True)


if __name__ == '__main__':
    main()
