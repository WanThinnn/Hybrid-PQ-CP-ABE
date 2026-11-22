---
title: Attribute-Based File Encryption in JavaScript
description: Encrypt and decrypt files in Node.js with the PQ-CP-ABE JavaScript/WASM SDK, CoverCrypt hybridized mode and attribute-based access policies.
icon: lucide/file-lock-2
---

# Attribute-based file encryption in JavaScript

Use this Node.js example to encrypt a report for **finance admins** with the
JavaScript/WebAssembly SDK. CoverCrypt's `HYBRIDIZED` mode protects the data-key
encapsulation; AES-256-GCM encrypts the file contents. A finance viewer is denied
access to the same ciphertext.

For the access-control model, start with [post-quantum CP-ABE
explained](../../architecture/post-quantum-cp-abe.md). The
[scheme comparison](../../architecture/scheme-comparison.md) explains how this
differs from AC17 or TKN20.

## Install and prepare Node.js

Use Node.js 18+ with a package supporting the modern 6.1 API:

```sh
npm install @wanthinnn/pq-cp-abe
```

If your installed release lacks the methods below, use a matching newer package
or follow the [WASM source-build guide](build.md). The JavaScript modules and
WASM runtime assets must come from the same build.

## Run the file-encryption example

Save this as `encrypt-report.mjs` in the project where you installed the package.
Run `node encrypt-report.mjs`. Each run creates its own temporary directory.

```js
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';
import {
  KemMode, PolicyMismatchError, PqCpAbe,
} from '@wanthinnn/pq-cp-abe';

const abe = await PqCpAbe.create({
  config: {
    dimensions: [
      { name: 'department', attributes: ['finance', 'engineering'] },
      { name: 'role', attributes: ['admin', 'viewer'] },
    ],
  },
  mode: KemMode.HYBRIDIZED,
});

// Authority: issue keys for two different attribute sets.
const authority = abe.setup();
const adminKey = abe.keygen({
  masterSecretKey: authority.masterSecretKey,
  attributes: 'department::finance role::admin',
});
const viewerKey = abe.keygen({
  masterSecretKey: authority.masterSecretKey,
  attributes: 'department::finance role::viewer',
});

const directory = await mkdtemp(join(tmpdir(), 'cpabe-js-'));
const source = join(directory, 'report.txt');
const encrypted = join(directory, 'report.cpabe');
const restored = join(directory, 'report.restored.txt');
await writeFile(source, 'Quarterly finance report: confidential.\n');

// Node Buffer is a Uint8Array, so readFile output is accepted directly.
const ciphertext = abe.encrypt({
  publicKey: authority.publicKey,
  plaintext: await readFile(source),
  policy: 'department::finance && role::admin',
  aesKeyBits: 256,
});
await writeFile(encrypted, ciphertext);

const plaintext = abe.decrypt({
  secretKey: adminKey,
  ciphertext: await readFile(encrypted),
});
await writeFile(restored, plaintext);
assert.deepEqual(await readFile(restored), await readFile(source));
assert.throws(
  () => abe.decrypt({ secretKey: viewerKey, ciphertext }),
  PolicyMismatchError,
);
console.log('Round trip succeeded; the finance viewer was denied access.');
console.log(`Files: ${directory}`);
```

`PqCpAbe.create` is asynchronous because it loads the WASM runtimes. The
encryption and decryption calls are synchronous after initialization.
The assertions verify both a successful round trip and rejected access.

## Host files, browser files and WASM files

The example uses Node's filesystem to read and write **host files**, then calls
the SDK's byte-based API. It does not pass host paths to `encryptFile` or
`decryptFile`: those methods operate on the separate **WASM virtual filesystem**.
See the [JavaScript file API](api.md#virtual-filesystem-methods) or the
[npm CLI guide](cli.md) if you prefer path-based commands.

For a browser application, load the SDK through your supported bundler/hosting
setup, read a selected `File` with `new Uint8Array(await file.arrayBuffer())`,
and pass those bytes to `abe.encrypt`. The returned ciphertext can be downloaded
as a `Blob`. Node's `node:fs`, `node:os` and `node:path` imports do not run in a
browser. Ensure both WASM assets are served, and use a Web Worker for expensive
operations to keep the interface responsive. See [browser setup](README.md).

## Keep authority keys outside recipient applications

The demo combines authority, sender and recipient in one process for clarity.
It keeps keys in memory; a new run creates a different authority. Retain the
corresponding user key securely if you need to decrypt the saved ciphertext later.
The sample `.txt` files remain plaintext; only `report.cpabe` is encrypted.

In a deployed system, issue user keys in a trusted authority service and give
the sender an authenticated public key. Do not embed the master secret in a
browser bundle. This example loads complete files into memory.

For sender authentication, use `encryptAndSign` and `decryptAndVerify` with
trusted verification keys; see [signed JavaScript operations](api.md#object-interface-pqcpabe-and-scheme-classes).
Encryption alone does not establish the sender's identity.

## Continue

- [JavaScript API reference](api.md): types, factories, errors and signing.
- [Encrypt files in Python](../python/encrypt-files.md): the same policy with native bindings.
- [Compare AC17, TKN20 and CoverCrypt](../../architecture/scheme-comparison.md): backend and mode choices.
- [Published benchmarks](../../architecture/benchmarks.md): read the measured runtime and API before comparing results with WASM.
