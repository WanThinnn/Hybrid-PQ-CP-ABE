#!/usr/bin/env python3
"""Compile the native example extracted from Markdown against an isolated SDK.

No benchmark is run. Published SDK downloads are checked against SHA256SUMS.txt.
"""
import argparse
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import shutil
import subprocess
import tempfile
from urllib.request import Request, urlopen
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def blocks(text, language):
    return re.findall(r'^```' + language + r'\n(.*?)^```\s*$', text, re.M | re.S)


def extract_examples(root=ROOT):
    sources = [(root / name).read_text(encoding='utf-8') for name in
               ('docs/native/api.md', 'docs/bindings/download.md')]
    code = [blocks(text, 'c')[0] for text in sources]
    if code[0] != code[1]:
        raise ValueError('Download and Native API C examples differ')
    for text in sources:
        if '-std=c17' not in text:
            raise ValueError('The native examples must explicitly select C17')
    commands = [re.findall(r'^(?:gcc|clang) -std=[^\n]+', source, re.M) for source in sources]
    if commands[0] != commands[1]:
        raise ValueError('Download and Native API compiler commands differ')
    if not commands[0] or any(not re.match(r'^(?:gcc|clang) -std=c17(?:\s|$)', command) for command in commands[0]):
        raise ValueError('Every documented native compiler command must select C17')
    return code[0], sources[0]


def extract_archive(raw, destination):
    destination = destination.resolve()
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        for member in archive.infolist():
            path = PurePosixPath(member.filename)
            if path.is_absolute() or '..' in path.parts or '\\' in member.filename or ':' in member.filename:
                raise ValueError(f'Unsafe archive entry: {member.filename}')
            if (member.external_attr >> 16) & 0o170000 == 0o120000:
                raise ValueError('SDK archive must not contain symlinks')
        archive.extractall(destination)
    candidates = [path.parent for path in destination.rglob('include')
                  if path.is_dir() and (path.parent / 'lib').is_dir()]
    if len(candidates) != 1:
        raise ValueError('Expected one SDK root with include/ and lib/')
    return candidates[0]


def download_sdk(destination):
    # Reuse the release checksum parser and bounded downloads used by wheels.
    spec = importlib.util.spec_from_file_location('stage_native', ROOT / 'bindings/python/stage_native.py')
    stage = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(stage)
    headers = {'User-Agent': 'pq-cp-abe-doc-examples', 'Accept': 'application/vnd.github+json'}
    if os.environ.get('GITHUB_TOKEN'):
        headers['Authorization'] = 'Bearer ' + os.environ['GITHUB_TOKEN']
    with urlopen(Request('https://api.github.com/repos/WanThinnn/Hybrid-PQ-CP-ABE/releases/latest', headers=headers), timeout=30) as response:
        release = json.load(response)
    system = {'Windows': 'win', 'Linux': 'linux', 'Darwin': 'macos'}[platform.system()]
    arch = stage.normalized_arch()
    pattern = rf'libhybrid-pq-cp-abe_{system}(?:12)?_{arch}_v[^/]+\.zip'
    assets = release['assets']
    sdk = next((asset for asset in assets if re.fullmatch(pattern, asset['name'])), None)
    checksum = next((asset for asset in assets if asset['name'] == 'SHA256SUMS.txt'), None)
    if sdk is None or checksum is None:
        raise ValueError(f'Latest stable release lacks SDK/checksums for {system}/{arch}')
    expected = stage.checksums(stage.download(checksum['browser_download_url'], 200_000))
    raw = stage.download(sdk['browser_download_url'], 500_000_000)
    if hashlib.sha256(raw).hexdigest() != expected.get(sdk['name']):
        raise ValueError('Published SDK SHA-256 mismatch')
    print(f'Verified {release["tag_name"]}: {sdk["name"]}', flush=True)
    return extract_archive(raw, destination)


