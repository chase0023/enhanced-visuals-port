# Build receipt — 1.0.1

What produced `enhancedvisuals.hmod` on this repository's `v1.0.1` release, so the published file can be
traced to its source.

| | |
|---|---|
| Source | commit `ea27df9b43712f9b03e866e1ddf49f74274c0019`, tree `55baaa21edfb90d94c64cd78948033aabfc6a192` — the `v1.0.1` tag; this receipt was committed after the build and a package does not carry it, which is why rebuilding on top of it yields the same digest |
| Horizon (pinned in `horizon-mod.toml`) | `8f83e5c85ae3b9a117f742e8d91170c5c9080a06` — the Nov 21 build |
| horizon-mixins | `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` |
| SDK | `sdk-8f83e5c85ae3b9a117f742e8d91170c5c9080a06`, the signed SDK the launcher fetches for that pin |
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| Command | `cargo horizon-mod build` (with `cargo-horizon-mod` at the pin), from a clean tree |
| File | `enhancedvisuals.hmod`, 5033278 bytes |
| SHA-256 | `06c2fbca3734c02ab820b434c4f1696ca02302b008a021e054bd8e02fe26b8e5` |
| Signature | `enhancedvisuals.hmod.minisig`, key `RWRWs/XGouk786…` |

`cargo horizon-mod check` on the built file (`cargo-horizon-mod` at `8f83e5c85a…`, the revision this registry
pins):

```
    enhancedvisuals 1.0.1 (native-mod, format native-1/8f83e5c85ae3b9a117f742e8d91170c5c9080a06/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
      5033278 bytes, SHA-256 06c2fbca3734c02ab820b434c4f1696ca02302b008a021e054bd8e02fe26b8e5, unsigned
      native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

This file is not part of the package: a package carries only the manifest, the Cargo project, `src/`,
`assets/`, the notices and the icon. Committing it after the build and rebuilding gives the same digest,
which is how the digest above was recorded.
