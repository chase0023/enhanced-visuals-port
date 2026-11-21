# EnhancedVisuals for Horizon

A native Horizon mod (`.hmod`) that ports EnhancedVisuals 1.8.35 (Fabric, Minecraft 26.3;
CreativeMD and AriaFreeze, NeoForge credit Sonicjumper; LGPL-3.0-only) to Horizon's Rust client.
The textures, sounds and the numbers behind them are the original's.

| Effect | What it does | Original assets |
|---|---|---|
| Blood on damage | The hit's damage type picks the texture: slashes for a melee hit, impact for falls, pierce for arrows and cactus, the splatter for a sting, thorns, the warden's boom, a mace and anything else the original names no species or weapon for. Count is the original formula `min(5000, (int)(damage × healthScaler(health)) × amount%)`: a nearly dead player bleeds about three times as much for the same hit. Lifetime 25–75 s × the duration setting, drawn at full opacity and faded to nothing (the colour's own 0.7 alpha never reaches the screen — the original's draw call takes the alpha from the opacity), centre-biased placement (`1 − rand²`, as `generateOffset`), random size (20–100% of 15% of the GUI height) and one of eight stored rotations. | `splatter`, `slash`, `impact`, `pierce` |
| Elemental damage | The original's counts and lifetimes: fire 1 splash tinted black over 100–1000 ticks, freezing 4 ice `#C8FFFF`, drowning 4 blue `#0000FF` blobs, wither 4 black at 1.5–2.5× size, lightning 10 `#7878FF` shocks, 4 white `break` shards for `fly_into_wall`, 4 green `#007E00` parasites, 4 white heat splats for `hyperthermia`, each 10–15 ticks. Starving draws only the tunnel overlay, as the original does. | `fire`, `ice`, `wither`, `shock`, `blob`, `dust`, `break`, `parasite`, `heat` |
| Low-health pulse | Below 6 health a red pulse follows the heartbeat: the buffer is `health / max × 100 + 15` ticks, the out beat at 0 and the in beat at 5, peak `min(0.7, intensity) × 0.5 × strength`. | overlay `low_health` |
| Heartbeat | The original's `heartbeatin.ogg`, `heartbeatout.ogg` and `ringing.ogg`, played as the original played them. | `sounds/` |
| Explosions | `ExplosionHandler`'s own maths: damage `(d² + d)/2 × 7 × 2r + 1` from `(1 − distance/2r) × getSeenPercent`, so a wall between you and the blast means nothing at all; dust `IntCurve(0,0,20,30)`, 500–1000 ticks; the ringing at 0.5 fading over `IntCurve(0,50,20,200)` ticks while **everything else is muffled and low-passed** over the same curve. No camera shake — the original has none. | `dust` |
| Splash potions | `PotionHandler`: the potion overlay, peak `min(1, 2 × (1 − distance/5))`, 30–60 ticks. | overlay `potion` |
| Sand | `SandSplatHandler`: `(int)(random × 1.5)` sand splats a tick on the ground on `minecraft:sand` while sprinting, none while walking, 100 ticks each. | `sand` (6 stills) |
| Enderman | `SlenderHandler`: the `slender` overlay, opacity `0.25 × 1/(distance/3)²` capped at 0.3, for the nearest enderman within 16 blocks. | overlay `slender` |
| Hunger | `SaturationHandler`: the frame desaturates as food drops — `Saturation` 1.0 (untouched) at food 8 or more, 0.0 (grey) at 2 or less — faded by 0.0005 a tick. | — (see below) |
| Blur | Three sources, as the original has: the blast's `maxBlur` (50–100 px over 10–20 ticks), the heartbeat's beats (`min(0.7, intensity) × 50`, so 20 px at one heart and up to 35 px, over 5 ticks), and 5–10 px for 10 ticks when the eyes enter or leave water (`SplashHandler`). The original's `UnderwaterHandler` is `enabled = false` there, so it is not ported. Drawn by the companion shader pack; without it a flat haze stands in. | — (see below) |
| Rain | Rain drops, the count from the look angle (the original's `lookUpFactor` 2.0 and `lookDownFactor` 0.1), one a tick under open sky, 100 ticks each. | `drop` |
| Hurt shapes | The original picks the shape from the attacker's held item (sword/axe → slash, pickaxe/shovel → impact, hoe → pierce) and, with an empty hand, from its species; a melee hit therefore draws the slash, an arrow pierce, a fall impact, anything else the splatter. | `slash`, `impact`, `pierce`, `splatter` |
| Hit flash | The original's `hitEffectIntensity` overlay, off by default there and here. | overlay `damaged` |
| Animation | The original's own frame clocks: a group with a `.ani` names `50`, so a frame lasts 150 ms (`wither`, `parasite`, `shock`), and a group without one still animates at 3 ms a frame (`heat`), both on a clock every particle of a kind shares — the original reads `System.nanoTime`, so particles made at different moments show the same frame. A single file is a still the particle keeps. `slender` cycles its four overlays at 150 ms (`VisualTypeOverlay(name, 50)`). `impact` has three variants: the fourth file is `impact3alphatest.png`, which the original's `impact3.png` lookup never finds. | all |
| Death screen | `DeathMessages`: a quip under "You died!", quoted and centred at y 114, picked with the original's `nextInt(size - 1)` — which never picks the last of its nine messages. | — |
| Settings | Twelve rows on the mod's page in **Options > Mods**: blood, hit flash, blood amount, blood duration, max splatters, low-health pulse, pulse strength, heartbeat volume, heartbeat intensity, sounds, rain, the shader pack's blur, and the death screen's quips. | — |

Drawn in its own HUD layer, so F1 hides it with the rest of the HUD. A spectator sees nothing of
it, as in the original, nothing draws outside a world, and leaving the world clears everything.

## Layout

| Path | What it is |
|---|---|
| `horizon-mod.toml` | The manifest; `[native]` pins the build it is made for |
| `src/lib.rs` | The hooks: settings, the HUD layer, the world's damage/blast/potion moments, the client tick |
| `src/effects.rs` | All the logic and every number, with no Horizon types: 24 tests run without the game |
| `src/sprites.rs` | Generated: the grid texture of each particle family |
| `assets/enhancedvisuals/textures/gui/ev/` | The original textures, pre-rotated and packed |
| `assets/enhancedvisuals/sounds/`, `sounds.json` | The original sounds, unchanged |
| `assets/enhancedvisuals/lang/en_us.json` | The settings' labels |
| `tools/gen_assets.py` | Rebuilds the textures, `src/sprites.rs`, the sounds and the icon from the original jar |
| `shaderpack/native-enhancedvisuals.hshader` | The companion pack: the blur and desaturation over the world's own picture. Also in `assets/enhancedvisuals/`, so the mod installs it into `shaderpacks/` |
| `shaderpack/src/`, `tools/pack_shader.py` | The pack's source and the script that checks and packs it |
| `LICENSE`, `NOTICE` | LGPL-3.0-only, as the original |

## The blur: real blur, needs the companion pack

The original's blur is a post-process: `shaders/post/blur.json` blurs the finished frame, and a
desaturation shader turns it grey. A `.hmod` cannot sample the frame, so by default this port
draws a **flat wash** where the original blurs — a grey tint, not a blur. That difference is worth
closing, and Horizon has the one mechanism that can: a **shader pack** whose post pass reads the
frame and the eight `vec4` slots a mod hands it every frame (`frame.mods`, the `shader_params`
hook).

`shaderpack/native-enhancedvisuals.hshader` (source in `shaderpack/src`, built by
`python tools/pack_shader.py`) is that pack. It reproduces the original's own chain, stage for
stage:

* **The blur is the original's radius.** `VisualTypeBlurClient` hands its post chain
  `max(1, floor(intensity))` pixels and runs a chain per visual, so the radii add up. Those are
  `ExplosionHandler`'s `maxBlur` (50 px at no damage, up to 100 px from 10), lasting
  `explosionBlurTime` (10 to 20 ticks); `HeartbeatHandler`'s `min(0.7, intensity) × 50` on the out
  and in beats (up to 35 px, 5 ticks); and `SplashHandler`'s 5–10 px for 10 ticks whenever the eyes
  enter or leave water. The mod reports the sum, in pixels, and the pack blurs by exactly that.
  The original blurs with three rounds of a separable box blur; three box rounds per axis are a
  Gaussian of standard deviation about one radius, so the pack does a two-pass separable Gaussian
  with σ = radius — the same spread for two passes instead of six.
* **The saturation is the original's.** `color_convolve.fsh`'s `Chroma * Saturation + Luma` with
  its own `Gray = vec3(0.3, 0.59, 0.11)`, and `SaturationHandler`'s `DecimalCurve(2.0, 0.0, 8.0,
  1.0)` of the food level: 0 (grey) at 2 or less, 1 (untouched) at 8 or more, faded by 0.0005 a
  tick.

With the mod absent, or both values at rest, it passes the picture through untouched, so it can be
left picked as the home shader.

**To use it: one pick, nothing to copy.** The mod carries the pack in its own assets and writes it
to `shaderpacks/native-enhancedvisuals.hshader` the first time the game starts with it installed
(the name matters: `native-` followed by the mod's id is how Horizon ties a pack to its mod). Then:

1. **Options > Horizon > Shader Packs** → pick **Enhanced Visuals**.
2. That is it. The toggle **Shader pack draws the blur** is already on (its default follows whether
   the mod found the pack), so the flat washes stay off and the effect is not counted twice; the
   mod's frame slots carry the radius in pixels and the `Saturation`.

**Without the pack picked** there is no blur at all, and no hazy stand-in either: turn **Shader
pack draws the blur** off to get the flat haze (a splash's 5 px is a slight veil, a blast's 100 px a
grey screen). That is also the switch for a pack installed under another name, which the mod cannot
find on its own; with the pack missing at startup it writes one there and leaves the toggle off.

The pack passes `horizon-shader check` (shader API 0.2, two passes, `[budget] gpu_ms = 0.8`). It is
built from `shaderpack/src` by `tools/pack_shader.py`; a mod package allows no other `.toml` than
its manifest, so the pack ships as a file of its own (in `shaderpack/` and in the mod's `assets/`,
whence it installs itself).

## How it hangs together

* A Horizon mod draws through the game's `DrawContext`, which blits axis-aligned sprites: it has no
  rotated textured quad. So each original frame is pre-turned into a grid — a column per 45° step,
  a row per animation frame — and the mod picks a cell with the `src` rectangle of one `sprite`
  call. A cell is padded to `content × √2` so a turned square is never cut, and `Sprite::padding`
  gives the splat back its original size. `tools/gen_assets.py` builds those grids; 0/45/90/135°
  covers every direction once the random size and placement are counted in.
* The grid has to fit the game's HUD atlas, which walks `assets/<namespace>/textures/gui` for a mod
  and **drops any non-particle sprite wider than 1024 px or taller than 512** rather than scaling
  it. Four turns across is 728 px at blood's 128 px content; frames are the rows, two per texture,
  so a five-frame group is three textures and no texture is over 728×364. `sprites.rs` carries one
  name per texture and a test pins the sizes, because a texture the atlas drops draws nothing at
  all — silently.
* Everything on screen lives in one `Mutex<State>` in the mod. In single player the world and the
  client are different `Game` instances on different threads, and both reach it.
* The world's hooks only name what happened. `living_damage_pre` reports the damage type of a hit
  on a player, `explosion_prime` the centre and radius, `potion_splash` where a potion broke. The
  client tick compares the local player's health with the previous tick and turns a drop into
  splats — with the waiting damage type if one arrived, else the original's default splatter.
* Joining someone else's server runs only the client's hooks, so a health drop still gives blood,
  but without the damage type.
* Sounds go through `play_local_sound` with the mod's own `sounds.json`, so they are heard by the
  player alone and never sent to the server.
* The textures reach the game's HUD atlas the normal way for a mod: `assets/enhancedvisuals/textures/gui/...`
  is packed under the names `enhancedvisuals:gui/...` (`assets/demo/textures/gui/widget.png` packs
  as `demo:gui/widget` in the game's own tests), and only when the full atlas is built, which is
  the case with mods installed.

## Building it

```sh
cargo test                      # the mod's own logic, no game needed
cargo horizon-mod build         # fetches .horizon/sdk, writes target/horizon-mod/enhancedvisuals.hmod
python tools/pack_shader.py     # checks the companion pack, writes shaderpack/native-enhancedvisuals.hshader
horizon-launcher native-build . --instance "My Instance" --trust-native-code
```

`tools/pack_shader.py` needs `horizon-shader` from the pinned Horizon checkout
(`cargo build -p horizon-shader-api --bin horizon-shader`), on `PATH` or in `HORIZON_SHADER`.

The launcher compiles the mod against the prebuilt game of the build `[native]` pins and makes
that build the one the instance plays. To install by hand, copy the `.hmod` into the game's
`horizon-native-mods/` folder and list it as enabled in `horizon-mods.txt`; disable any older
`enhancedvisuals` mod there first, since both claim the same id.

`--horizon nightly-<YYYY-MM-DD>` and `cargo horizon-mod update --horizon nightly` move the pin; only
a build with a published `sdk-<build>` release can be built against.

## What was verified, and what was not

* `cargo test`: 26 tests pass, covering the damage-type table, the counting and lifetime formulas,
  the heartbeat sequence and its cap, the fade, the frame clock of animated textures (150 ms with a `.ani`, 3 ms without), the death screen's quips, the clear on death, the angle
  padding, every texture's size against the HUD atlas's cap, the overlay order, the blast falloff,
  and the settings' indices.
* The crate compiles against the signed SDK of the pinned build (Nightly 2026-11-20,
  `0a875da9`, content API 0.2, package format 6), with no warning: every hook, event field and
  host method it calls is the SDK's own surface, not a guess.
* `horizon-shader check` passes the companion pack (shader API 0.2, two post passes, no problems),
  and `python tools/pack_shader.py` writes the 2.5 KB `.hshader`.
* `cargo horizon-mod build` completes and writes a 5.0 MB `enhancedvisuals.hmod`: the manifest, the
  sources, `uses.json`, the icon, and every texture and sound under `assets/enhancedvisuals/`.
* The decompiled original (`CFR` on the jar) was read for every number here: the blood colours and
  `healthScaler`, the splash counts and lifetimes, the `.ani` frame clocks (150 ms), the heartbeat
  buffer, the hunger curve, the enderman falloff, the blast curves and the sound muffle.
* **Not verified**: what it looks like in game, and whether the level, entity and setting reads
  answer as expected on your machine. That needs a real Horizon install with Minecraft 26.3
  resources. Check, in a survival world: a zombie's hit (red slashes), a fall (impact splats), a
  skeleton's arrow (pierce), dropping below 3 hearts (pulse and heartbeat), standing in fire (fire
  splats), a creeper at close range (dust, ringing, muffled sound), a splash potion at
  your feet (potion overlay), rain (drops), and every row on the mod's settings page.

## What is not ported, and why

| Missing | Reason |
|---|---|
| The real blur and desaturation **without** the companion pack | A mod cannot sample the frame: `DrawContext` blits sprite rectangles and a `RenderPipeline`'s shader sees only its own quad. Horizon's post passes live in a separate `.hshader` pack, which the player installs and picks — that is what `shaderpack/` is for, and with it picked the blur and the desaturation are the original's own maths (see **The blur** above). Without it the port draws a flat haze and a grey wash on the original's curves, and `UnderwaterHandler`'s standing blur is absent (it is `enabled = false` in the original too). |
| `HealthHandler` | The original ships it `enabled = false`: it would draw one `impact` splat per point of *missing* health, forever. |
| `UnderwaterHandler` | Also `enabled = false`: a standing 5 px blur while the eyes are under water. |
| The `snow` textures | In the jar but no handler draws them. |
| Per-type `opacity` and `disabled` config | `VisualType.opacity` (a multiplier per effect) and its `disabled` switch are config surface; the effect toggles cover the enabled/disabled half. |
| Focus shader | Only the thirst addons trigger it, and Horizon has no thirst system. |
| Cold Sweat / Survive / Tough As Nails addons | Third-party mods' APIs; Horizon has no counterpart to integrate with. |
| The `/displayvisual` and `/clearvisual` commands | `DeathMessages` is ported (the quip under "You died!", at y 114, centred and quoted, from the original's nine); the two server commands are not written yet. |
| Multiplayer damage, blast and potion packets | `DamagePacket`, `ExplosionPacket` and `PotionPacket` are not ported: in single player the world's hooks reach the client in the same process, and on a server that has the mod installed the same hooks run there. A client-only install on someone else's server sees plain health drops. |


## Changes in this pass, towards 1:1

Done: a real blur and desaturation through the companion shader pack (above), with the original's
own radii and saturation maths; the heartbeat's and the water splash's blur, which were missing;
the hunger wash no longer inverted (`Saturation` 1.0 is an untouched picture — the flat wash had
been drawing *strongest* at full food); no camera shake (the original has none); blast damage
multiplied by `getSeenPercent` (a 3×3×3 ray sample over the player's box, as `ServerExplosion`
does); rain asked of `Level.isRainingAt` at feet and head; water read from the fluid at the eyes
(`wasEyeInWater`); the ringing at `maxExplosionVolume` 0.5 fading to 0 over the blast's own curve;
and 8 stored angles instead of 4.

Left, and why:

| Difference | Why it cannot be closed |
|---|---|
| Random 0–360° rotation | `DrawContext::sprite` blits axis-aligned cells; 8 stored angles (45°) is the most the atlas budget carries. A `RenderPipeline` shader could turn a quad, but only for world geometry, not HUD sprites. |
| Real blur and desaturation with no pack picked | A mod cannot sample the frame: pipelines see only their own quad. The pack closes it once picked. |
| Blood shape from the attacker's held item | No host read for another entity's equipment, and no way to name an entity type (`EntityTypeKey` has no public parse), so a melee hit draws the slash. |
| Splash potion tint | No host read for a stack's potion contents. |
| Texture resolution | 96 px non-blood splats and 256 px overlays, to stay inside the atlas budget. |
| Per-handler/per-type `opacity` config, `damageBlackList`, `useHealthPercentage` | Config surface, not visuals; the effect toggles cover the enabled/disabled half. |
| `/displayvisual`, `/clearvisual`, the three packets | Not written yet. |

## Licence

The port is a derivative work of EnhancedVisuals: LGPL-3.0-only. `LICENSE` (the original's) and
`NOTICE` travel inside the `.hmod`. The original textures are redistributed under the same licence,
unmodified apart from being turned into the rotation grids and, for the overlays, downscaled to
256 px; the sounds are byte-for-byte the original's.
