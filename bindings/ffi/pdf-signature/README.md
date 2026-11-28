# PDF Composite ML-DSA FFI bridge

A small Rust-to-C ABI bridge around the vendored `pqc-underskrift` PDF/CMS
engine. The package depends on the local fork with `external-composite` only.
It **does not** implement private-key generation, ML-DSA, Ed25519, Ed448, or
ECDSA signing/verification. The C++ SDK must perform the full Composite
SignPreHashed/VerifyPreHashed operation (RFC 10073); this module handles
CMS (RFC 10082) and PDF embedding.

## Public C ABI

See [include/pdf_signature_ffi.h](include/pdf_signature_ffi.h):

1. `pqpdf_prepare`: PDF and Composite X.509 DER to opaque state and 64-byte
   SignedAttrs pre-hash. Optional DER certificate chain supported.
2. `pqpdf_finalize`: consumes the state, accepts **raw** external Composite
   signature, returns signed PDF with incremental CMS update.
3. `pqpdf_extract`: returns signer certificate, Composite signature, pre-hash
   and whether the signature covers the final PDF revision. **Not cryptographic
   verification** or certificate-chain trust.
4. `pqpdf_prepared_free`, `pqpdf_buffer_free`,
   `pqpdf_verification_free`, `pqpdf_last_error`.

The ABI exposes no Rust structs except an opaque handle; buffers are
allocated/freed by Rust and must not be freed using C/C++ `free` or `delete`.
The raw Composite signature is encoded in CMS `SignerInfo.signature` by
`underskrift`.

## Build

```bash
cargo build --manifest-path bindings/ffi/pdf-signature/Cargo.toml --release --locked
cargo test --manifest-path bindings/ffi/pdf-signature/Cargo.toml --locked
make pdf-deps
```

Native dependency staging provides both static and shared C ABI libraries
under `build/deps/<platform>/lib/pqc-underskrift/` and a public C header
under `build/deps/<platform>/include/pqc-underskrift/`.
The old standalone `underskrift.lib` is superseded by the bridge archive to
avoid duplicate Rust runtime linkage. The **opt-in WASM static archive** has
been built with Emscripten 7.0.0 and Rust nightly on Ubuntu WSL via
`make wasm-pdf-deps -j6`. This does **not** mean PDF signing is exposed by the
main SDK's JS/WASM runtime; the C++ linker/JS exports and cryptographic signer
remain separate future work.

## Boundaries

This phase supports invisible one-signer PDF CMS Composite signatures and
structural verification only. No Composite X.509 certificate issuer, no
private-key API, no crypto verification, no PAdES interoperability guarantee,
no external timestamp/LTV. The bridge has not yet been linked into the native
CP-ABE API or Python/npm SDK.
