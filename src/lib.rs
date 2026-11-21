//! EnhancedVisuals for Horizon: blood with the original's textures, a heartbeat and a red pulse
//! at low health, explosion dust and ringing, potion, fire, freezing, drowning and starving
//! overlays, and rain drops.
//!
//! A port of EnhancedVisuals 1.8.35 (Fabric, Minecraft 26.3; CreativeMD and AriaFreeze, NeoForge
//! credit Sonicjumper), LGPL-3.0-only. Every number the visuals use is the original's, recovered
//! from its jar; [`effects`] holds them and the tests that pin them, and this file is the glue
//! to the game's hooks.
//!
//! What the hooks do:
//!
//! * `living_damage_pre`, `explosion_prime` and `potion_splash` run on the world's side and only
//!   name what happened: the damage type, where a blast was, where a potion broke. In single
//!   player this is the integrated server, in the same process, so the client tick reads them
//!   from the one `Mutex<State>`.
//! * `client_tick_post` is the clock: it reads the local player's health, turns a drop into
//!   blood, ages everything and plays the queued sounds.
//! * `gui_layer_draw` draws the overlays and the splatters through the game's own HUD batch.
//! * No camera moves: the original has no shake, and neither does this.

mod effects;
mod sprites;

use effects::{
    DrawItem, Input, Options, Sound, State, AMOUNTS, CAPS, DURATIONS, FALLBACK_MAX_HEALTH,
};
use mod_api::{
    self, ClientHost, ClientTickPostEvent, ClipBlock, ClipFluid, EntityRef, ExplosionPrimeEvent,
    GuiLayer, GuiLayerDrawEvent, GuiLayerPlacement, Hooks, LevelRandomHost, LevelRef,
    LivingDamagePreEvent, PotionSplashEvent, RegisterGuiLayersEvent, Registrar,
    RegistrationContext, ScreenRenderPostEvent, ServerHost, SettingSpec, ShaderParamsEvent,
    SoundPlay,
};
use std::sync::Mutex;

/// The mod's id, and the namespace its assets and translation keys live in.
pub const ID: &str = "enhancedvisuals";
/// The one HUD layer the mod draws in, above every vanilla layer.
const LAYER: &str = "enhancedvisuals:hud";

/// The mod. One shared state: the client's hooks and, in single player, the world's hooks are
/// on different threads and both reach it.
#[derive(Default)]
pub struct EnhancedVisuals {
    state: Mutex<State>,
}

