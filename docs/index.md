---
title: Post-Quantum CP-ABE SDK
description: A CP-ABE SDK for Python, JavaScript/WASM and C/C++, with CoverCrypt post-quantum attribute-based encryption, classical AC17/TKN20 and optional ML-DSA signatures.
icon: lucide/house
---

<div class="pq-hero" markdown>

<span class="pq-eyebrow">:lucide-shield-check: Post-quantum access control SDK</span>

# PQ-CP-ABE Documentation

**PQ-CP-ABE** is a **ciphertext-policy attribute-based encryption (CP-ABE)** library and access control SDK for **Python**, **JavaScript / WebAssembly**, native **C/C++**, the command line and **HashiCorp Vault**.

CoverCrypt provides post-quantum attribute-based encryption with ML-KEM-based key encapsulation, while optional ML-DSA signatures provide message authenticity.

<div class="pq-actions" markdown>

[Get started with Python](bindings/python/README.md){ .md-button .md-button--primary }
[Use JavaScript / WASM](bindings/wasm/README.md){ .md-button }
[View on GitHub](https://github.com/WanThinnn/Hybrid-PQ-CP-ABE){ .md-button }

</div>

</div>

<section class="pq-primer" markdown>

<span class="pq-eyebrow">:lucide-layers-3: How it works</span>

## Post-quantum attribute-based encryption

**PQ-ABE** brings attribute-based access policies together with cryptography designed to withstand quantum attacks.

<div class="pq-primer__grid" markdown>

<div class="pq-primer__item" markdown>

### :lucide-key-round: Access follows the ciphertext

In **ciphertext-policy ABE (CP-ABE)**, the sender sets the policy and a trusted authority issues keys carrying attributes. The policy protects a data key; **AES-GCM** encrypts the payload.

[:lucide-arrow-right: Explore the access-control model](bindings/README.md)

</div>

<div class="pq-primer__item" markdown>

### :lucide-shield-check: Post-quantum with CoverCrypt

For **post-quantum CP-ABE**, choose **CoverCrypt**, the SDK's post-quantum ABE backend, with **ML-KEM-768** in hybridized or post-quantum mode. AC17 and TKN20 remain classical; ML-DSA adds signatures independently.

[:lucide-arrow-right: Compare schemes and PQ modes](architecture/README.md)

</div>

</div>

</section>

## Choose your SDK

<div class="grid cards" markdown>

-   <span class="pq-card-icon">:lucide-file-code-2:</span> **Python SDK**

    ---

    Native Python bindings with typed classes, functional APIs, file operations, and CLI support.

    [:lucide-arrow-right: Python guide](bindings/python/README.md)

-   <span class="pq-card-icon">:lucide-box:</span> **JavaScript / WebAssembly**

    ---

    Run the SDK in Node.js or browsers through WebAssembly with TypeScript declarations.

    [:lucide-arrow-right: WASM guide](bindings/wasm/README.md)

-   <span class="pq-card-icon">:lucide-braces:</span> **Native C / C++**

    ---

    Link directly against the native library and use the public ABI without a language binding.

    [:lucide-arrow-right: Native API](native/api.md)

-   <span class="pq-card-icon">:lucide-terminal:</span> **Command line**

    ---

    Set up authorities, issue user keys, encrypt, sign, decrypt, and verify from a terminal.

    [:lucide-arrow-right: CLI reference](native/cli.md)

-   <span class="pq-card-icon">:lucide-key-round:</span> **HashiCorp Vault**

    ---

    Use the Vault secrets engine to centralize authority operations and controlled key issuance.

    [:lucide-arrow-right: Vault integration](integrations/vault.md)

-   <span class="pq-card-icon">:lucide-network:</span> **Architecture**

    ---

    Understand AC17, TKN20, CoverCrypt, access policies, hybrid encryption, and PQC modes.

    [:lucide-arrow-right: Architecture](architecture/README.md)

</div>

## Cryptographic building blocks

<div class="grid cards" markdown>

- :lucide-lock-keyhole:{ .lg .middle } **CP-ABE + AES-GCM**  
  Attribute-based key protection with authenticated bulk encryption.

- :lucide-shield:{ .lg .middle } **ML-KEM**  
  Post-quantum key encapsulation inside supported CoverCrypt modes.

- :lucide-badge-check:{ .lg .middle } **ML-DSA**  
  Optional post-quantum signatures for authenticated encrypt/decrypt flows.

- :lucide-fingerprint:{ .lg .middle } **SHA3-256**  
  Cryptographic hashing for data integrity verification and secure fingerprinting.

- :lucide-key-round:{ .lg .middle } **Curve25519**  
  Classical elliptic-curve key agreement used by CoverCrypt in hybridized mode.

- :lucide-layers-3:{ .lg .middle } **CoverCrypt**  
  Policy-based key encapsulation with classic, post-quantum, and hybridized modes.
  
</div>

<div class="pq-security-note" markdown>

:lucide-info: **Security model:** AC17 and TKN20 remain classical ABE schemes. ML-DSA signatures do not make their key encapsulation post-quantum. CoverCrypt provides the post-quantum confidentiality path when configured with the applicable ML-KEM mode.

</div>

## Reference

[Parameters and ciphertext format](native/parameters.md) ·
[Build from source](native/build.md) ·
[Benchmarks](architecture/benchmarks.md) ·
[Release notes](project/releases.md)
