# Build receipt — 1.0.4

What produced `enhancedvisuals.hmod` on this repository's `v1.0.4` release, so the published file can be
traced to its source.

| | |
|---|---|
| Source | commit `69001edd67e825942e26fc0ac1192bdbf1ac1ed2`, tree `a9701b9c8414537ac9ee7cc1650c7a9143735300`. The `v1.0.4` tag points at the commit after it, which adds only this receipt; a package does not carry it, so rebuilding from either is the same digest |
| Horizon (pinned in `horizon-mod.toml`) | `f1bb66f353cf67f2aa4665293fc75c8d91186fc7` — Nightly 2026-11-22 |
| horizon-mixins | `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` |
| SDK | `sdk-f1bb66f353cf67f2aa4665293fc75c8d91186fc7`, the signed SDK the launcher fetches for that pin |
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13) |
| Command | `cargo horizon-mod build` (with `cargo-horizon-mod` at the pin), from a clean tree |
| File | `enhancedvisuals.hmod`, 5047079 bytes |
| SHA-256 | `c1b4572d26214a73fd82737fd5d5f9c211dc0f73b4dbd3d22205f7f15d933157` |
| Signature | `enhancedvisuals.hmod.minisig`, key `RWRWs/XGouk786…` |

`cargo horizon-mod update` from 2b77287 to this build reported **none of the 275 API items the mod uses changed**, so
this version is a repin and a rebuild, not a code change.

The package's licence members are `LICENSE` (LGPLv3), `COPYING` (the GNU GPL v3 the LGPLv3 incorporates) and
`NOTICE`, all three inside the archive.

`cargo horizon-mod check` on the built file:

```
    enhancedvisuals 1.0.4 (native-mod, format native-1/f1bb66f353cf67f2aa4665293fc75c8d91186fc7/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
      5047079 bytes, SHA-256 c1b4572d26214a73fd82737fd5d5f9c211dc0f73b4dbd3d22205f7f15d933157, unsigned
      native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```
