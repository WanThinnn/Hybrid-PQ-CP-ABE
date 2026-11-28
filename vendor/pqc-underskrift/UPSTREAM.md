# Upstream attribution

This directory vendors the maintainer fork of `underskrift` for PDF signing and
future Composite ML-DSA/CMS integration in Hybrid PQ-CP-ABE SDK.

- Upstream project: [`kushaldas/underskrift`](https://github.com/kushaldas/underskrift)
- Repository (update source): https://github.com/WanThinnn/pqc-underskrift
- Vendored version: 0.1.4 (`Cargo.toml`)
- Vendored commit: `3a953ef41043c41c653b7194a155f4c226c4f370`
- License: BSD-2-Clause (declared in the fork's `Cargo.toml`; see original licensing terms)

This is ordinary vendored source, not a Git submodule. The main repository pins
the source SHA in `vendor/upstreams.json`. On a maintainer machine:

```sh
python scripts/vendor.py init pqc-underskrift
python scripts/vendor.py pull pqc-underskrift
```

The vendor helper tracks the fork's default branch and updates this file's
vendored commit after successful merges. PDF Composite integration has not yet
been wired into the SDK build.