impl EnhancedVisuals {
    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        // A panic in another hook must not take the visuals down with it.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The instance folder as a path, from the value registration was given.
fn instance_path(dir: &mod_api::InstanceDirectory) -> Option<std::path::PathBuf> {
    match dir {
        #[cfg(unix)]
        mod_api::InstanceDirectory::Unix(bytes) => {
            use std::os::unix::ffi::OsStrExt;
            Some(std::ffi::OsStr::from_bytes(bytes).into())
        }
        #[cfg(windows)]
        mod_api::InstanceDirectory::Windows(units) => {
            use std::os::windows::ffi::OsStringExt;
            Some(std::ffi::OsString::from_wide(units).into())
        }
        _ => None,
    }
}

/// The companion pack, as it ships in this mod's assets.
const PACK: &[u8] = include_bytes!("../assets/enhancedvisuals/native-enhancedvisuals.hshader");
/// Its name in `shaderpacks/`: Horizon ties the `native-` prefix to this mod's id.
const PACK_NAME: &str = "native-enhancedvisuals.hshader";

/// Watches for the companion pack and puts it in `shaderpacks/` the first time, so the blur is
/// one pick away (**Options > Horizon > Shader Packs**) instead of a file to find and copy.
/// True once it is there, as the `.hshader` or as a folder of the same name.
fn install_pack(dir: &mod_api::InstanceDirectory) -> bool {
    let Some(root) = instance_path(dir) else {
        return false;
    };
    let packs = root.join("shaderpacks");
    if packs.join(PACK_NAME).is_file()
        || packs
            .join("native-enhancedvisuals")
            .join("horizon-shaders.toml")
            .is_file()
    {
        return true;
    }
    if std::fs::create_dir_all(&packs).is_err() {
        return false;
    }
    std::fs::write(packs.join(PACK_NAME), PACK).is_ok()
}

/// The index of `value` in one of the original's option lists.
fn choice_of(list: &[u32], value: u32) -> u32 {
    list.iter().position(|&v| v == value).unwrap_or(0) as u32
}

/// A toggle's translation key.
fn label(key: &str) -> String {
    format!("{ID}.setting.{key}")
}

#[mod_api::horizon_mod]
impl Hooks for EnhancedVisuals {
    /// The original's config screen, as settings on the mod's page in **Options > Mods**.
    fn register(&self, registrar: &mut dyn Registrar, context: &RegistrationContext) {
        // The companion pack is written out for the player the first time the game starts with
        // this mod in it (`install_pack`). Its presence picks the toggle's default: with the
        // pack there, the pack draws the blur and this mod's flat haze must stay off.
        let pack = context.instance_dir().map(install_pack).unwrap_or(false);
        let defaults = Options::default();
        let toggles = [
            ("blood", defaults.blood),
            ("hit_flash", defaults.hit_flash),
            ("pulse", defaults.pulse),
            ("sounds", defaults.sounds),
            ("rain", defaults.rain),
            ("shader_pack", !pack),
            ("death_messages", defaults.death_messages),
        ];
        for (key, default) in toggles {
            let _ = registrar.register_setting(SettingSpec::toggle(key, label(key), default));
        }
        let choices: [(&str, &[u32], u32); 4] = [
            ("amount", &AMOUNTS, choice_of(&AMOUNTS, defaults.amount)),
            (
                "duration",
                &DURATIONS,
                choice_of(&DURATIONS, defaults.duration),
            ),
            ("max", &CAPS, choice_of(&CAPS, defaults.max)),
            (
                "pulse_strength",
                &AMOUNTS,
                choice_of(&AMOUNTS, defaults.pulse_strength),
            ),
        ];
        for (key, list, default) in choices {
            let options: Vec<String> = list
                .iter()
                .map(|value| format!("{ID}.setting.{key}.{value}"))
                .collect();
            let _ = registrar.register_setting(SettingSpec::choice(
                key,
                label(key),
                options,
                default as usize,
            ));
        }
    }

    /// What the bundled shader pack reads each frame, in its `frame.mods[0]`: the radius of the
    /// original's blur in pixels (`ExplosionHandler`'s `maxBlur` over `explosionBlurTime`,
    /// `HeartbeatHandler`'s beats, `SplashHandler`'s water), as its own post chain got them, and
    /// `SaturationHandler`'s `Saturation`, 1.0 being an untouched picture. Asked while any pack is
    /// on; the pack that reads it is `native-enhancedvisuals`, this mod's own.
    fn shader_params(&self, _host: &dyn ClientHost, event: &mut ShaderParamsEvent<'_>) {
        // Written every frame, whatever the settings say: without the pack nothing reads them,
        // and with it the blur and the desaturation are the original's own.
        event.params[0] = effects::shader_strength(&self.state());
    }

    /// `DeathMessages`: a quip under "You died!", picked when that screen opens and drawn at
    /// y 114 in the middle, quoted and white, as `EVRenderer` drew it.
    fn screen_render_post(&self, _host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
        if event.screen.id != "dead" {
            // The original caches its quip for the whole session; a new death picks a new one.
            self.state().death_quip = None;
            return;
        }
        let quip = {
            let mut state = self.state();
            if !state.options.death_messages {
                return;
            }
            match state.death_quip {
                Some(quip) => quip,
                None => {
                    let quip = match effects::death_message(&mut state.rng) {
                        Some(quip) => quip,
                        None => return,
                    };
                    state.death_quip = Some(quip);
                    quip
                }
            }
        };
        let line = format!("\"{quip}\"");
        let width = event.draw.size()[0];
        // The original centred on the message's own width, the quotes hanging off either side.
        let at = width / 2.0 - event.draw.text_width(quip) / 2.0;
        event.draw.text(&line, [at, 114.0], 0xFFFF_FFFF, false);
    }

