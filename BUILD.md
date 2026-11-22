# Build receipt — 1.0.3

What produced `enhancedvisuals.hmod` on this repository's `v1.0.3` release, so the published file can be
traced to its source.

| | |
|---|---|
| Source | commit `9e3510b83b657da9f9966898648adf984f16de67`, tree `56ca926fbff7b5e62854f46a06068be7c7085463`. The `v1.0.3` tag points at the commit after it, which adds only this receipt; a package does not carry it, so rebuilding from either is the same digest |
| Horizon (pinned in `horizon-mod.toml`) | `2b77287de099c9316278ba7ac4d83590fdb7ef30` — Nightly 2026-11-22 |
| horizon-mixins | `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` |
| SDK | `sdk-2b77287de099c9316278ba7ac4d83590fdb7ef30`, the signed SDK the launcher fetches for that pin |
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| Command | `cargo horizon-mod build` (with `cargo-horizon-mod` at the pin), from a clean tree |
| File | `enhancedvisuals.hmod`, 5047056 bytes |
| SHA-256 | `41df409cfa46ad54485ba726476188ac9257a72a767bb82fcc5159a234dd2c3c` |
| Signature | `enhancedvisuals.hmod.minisig`, key `RWRWs/XGouk786…` |

`cargo horizon-mod update` from the Nov 21 build (8f83e5c) to this one reported **none of the 275 API items the
mod uses changed**, so this version is a repin and a rebuild, not a code change.

The package's licence members are `LICENSE` (LGPLv3), `COPYING` (the GNU GPL v3 the LGPLv3 incorporates) and
`NOTICE`, all three inside the archive.

`cargo horizon-mod check` on the built file (`cargo-horizon-mod` at the registry's pinned revision,
`8f83e5c85a…`):

```
    enhancedvisuals 1.0.3 (native-mod, format native-1/2b77287de099c9316278ba7ac4d83590fdb7ef30/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
      5047056 bytes, SHA-256 41df409cfa46ad54485ba726476188ac9257a72a767bb82fcc5159a234dd2c3c, unsigned
      native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```