def run_example(sdk, directory, *, clean_env=False, powershell='pwsh'):
    code, text = extract_examples()
    sdk = sdk.resolve()
    work = directory / 'example'
    work.mkdir()
    (work / 'demo.c').write_text(code, encoding='utf-8')
    system = platform.system()
    env = os.environ.copy()
    if clean_env:
        for key in list(env):
            if key.upper() in {'CPATH', 'C_INCLUDE_PATH', 'CPLUS_INCLUDE_PATH', 'LIBRARY_PATH', 'INCLUDE', 'LIB', 'PYTHONPATH', 'LD_LIBRARY_PATH', 'DYLD_LIBRARY_PATH', 'DYLD_FALLBACK_LIBRARY_PATH', 'CFLAGS', 'CXXFLAGS'} or key.startswith('HCPABE_'):
                del env[key]
    env['CPABE_DOCS_SDK'] = str(sdk)
    env.pop('PSModulePath', None)
    if system == 'Windows':
        shell = shutil.which(powershell)
        compiler = shutil.which('gcc')
        if not shell or not compiler:
            raise RuntimeError('PowerShell and Windows GCC must be available on PATH')
        section = text.split('### Compile and run on Windows', 1)[1].split('### Compile and run on Linux', 1)[0]
        snippets = blocks(section, 'powershell')
        compilation = next(block for block in snippets if block.lstrip().startswith('gcc -std='))
        execution = next(block for block in snippets if block.lstrip().startswith('Set-Content'))
        for name in ('libhybrid-pq-cp-abe.dll', 'libcpabe_tkn20.dll'):
            if not (sdk / 'lib' / name).is_file():
                raise ValueError(f'Missing packaged runtime: {name}')
        if clean_env:
            windows = Path(os.environ['SystemRoot'])
            env['PATH'] = os.pathsep.join([str(Path(compiler).parent), str(Path(shell).parent), str(windows / 'System32'), str(windows)])
        # Run CLI help too, but do not rely on a globally installed CLI.
        script = '$ErrorActionPreference = "Stop"\n$SDK = $env:CPABE_DOCS_SDK\n'
        script += '& "$SDK\\bin\\pq-cp-abe.exe" --help\nif ($LASTEXITCODE -ne 0) { throw "CLI failed" }\n'
        script += compilation + '\n' + execution + '\nif ($LASTEXITCODE -ne 0) { throw "Comparison failed" }\n'
        path = work / 'example.ps1'
        path.write_text(script, encoding='utf-8')
        command = [shell, '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', str(path)]
    else:
        heading = '#### macOS' if system == 'Darwin' else '#### Linux'
        section = text.split(heading, 1)[1].split('#### ', 1)[0]
        snippet = blocks(section, 'sh')[0]
        # Substitute only the user's SDK placeholder; compile/run commands stay verbatim.
        snippet = re.sub(r'^SDK=.*$', 'SDK="$CPABE_DOCS_SDK"', snippet, flags=re.M)
        if clean_env:
            env['PATH'] = '/usr/bin:/bin:/usr/sbin:/sbin'
        path = work / 'example.sh'
        path.write_text('set -eu\n' + snippet, encoding='utf-8')
        command = ['/bin/bash', str(path)]
    result = subprocess.run(command, cwd=work, env=env, capture_output=True, text=True, errors='replace', timeout=90)
    print(result.stdout, end='')
    if result.returncode:
        raise RuntimeError(f'Example exited {result.returncode}:\n{result.stderr}')
    if (work / 'message.txt').read_bytes() != (work / 'recovered.txt').read_bytes():
        raise AssertionError('Recovered plaintext differs from input')
    print(f'PASS: Markdown C17 example on {system}/{platform.machine()}, clean_env={clean_env}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    inputs = parser.add_mutually_exclusive_group(required=True)
    inputs.add_argument('--sdk', type=Path, help='Existing extracted SDK (never modified)')
    inputs.add_argument('--archive', type=Path, help='Locally built release ZIP')
    inputs.add_argument('--latest', action='store_true', help='Fetch stable release and verify SHA-256')
    parser.add_argument('--clean-env', action='store_true', help='Remove incidental compiler paths/configuration')
    parser.add_argument('--powershell', default='pwsh')
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='pq-abe-docs-') as temporary:
        directory = Path(temporary)
        sdk = args.sdk or (extract_archive(args.archive.read_bytes(), directory / 'sdk') if args.archive else download_sdk(directory / 'sdk'))
        run_example(sdk, directory, clean_env=args.clean_env, powershell=args.powershell)


if __name__ == '__main__':
    main()