    /// The splatters and overlays go above every vanilla layer, so the hotbar and chat never
    /// cover them, and F1 hides them with the rest of the HUD.
    fn register_gui_layers(&self, _: &RegistrationContext, event: &mut RegisterGuiLayersEvent<'_>) {
        event.layers.push(GuiLayer {
            id: LAYER.to_owned(),
            placement: GuiLayerPlacement::AboveAll,
        });
    }

    /// One frame of the HUD: the overlay, then every splat above it.
    fn gui_layer_draw(&self, _: &dyn ClientHost, event: &mut GuiLayerDrawEvent<'_>) {
        if event.layer != LAYER {
            return;
        }
        let items: Vec<DrawItem> = {
            let state = self.state();
            effects::draw_list(&state)
        };
        if items.is_empty() {
            return;
        }
        let [width, height] = event.draw.size();
        for item in items {
            let dst = item.dest(width, height);
            if item.fill {
                event.draw.fill(dst, item.tint);
                continue;
            }
            // A sprite the resources do not have draws nothing; the frame still runs.
            let _ = event
                .draw
                .sprite(item.sprite, item.src, dst, Some(item.tint));
        }
    }

    /// The world names the damage type of a hit on a player; the next client tick that sees
    /// the health drop draws it.
    fn living_damage_pre(
        &self,
        _level: &mut dyn LevelRandomHost,
        event: &mut LivingDamagePreEvent<'_>,
    ) {
        if !event.kind.ends_with("player") {
            return;
        }
        self.state().push_hit(event.source.damage_type);
    }

    /// A blast near the player: dust and the ringing, scaled by its distance and by how much of
    /// the player the blast can see (`ServerExplosion.getSeenPercent`).
    fn explosion_prime(&self, _host: &mut dyn ServerHost, event: &mut ExplosionPrimeEvent) {
        // The blast's occlusion is a level ray, which only the client's host answers: the tick
        // that draws it resolves the blast.
        self.state().pending_blast = Some((event.center, event.radius));
    }

    /// A splash potion near the player: the original's potion overlay.
    fn potion_splash(&self, _host: &mut dyn ServerHost, event: &mut PotionSplashEvent<'_>) {
        let mut state = self.state();
        let distance = distance(state.pos, event.pos);
        // The splash's own colour is in the stack's item components, which the host does not
        // expose yet, so the overlay is the original's white.
        let rgb = 0xFF_FFFF;
        effects::potion(&mut state, distance, rgb);
    }

