# External Composite ML-DSA PDF/CMS integration

This fork adds a **keyless** signing path for the following LAMPS algorithms:

| Algorithm | RFC 10073 OID | RFC 10082 CMS digest |
| --- | --- | --- |
| ML-DSA-44 + Ed25519 | 1.3.6.1.5.5.7.6.39 | SHA-512 |
| ML-DSA-65 + Ed25519 | 1.3.6.1.5.5.7.6.48 | SHA-512 |
| ML-DSA-87 + Ed448 | 1.3.6.1.5.5.7.6.51 | SHAKE256, 64 bytes |
| ML-DSA-87 + ECDSA P-384 | 1.3.6.1.5.5.7.6.49 | SHA-512 |

## Dependency-minimal build

```sh
cargo +stable check --no-default-features --features external-composite
cargo +stable test --lib --no-default-features --features external-composite
cargo +stable test --test composite_remote --no-default-features --features external-composite
```

The `external-composite` profile **does not compile** `tsp-ltv`, `rsa`,
`p256`, `p384`, or `ed25519-dalek` (nor OpenSSL). PDF/CMS/X.509/DER
parsing and SHA-512/SHAKE256 still compile. The original software signing,
verification, timestamp and LTV stacks are available through their existing
features; the default feature set retains the legacy behavior.

## Signing contract

```text
certificate = X.509 DER with Composite SubjectPublicKeyInfo
algorithm = CompositeAlgorithm::MlDsa65Ed25519
prepared = prepare_composite_signature(pdf_bytes, identity, options)
prehash = prepared.attrs_hash          // SHA-512 or SHAKE256/64 of SignedAttrs DER
signature = SDK.CompositeSignPrehashed(algorithm, prehash, context="")
signed_pdf = finalize_signature(prepared, signature)
```

The external Crypto++/liboqs SDK (NOT this Rust crate) MUST implement the full
RFC 10073 algorithm, including `Prefix || Label || len(ctx) || ctx || PH(M)`,
the ML-DSA component context `Label`, fresh component keys, and the
traditional component signature. The returned signature is a **raw** RFC 10073
composite signature (ML-DSA signature followed by the traditional signature),
not a CMS blob. Rust builds `SignerInfo`, CMS `SignedData`, and the PDF.

`prepare_composite_signature` checks the composite certificate SPKI OID and
parameters, selects the mandated hash, and reserves at least 32 KiB for the
CMS `/Contents` field. `finalize_signature` validates the external signature
length/structure, embeds it in the CMS `signature` OCTET STRING, and adds an
incremental update to the PDF.

For the verification handoff, call
`composite::extract_composite_verification_inputs(signed_pdf)`. It checks
PDF byte ranges, CMS message-digest, signature OID/digest mapping and X.509
SPKI shape, returning `attrs_hash`, the raw composite signature and
certificate DER. **The SDK must still perform BOTH cryptographic component
verifications and certificate trust evaluation.** The extraction result alone
must never be presented as a valid signature.

The `composite_remote` integration tests deliberately use a **synthetic,
cryptographically invalid** Composite certificate and a structurally valid
placeholder signature. Their purpose is to verify CMS/PDF encoding and
pre-hashing, not cryptographic validity, chain trust, or external PDF viewer
compatibility.

## Remaining implementation work

- C++ implementation of RFC 10073 Composite KeyGen/SignPreHashed/VerifyPreHashed
  using liboqs and Crypto++.
- Certificate issuance and chain-validation path for real Composite X.509 keys.
- End-to-end cryptographic tests with independently verified signatures and
  negative cases (changed signed content, swapped OIDs, wrong component).
- Rust FFI bridge to the native SDK, Python/WASM bindings, and optional PDF
  reader interoperability tests.

Standards: [RFC 10073](https://www.rfc-editor.org/rfc/rfc10073.html)
and [RFC 10082](https://www.rfc-editor.org/rfc/rfc10082.html).
