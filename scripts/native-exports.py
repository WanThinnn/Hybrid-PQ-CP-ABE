#!/usr/bin/env python3
"""Generate a Linux export map preserving public C and C++ declarations."""

from pathlib import Path
import re
import sys

header, output = map(Path, sys.argv[1:])
names = sorted(set(re.findall(r'^\s*LIB_API[ \t]+[^;{}]*?\b(\w+)\s*\(',
                              header.read_text(encoding='utf-8'), re.MULTILINE)))
if not names:
    raise SystemExit('No public LIB_API functions found')
scheme = header.with_name('cpabe-scheme.h').read_text(encoding='utf-8')
utilities = header.with_name('common-utils.h').read_text(encoding='utf-8')
cpp_names = sorted(set(re.findall(r'^int\s+(\w+)\s*\(', scheme, re.MULTILINE) +
                       re.findall(r'^inline\s+[^;{}]*?\b(\w+)\s*\(', utilities, re.MULTILINE)))
contents = ('{\n  global:\n' + ''.join(f'    {name};\n' for name in names) +
            '    extern "C++" {\n' + ''.join(f'      {name}*;\n' for name in cpp_names) +
            '    };\n  local: *;\n};\n')
if not output.is_file() or output.read_text(encoding='utf-8') != contents:
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(contents, encoding='utf-8')