    /// The clock: age what is on screen, add a hit's blood, beat the heart, bring the rain.
    fn client_tick_post(&self, host: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        let input = read_input(host);
        let options = read_options(host);
        let blast = {
            let mut state = self.state();
            state.pending_blast.take().map(|(center, radius)| {
                let seen = host
                    .local_player()
                    .and_then(|player| player.level(host).map(|level| (player, level)))
                    .map_or(0.0, |(player, level)| {
                        seen_percent(host, level, player.entity(), center)
                    });
                (distance(input.pos, center), radius, seen)
            })
        };
        if let Some((distance, radius, seen)) = blast {
            if input.in_world {
                let mut state = self.state();
                effects::explosion(&mut state, distance, radius, seen);
            }
        }
        let (sounds, play) = {
            let mut state = self.state();
            state.options = options;
            let sounds = effects::client_tick(&mut state, &input);
            let play = state.options.sounds;
            (sounds, play)
        };
        if !play {
            return;
        }
        for sound in sounds {
            let mut event = SoundPlay::ui(sound.event()).with_pitch(sound.pitch());
            if let Sound::Ring(ticks) = sound {
                // `SoundMuteHandler`: every other sound's volume is taken away over
                // `explosionSoundTime` (`setMuteVolume(1 - factor)`, a volume, no filter), and the
                // ringing starts at `maxExplosionVolume` (0.5) fading on the same curve.
                event = event.with_volume(0.5);
                host.muffle_sounds(1.0, 0.0, ticks, &["enhancedvisuals:ringing"]);
                if let Some(played) = host.play_local_sound(&event, 0) {
                    played.fade(host, 0.0, ticks);
                }
                continue;
            }
            let _ = host.play_local_sound(&event, 0);
        }
    }
}

/// `ServerExplosion.getSeenPercent`: the share of a 3x3x3 grid over the player's box that a ray from
/// the blast's centre reaches without hitting a block.
fn seen_percent(
    host: &dyn ClientHost,
    level: LevelRef,
    entity: EntityRef,
    center: [f64; 3],
) -> f32 {
    let Some([min, max]) = entity.bounding_box(host) else {
        return 0.0;
    };
    let step = |lo: f64, hi: f64| {
        let extent = hi - lo;
        if extent <= 0.0 {
            1.0
        } else {
            1.0 / (extent * 2.0 + 1.0)
        }
    };
    let (dx, dy, dz) = (
        step(min[0], max[0]),
        step(min[1], max[1]),
        step(min[2], max[2]),
    );
    let mut seen = 0usize;
    let mut samples = 0usize;
    let mut x = 0.0f64;
    while x <= 1.0 {
        let mut y = 0.0f64;
        while y <= 1.0 {
            let mut z = 0.0f64;
            while z <= 1.0 {
                let at = [
                    min[0] + (max[0] - min[0]) * x,
                    min[1] + (max[1] - min[1]) * y,
                    min[2] + (max[2] - min[2]) * z,
                ];
                samples += 1;
                let blocked = level
                    .clip(host, at, center, ClipBlock::Collider, ClipFluid::None)
                    .is_some_and(|hit| !hit.miss);
                if !blocked {
                    seen += 1;
                }
                z += dz;
            }
            y += dy;
        }
        x += dx;
    }
    if samples == 0 {
        0.0
    } else {
        seen as f32 / samples as f32
    }
}

/// How far `at` is from `from`, in blocks.
fn distance(from: [f64; 3], at: [f64; 3]) -> f32 {
    let [dx, dy, dz] = [at[0] - from[0], at[1] - from[1], at[2] - from[2]];
    ((dx * dx + dy * dy + dz * dz).sqrt()) as f32
}

/// The settings the player chose on the mod's page, with the original's defaults.
fn read_options(host: &dyn ClientHost) -> Options {
    let defaults = Options::default();
    Options::from_choices(
        host.setting_bool("blood").unwrap_or(defaults.blood),
        host.setting_bool("hit_flash").unwrap_or(defaults.hit_flash),
        host.setting_choice("amount")
            .unwrap_or_else(|| choice_of(&AMOUNTS, defaults.amount)),
        host.setting_choice("duration")
            .unwrap_or_else(|| choice_of(&DURATIONS, defaults.duration)),
        host.setting_choice("max")
            .unwrap_or_else(|| choice_of(&CAPS, defaults.max)),
        host.setting_bool("pulse").unwrap_or(defaults.pulse),
        host.setting_choice("pulse_strength")
            .unwrap_or_else(|| choice_of(&AMOUNTS, defaults.pulse_strength)),
        host.setting_bool("sounds").unwrap_or(defaults.sounds),
        host.setting_bool("rain").unwrap_or(defaults.rain),
        host.setting_bool("shader_pack")
            .unwrap_or(defaults.shader_pack),
        host.setting_bool("death_messages")
            .unwrap_or(defaults.death_messages),
    )
}

/// What one client tick reads: the local player's health, position and look, and the weather of
/// the level they are in.
fn read_input(host: &dyn ClientHost) -> Input {
    let mut input = Input {
        in_world: false,
        pos: [0.0; 3],
        health: FALLBACK_MAX_HEALTH,
        max_health: FALLBACK_MAX_HEALTH,
        rain: 0.0,
        rain_visible: false,
        pitch: 0.0,
        creative: false,
        on_ground: false,
        on_sand: false,
        sprinting: false,
        food: 20,
        eyes_in_water: false,
        enderman: None,
    };
    if !host.playing() {
        return input;
    }
    let Some(player) = host.local_player() else {
        return input;
    };
    // A spectator sees none of it, as in the original.
    if player.is_spectator(host).unwrap_or(false) {
        return input;
    }
    let entity = player.entity();
    input.in_world = true;
    if let Some(living) = host.living(entity) {
        input.health = living.health(host).unwrap_or(FALLBACK_MAX_HEALTH);
        input.max_health = living
            .max_health(host)
            .unwrap_or(FALLBACK_MAX_HEALTH)
            .max(1.0);
    }
    input.pos = entity.position(host).unwrap_or([0.0; 3]);
    if let Some([_, pitch]) = entity.rotation(host) {
        input.pitch = pitch;
    }
    // `doEffectsInCreative` is false in the original's config.
    input.creative = player.is_creative(host).unwrap_or(false);
    input.on_ground = entity.on_ground(host).unwrap_or(false);
    input.sprinting = entity.is_sprinting(host).unwrap_or(false);
    // `Entity.wasEyeInWater`, through the fluid the entity's eyes last saw.
    input.eyes_in_water = entity
        .eye_fluid(host)
        .flatten()
        .is_some_and(|fluid| fluid == "minecraft:water");
    input.food = player.food(host).map_or(20, |food| food.level);
    if let Some(level) = player.level(host) {
        if let Some((rain, _thunder)) = level.weather(host) {
            input.rain = rain;
        }
        // `Level.isRainingAt` at the feet, or at the top of the player's box.
        let feet = [
            input.pos[0].floor() as i32,
            input.pos[1].floor() as i32,
            input.pos[2].floor() as i32,
        ];
        let top = entity
            .bounding_box(host)
            .map_or(feet[1], |[_, max]| max[1].floor() as i32);
        let head = [feet[0], top, feet[2]];
        input.rain_visible = level.is_raining_at(host, feet).unwrap_or(false)
            || level.is_raining_at(host, head).unwrap_or(false);
        // `SandSplatHandler.isOnSand`: the block below the player is in `minecraft:sand`.
        let below = [
            input.pos[0].floor() as i32,
            (input.pos[1] - 0.1).floor() as i32,
            input.pos[2].floor() as i32,
        ];
        input.on_sand = level
            .block(host, below)
            .and_then(|block| host.block_in_tag(block, "minecraft:sand"))
            .unwrap_or(false);
        input.enderman = nearby_enderman(host, level, input.pos);
    }
    input
}

/// `SlenderHandler`'s scan: the closest enderman within 16 blocks of the player, as a distance.
fn nearby_enderman(host: &dyn ClientHost, level: LevelRef, pos: [f64; 3]) -> Option<f32> {
    let box_min = [pos[0] - 16.0, pos[1] - 16.0, pos[2] - 16.0];
    let box_max = [pos[0] + 16.0, pos[1] + 16.0, pos[2] + 16.0];
    let found = level.entities_in(host, box_min, box_max)?;
    let mut nearest: Option<f32> = None;
    for entity in found {
        // The API names no entity types, so an enderman is recognised by its shape: 2.9 blocks
        // tall and half a block wide, which no other vanilla mob is.
        let Some([min, max]) = entity.bounding_box(host) else {
            continue;
        };
        let (tall, wide) = (max[1] - min[1], max[0] - min[0]);
        if !(tall >= 2.7 && wide <= 0.9) {
            continue;
        }
        let Some(at) = entity.position(host) else {
            continue;
        };
        let distance = distance(pos, at);
        if nearest.is_none_or(|best| distance < best) {
            nearest = Some(distance);
        }
    }
    nearest
}
