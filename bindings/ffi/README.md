# Internal FFI modules

This directory holds the project-owned **native foreign-function interfaces**
used by the Hybrid PQ-CP-ABE SDK's C/C++ core. These are **not**
language-facing SDKs or separately versioned Python/JavaScript packages.

| Module | Language | Interface to the native SDK | Cryptographic / document engine |
| --- | --- | --- | --- |
| `covercrypt/` | Rust | C ABI (`covercrypt_ffi.h`) | Vendored Cosmian CoverCrypt |
| `tkn20/` | Go | C ABI (`cpabe_tkn20.h`) | Vendored Cloudflare CIRCL |
| `pdf-signature/` | Rust | C ABI (`pdf_signature_ffi.h`) | Vendored pqc-underskrift (PDF/CMS only) |

**Public-facing integrations** remain alongside this directory:
`bindings/python/` (native extension SDK), `bindings/wasm/`
(browser/Node WebAssembly SDK), and `bindings/npm/` (npm packaging).

The common dependency source lives in `vendor/`. The Rust crates reference it
using `../../../vendor/...`; the Go module also resolves CIRCL through
`../../../vendor/circl`. Do not duplicate upstream code in this folder.

Build dependencies through `make deps`, or build the PDF C ABI bridge alone
with `make pdf-deps`. Optional WASM PDF staging is `make wasm-pdf-deps`.
Only the PDF/CMS transport is implemented for the PDF bridge so far; the
Composite ML-DSA signer/verifier remains in the planned C++ crypto layer.

Moving one of these modules requires updating its Cargo/Go relative path
dependencies, CI Go module paths, scripts, vendor fingerprint inputs, release
packaging, and documentation. The shared library outputs under
`build/deps/<platform>/` are intentionally unchanged by this organization.
