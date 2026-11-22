---
title: Encrypt Files with CP-ABE in Python
description: Encrypt and decrypt a file in Python using CoverCrypt hybridized mode, attribute-based access policies and the modern PQ-CP-ABE API.
icon: lucide/file-lock-2
---

# Encrypt files with CP-ABE in Python

This walkthrough encrypts a report for users who belong to **finance** and have
the **admin** role. It uses the native Python API, CoverCrypt in `HYBRIDIZED`
mode and AES-256-GCM. A finance viewer's key cannot decrypt the same report.

New to attribute-based encryption? Read [how CP-ABE and post-quantum ABE
work](../../architecture/post-quantum-cp-abe.md). For backend selection, see
[AC17 vs TKN20 vs CoverCrypt](../../architecture/scheme-comparison.md).

## Prepare the Python SDK

Use Python 3.10+ and a package/native runtime supporting the modern 6.1 API:

```sh
python -m pip install --upgrade pq-cp-abe
python -c "import pq_cp_abe as abe; print(abe.__version__, abe.get_version())"
```

If the required release is only available in your local checkout, install it
from the repository root with `python -m pip install -e ./bindings/python`.
An editable installation does not rebuild the native library. Follow the
[Python installation and build guide](build.md) if the installed runtime is older
or a compatible wheel is unavailable.

## Encrypt, decrypt and check access

Save this complete example as `encrypt_report.py`, then run
`python encrypt_report.py`. It creates a new temporary directory for each run,
so it does not overwrite your existing reports.

```python
from pathlib import Path
from tempfile import mkdtemp

from pq_cp_abe import KemMode, PolicyMismatchError, PqCpAbe

abe = PqCpAbe(
    config={
        "dimensions": [
            {"name": "department", "attributes": ["finance", "engineering"]},
            {"name": "role", "attributes": ["admin", "viewer"]},
        ]
    },
    mode=KemMode.HYBRIDIZED,
)

# Authority: create keys and issue the two users' attribute keys.
authority = abe.setup()
admin_key = abe.keygen(
    authority.master_secret_key, "department::finance role::admin"
)
viewer_key = abe.keygen(
    authority.master_secret_key, "department::finance role::viewer"
)

directory = Path(mkdtemp(prefix="cpabe-python-"))
source = directory / "report.txt"
encrypted = directory / "report.cpabe"
restored = directory / "report.restored.txt"
source.write_bytes(b"Quarterly finance report: confidential.\n")

# Sender: only the public key and the policy are needed for encryption.
policy = "department::finance && role::admin"
ciphertext = abe.encrypt(
    authority.public_key, source.read_bytes(), policy, aes_key_bits=256
)
encrypted.write_bytes(ciphertext)

# Recipient: use an issued user key, not the authority master secret.
plaintext = abe.decrypt(admin_key, encrypted.read_bytes())
restored.write_bytes(plaintext)
assert restored.read_bytes() == source.read_bytes()

try:
    abe.decrypt(viewer_key, ciphertext)
except PolicyMismatchError:
    print("Access denied for the finance viewer, as expected.")
else:
    raise AssertionError("The viewer must not decrypt the admin report")

print(f"Round trip succeeded. Files: {directory}")
```

The directory contains the original report, ciphertext and restored report.
Only `report.cpabe` is encrypted; the two `.txt` files contain plaintext.
The program raises an error if the authorized round trip fails or the viewer
unexpectedly gains access.

## Use your own files and keys

Replace the sample report with a path to your input file and choose a separate
output path. This example reads the entire file into memory; it is not a
streaming implementation. For the SDK's native path-based operations, see
the [Python file API reference](api.md) and [CLI guide](cli.md).

The demo creates a new authority and keeps all keys in memory. Once it exits,
the saved ciphertext cannot be recovered without the corresponding retained
user key or master secret. In an application, persist the authority securely,
issue user keys through a trusted service and authenticate the public key
distributed to senders. A sender needs no master secret; a recipient receives
only the attributes they are authorized to use.

For sender authentication, use `setup_with_signing`, `encrypt_and_sign` and
`decrypt_and_verify` as shown in the [signed Python workflow](README.md#signed-encryption-ml-dsa).
AES-GCM authenticates encrypted data but does not identify who sent it.

## Continue

- [Python API reference](api.md): method signatures, parameters and errors.
- [Encrypt files in JavaScript](../wasm/encrypt-files.md): the same policy in Node.js.
- [Compare schemes and modes](../../architecture/scheme-comparison.md): choose a backend for your requirements.
- [Published benchmarks](../../architecture/benchmarks.md): inspect recorded latency, payload sizes and configuration.
