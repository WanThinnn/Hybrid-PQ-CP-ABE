---
title: AC17 vs TKN20 vs CoverCrypt
description: Compare the AC17, TKN20 and CoverCrypt backends in PQ-CP-ABE, their post-quantum modes, policy configuration and benchmark interpretation.
icon: lucide/scale
---

# AC17 vs TKN20 vs CoverCrypt

PQ-CP-ABE exposes three attribute-based encryption backends through common
Python, JavaScript/WASM and native interfaces. The main selection question is
whether you need **classical ABE** or **post-quantum policy-based key protection**.
For terminology, start with [what post-quantum CP-ABE means](post-quantum-cp-abe.md).

## Compare the backends

| Property | AC17 | TKN20 | CoverCrypt |
| --- | --- | --- | --- |
| Cryptographic family | Classical pairing-based ABE | Classical pairing-based ABE | Policy-based KEM with classical, post-quantum and hybridized modes |
| Post-quantum key protection in this SDK | No | No | Yes, in `HYBRIDIZED` or `POST_QUANTUM` mode |
| CoverCrypt dimensions configuration | Not applicable | Not applicable | Required at authority setup |
| Modern Python/JS scheme selector | `Scheme.AC17` | `Scheme.TKN20` | `Scheme.COVERCRYPT` (default) |
| Dedicated class | `AC17CPABE` | `TKN20CPABE` | `CoverCryptABE` |
| Bulk payload encryption | AES-GCM | AES-GCM | AES-GCM |
| Optional ML-DSA signatures | Supported | Supported | Supported, independently of KEM mode |

The common interface does not make policies or serialized keys interchangeable.
Keep each authority's keys with its scheme. Selecting a different backend does
not convert ciphertext or grant an existing user key new rights.

## Choose for your application

For **post-quantum attribute-based encryption**, select CoverCrypt and explicitly
set the mode you intend to use. `HYBRIDIZED` combines Curve25519 and ML-KEM-768;
`POST_QUANTUM` uses the ML-KEM-based mode. `CLASSIC` provides classical key
encapsulation. See the [mode explanation](post-quantum-cp-abe.md#where-post-quantum-protection-comes-from)
and [Cosmian's CoverCrypt documentation](https://docs.cosmian.com/encrypt/covercrypt.html).

For **classical ABE evaluation or an existing AC17/TKN20 deployment**, choose the
matching backend and verify its policy behavior and ciphertext compatibility.
Do not switch to CoverCrypt by reusing a classical public key. Create or migrate
the authority and issue matching keys under a reviewed migration plan.

If you also need **sender authentication**, add ML-DSA and require verified
decryption. This choice is independent of the ABE backend; a post-quantum
signature does not change the confidentiality properties of classical ABE.

## Select a scheme in code

In Python, pass `scheme` to the facade or use a dedicated class:

```python
from pq_cp_abe import KemMode, PqCpAbe, Scheme

ac17 = PqCpAbe(scheme=Scheme.AC17)
tkn20 = PqCpAbe(scheme=Scheme.TKN20)
covercrypt = PqCpAbe(
    scheme=Scheme.COVERCRYPT,
    config={"dimensions": [{"name": "role", "attributes": ["admin", "viewer"]}]},
    mode=KemMode.HYBRIDIZED,
)
```

In JavaScript, initialize one facade for the selected backend:

```js
import { KemMode, PqCpAbe, Scheme } from '@wanthinnn/pq-cp-abe';

const covercrypt = await PqCpAbe.create({
  scheme: Scheme.COVERCRYPT,
  config: {
    dimensions: [{ name: 'role', attributes: ['admin', 'viewer'] }],
  },
  mode: KemMode.HYBRIDIZED,
});
// For classical ABE, use { scheme: Scheme.AC17 } or { scheme: Scheme.TKN20 }.
```

`config` and `mode` apply to CoverCrypt. Use the
[Python API](../bindings/python/api.md) or [JavaScript API](../bindings/wasm/api.md)
for the complete method signatures. For native C/C++ defaults and selectors,
consult the [native API](../native/api.md); do not infer legacy defaults from
the modern facades.

## Compare measurements fairly

Use the [latest complete published benchmark](benchmarks.md) for measured
latency and the original result files. Read the report's library version,
runtime, policy size, CoverCrypt dimensions/mode, payload sizes, signing options
and comparison semantics before interpreting its charts.

In particular, native Python bindings and JavaScript/WASM have different
runtime costs. A native result is not a browser measurement. Likewise, an ABE
primitive operation and a full ABE-plus-AES payload operation measure different
work, and signing adds another operation. The published report records how
native schemes are compared with the Charm AC17 reference.

No single timing establishes a universal fastest scheme. Choose candidates
that meet your security and policy requirements, then evaluate the workload
you will deploy. The tutorials below demonstrate functionality; they do not
produce benchmark timings.

## Build a complete workflow

- [Python file encryption](../bindings/python/encrypt-files.md): authority setup, policy encryption, decryption and denied access.
- [JavaScript file encryption](../bindings/wasm/encrypt-files.md): the same workflow through WASM in Node.js.
- [Architecture and authority management](README.md): deployment details and extending CoverCrypt dimensions.
- [Cryptographic workflows](detail.md) and [ciphertext format](../native/parameters.md): understand the shared encryption pipeline.
