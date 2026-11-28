#!/usr/bin/env python3
"""Native build input fingerprints and separate staged-output integrity records.

These local records detect accidental stale/corrupt caches, not a malicious
writer able to replace both the outputs and their records.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shlex
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
SOURCES = {
    'cryptopp': ['vendor/cryptopp'],
    'cryptopp-dll': ['vendor/cryptopp'],
    'liboqs': ['vendor/liboqs'],
    'rabe': ['vendor/librabe-ffi'],
    'covercrypt': ['vendor/cover_crypt', 'bindings/ffi/covercrypt'],
    'tkn20': ['vendor/circl', 'bindings/ffi/tkn20'],
    'pqc-underskrift': ['vendor/pqc-underskrift', 'bindings/ffi/pdf-signature'],
}
EXCLUDED = {'.git', 'target', 'build', 'build-cmake', 'CMakeFiles',
            '__pycache__', '.cache', '.work', 'obj', 'out'}
OUTPUT_SUFFIXES = {'.o', '.obj', '.a', '.lib', '.so', '.dylib', '.dll',
                   '.exe', '.pdb', '.pyc', '.exp', '.ilk', '.idb', '.tlog'}


def digest(path):
    value = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            value.update(chunk)
    return value.hexdigest()


def tree(path, source=False):
    if path.is_file():
        yield path
        return
    if not path.is_dir():
        raise FileNotFoundError(path)
    for directory, dirs, files in os.walk(path, followlinks=False):
        dirs[:] = sorted(d for d in dirs if not source or d not in EXCLUDED)
        for name in sorted(files):
            item = Path(directory) / name
            # Crypto++ MSBuild appends to this generated marker in the vendor
            # directory even with OutDir/IntDir redirected. It is not source.
            if source and (item.suffix in OUTPUT_SUFFIXES or
                           name in {'.git', 'CMakeCache.txt', 'adhoc.cpp.copied'}):
                continue
            if not source and name.startswith('.'):
                continue
            yield item


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=True)


def atomic_write(path, value):
    data = canonical(value) + '\n'
    try:
        if path.read_text(encoding='utf-8') == data:
            return
    except (OSError, UnicodeError):
        pass
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(mode='w', encoding='utf-8', newline='\n',
                                     dir=path.parent, delete=False) as stream:
        temporary = Path(stream.name)
        stream.write(data)
    try:
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def tool(command):
    parts = [command] if Path(command).is_file() else shlex.split(command, posix=os.name != 'nt')
    executable = shutil.which(parts[0]) if parts else None
    if not executable:
        raise RuntimeError('Missing fingerprint tool: ' + command)
    path = Path(executable).resolve()
    # Binary identity catches in-place updates, even for tools without a
    # portable version option (MSVC cl/link/lib). Versions also enter context.
    return [str(path), digest(path), parts[1:]]


def fingerprint(args):
    paths = set()
    for name in SOURCES[args.name] + [
            'scripts/build-deps.sh', 'scripts/build-deps.ps1',
            'scripts/native-dependency-cache.py', 'scripts/cryptopp-msvc.props',
            'scripts/linux-hardening.py',
            'scripts/cryptopp-dll.props', 'scripts/normalize-msvc-library.ps1',
            'vendor/upstreams.json']:
        paths.update(tree(ROOT / name, source=True))
    # Include repository Cargo configuration/toolchain overrides when present.
    for name in ['.cargo', 'rust-toolchain', 'rust-toolchain.toml']:
        if (ROOT / name).exists():
            paths.update(tree(ROOT / name, source=True))
    environment = {k: v for k, v in os.environ.items() if k in {
        'CC', 'CXX', 'AR', 'LD', 'CFLAGS', 'CXXFLAGS', 'CPPFLAGS', 'LDFLAGS',
        'CL', '_CL_', 'LINK', '_LINK_', 'INCLUDE', 'LIB', 'LIBPATH',
        'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTUP_TOOLCHAIN',
        'RUSTC', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
        'GOFLAGS', 'GOOS', 'GOARCH', 'GOAMD64', 'GOARM64', 'GOTOOLCHAIN', 'GOEXPERIMENT',
        'CGO_CFLAGS', 'CGO_CPPFLAGS', 'CGO_CXXFLAGS', 'CGO_LDFLAGS',
        'MACOSX_DEPLOYMENT_TARGET', 'SDKROOT', 'CMAKE_TOOLCHAIN_FILE',
        'VCToolsVersion', 'WindowsSDKVersion', 'VSCMD_ARG_TGT_ARCH'
    } or (k.startswith('CARGO_PROFILE_') or
          (k.startswith('CARGO_TARGET_') and k.endswith(('_RUSTFLAGS', '_LINKER'))))}
    tools = (['cl.exe', 'link.exe', 'lib.exe', 'msbuild', 'cmake', 'gcc', 'go', 'cargo']
             if args.platform == 'windows' else
             ['clang', 'clang++', 'cc', 'c++', 'ar', 'ld', 'cmake', 'make', 'go', 'cargo']
             if args.platform == 'macos' else
             ['gcc', 'g++', 'cc', 'c++', 'ar', 'ld', 'cmake', 'make', 'go', 'cargo'])
    if args.name == 'pqc-underskrift':
        # No Go/CMake/MSVC/other signing engines are used by this Rust-only crate.
        tools = ['cargo', 'rustup']
    for name in ('CC', 'CXX', 'AR', 'LD'):
        if os.environ.get(name):
            tools.append(os.environ[name])
    for optional in ('ninja', 'sccache'):
        if shutil.which(optional):
            tools.append(optional)
    # go env includes persistent go env -w settings, not just process env.
    go_env = subprocess.check_output(['go', 'env', '-json', 'GOOS', 'GOARCH',
        'GOAMD64', 'GOARM64', 'GOFLAGS', 'CGO_CFLAGS', 'CGO_CPPFLAGS',
        'CGO_CXXFLAGS', 'CGO_LDFLAGS', 'GOVERSION', 'GOTOOLCHAIN', 'GOEXPERIMENT',
        'CC', 'CXX', 'AR', 'GOWORK'], text=True) if args.name != 'pqc-underskrift' else '{}'
    for compiler in filter(None, (json.loads(go_env).get('CC'), json.loads(go_env).get('CXX'))):
        tools.append(compiler)
    external_config = {}
    cargo_home = Path(os.environ['CARGO_HOME']) if os.environ.get('CARGO_HOME') else Path.home() / '.cargo'
    for path in [cargo_home / 'config', cargo_home / 'config.toml',
                 Path(os.environ.get('CMAKE_TOOLCHAIN_FILE', '__missing__'))]:
        if path.is_file():
            external_config[str(path.resolve())] = digest(path)
    value = {'schema': 1, 'platform': args.platform, 'machine': platform.machine(),
             'name': args.name, 'context': args.context, 'environment': environment,
             'go_env': json.loads(go_env), 'external_config': external_config,
             'tools': [tool(t) for t in tools],
             'source': [(p.relative_to(ROOT).as_posix(), digest(p))
                        for p in sorted(paths)]}
    print(hashlib.sha256(canonical(value).encode('ascii')).hexdigest())


def output_paths(base, name):
    dep = 'cryptopp' if name == 'cryptopp-dll' else name
    lib = base / 'lib' / dep
    if name == 'cryptopp-dll':
        return [lib / 'cryptopp.dll', lib / 'cryptopp.lib']
    if name == 'pqc-underskrift':
        return [*tree(lib), *tree(base / 'include' / dep)]
    if name == 'cryptopp' and (base / 'lib').parent.name == 'windows':
        return [lib / 'libcryptopp.lib', *tree(base / 'include' / dep)]
    return [*tree(lib), *tree(base / 'include' / dep)]


def snapshot(base, name):
    windows = base.name == 'windows'
    extension = 'dylib' if base.name == 'macos' else 'so'
    required = {
        'cryptopp': ['lib/cryptopp/libcryptopp.lib', 'include/cryptopp/aes.h'] if windows else
                    ['lib/cryptopp/libcryptopp.a', f'lib/cryptopp/libcryptopp.{extension}', 'include/cryptopp/aes.h'],
        'cryptopp-dll': ['lib/cryptopp/cryptopp.dll', 'lib/cryptopp/cryptopp.lib'],
        'liboqs': ['lib/liboqs/liboqs.lib', 'lib/liboqs/liboqs.dll', 'lib/liboqs/liboqs_import.lib', 'include/liboqs/oqs/oqs.h'] if windows else
                  ['lib/liboqs/liboqs.a', f'lib/liboqs/liboqs.{extension}', 'include/liboqs/oqs/oqs.h'],
        'rabe': ['lib/rabe/librabe_ffi.lib', 'lib/rabe/rabe_ffi.dll', 'include/rabe/rabe.h'] if windows else
                ['lib/rabe/librabe_ffi.a', f'lib/rabe/librabe_ffi.{extension}', 'include/rabe/rabe.h'],
        'covercrypt': ['lib/covercrypt/libcovercrypt_ffi.lib', 'lib/covercrypt/covercrypt_ffi.dll', 'lib/covercrypt/covercrypt_ffi.dll.lib', 'include/covercrypt/covercrypt_ffi.h'] if windows else
                     ['lib/covercrypt/libcovercrypt_ffi.a', f'lib/covercrypt/libcovercrypt_ffi.{extension}', 'include/covercrypt/covercrypt_ffi.h'],
        'tkn20': ['lib/tkn20/libcpabe_tkn20.dll', 'lib/tkn20/libcpabe_tkn20.lib', 'include/tkn20/cpabe_tkn20.h'] if windows else
                 ['lib/tkn20/libcpabe_tkn20.a', f'lib/tkn20/libcpabe_tkn20.{extension}', 'include/tkn20/cpabe_tkn20.h'],
        'pqc-underskrift': ['lib/pqc-underskrift/libpdf_signature_ffi.lib',
                           'lib/pqc-underskrift/pdf_signature_ffi.dll',
                           'lib/pqc-underskrift/pdf_signature_ffi.dll.lib',
                           'include/pqc-underskrift/pdf_signature_ffi.h'] if windows else
                           ['lib/pqc-underskrift/libpdf_signature_ffi.a',
                            f'lib/pqc-underskrift/libpdf_signature_ffi.{extension}',
                            'include/pqc-underskrift/pdf_signature_ffi.h'],
    }
    for name_required in required[name]:
        if not (base / name_required).is_file():
            raise FileNotFoundError(base / name_required)
    paths = output_paths(base, name)
    if not paths:
        raise RuntimeError('Empty dependency output: ' + name)
    return [(p.relative_to(base).as_posix(), digest(p)) for p in sorted(paths)]


def record_path(base, name):
    dep = 'cryptopp' if name == 'cryptopp-dll' else name
    return base / 'lib' / dep / ('.dll-integrity.json' if name == 'cryptopp-dll'
                                else '.integrity.json')


def verify(base, name, expected=None):
    value = json.loads(record_path(base, name).read_text(encoding='utf-8'))
    if expected is not None and value['fingerprint'] != expected:
        raise ValueError('Input fingerprint changed: ' + name)
    if value['outputs'] != [list(p) for p in snapshot(base, name)]:
        raise ValueError('Output integrity mismatch: ' + name)
    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['fingerprint', 'check', 'record', 'ready'])
    parser.add_argument('platform', choices=['windows', 'linux', 'macos'])
    parser.add_argument('name', choices=[*SOURCES, 'all'])
    parser.add_argument('context', nargs='?', default='')
    args = parser.parse_args()
    base = ROOT / 'build' / 'deps' / args.platform
    if args.action == 'fingerprint':
        fingerprint(args)
    elif args.action == 'record':
        atomic_write(record_path(base, args.name),
                     {'fingerprint': args.context, 'outputs': snapshot(base, args.name)})
    elif args.action == 'check':
        try:
            verify(base, args.name, args.context)
        except (OSError, ValueError, KeyError, TypeError):
            # Invalidate readiness before a repair starts, including single-dep builds.
            (base / 'lib' / '.ready').unlink(missing_ok=True)
            raise SystemExit(1)
    else:
        names = ['cryptopp', 'liboqs', 'rabe', 'tkn20', 'covercrypt', 'pqc-underskrift']
        if args.platform == 'windows':
            names.append('cryptopp-dll')
        try:
            records = {name: verify(base, name) for name in names}
        except (OSError, ValueError, KeyError, TypeError):
            (base / 'lib' / '.ready').unlink(missing_ok=True)
            raise
        atomic_write(base / 'lib' / '.ready', {'schema': 1, 'dependencies': records})


if __name__ == '__main__':
    main()
