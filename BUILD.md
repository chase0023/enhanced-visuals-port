# Build receipt — 1.0.2

What produced `enhancedvisuals.hmod` on this repository's `v1.0.2` release, so the published file can be
traced to its source.

| | |
|---|---|
| Source | commit `72a83c35fc955f7d9d9eb8a1a3e419cdbe86e772`, tree `4d0971caf7768f69c03b8a0b62f4887184d7d2bf`. The `v1.0.2` tag points at the commit after it, which adds only this receipt; a package does not carry it, so rebuilding from either is the same digest |
| Horizon (pinned in `horizon-mod.toml`) | `8f83e5c85ae3b9a117f742e8d91170c5c9080a06` — the Nov 21 build |
| horizon-mixins | `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` |
| SDK | `sdk-8f83e5c85ae3b9a117f742e8d91170c5c9080a06`, the signed SDK the launcher fetches for that pin |
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| Command | `cargo horizon-mod build` (with `cargo-horizon-mod` at the pin), from a clean tree |
| File | `enhancedvisuals.hmod`, 5047050 bytes |
| SHA-256 | `b27c0fcff84c8f7fbd3ca459653cd35c0840070e2463b547107bdf1fc914685e` |
| Signature | `enhancedvisuals.hmod.minisig`, key `RWRWs/XGouk786…` |

The package's licence members are `LICENSE` (LGPLv3), `COPYING` (the GNU GPL v3 the LGPLv3 incorporates) and
`NOTICE`, all three inside the archive.

`cargo horizon-mod check` on the built file (`cargo-horizon-mod` at `8f83e5c85a…`, the revision this registry
pins):

```
    enhancedvisuals 1.0.2 (native-mod, format native-1/8f83e5c85ae3b9a117f742e8d91170c5c9080a06/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
      5047050 bytes, SHA-256 b27c0fcff84c8f7fbd3ca459653cd35c0840070e2463b547107bdf1fc914685e, unsigned
      native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```
