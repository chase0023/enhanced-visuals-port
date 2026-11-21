//! The mod's own logic: what a hit draws, how long it stays, when the heart beats, and which
//! cell of which texture is shown. No Horizon types are used here, so `cargo test` runs it
//! without the game.
//!
//! Ported from EnhancedVisuals 1.8.35 (CreativeMD, AriaFreeze; NeoForge credit Sonicjumper),
//! LGPL-3.0-only. The numbers are the original's: `DamageHandler`, `HeartbeatHandler`,
//! `RainHandler`, `ExplosionHandler`, `PotionHandler` and `EVRenderer`.

use crate::sprites::{self, Sprite};

/// The original's default max health, used when the attribute cannot be read.
pub const FALLBACK_MAX_HEALTH: f32 = 20.0;
/// GUI height fraction of a blood splatter (original: 15% of the window height).
pub const SPLAT_HEIGHT: f32 = 0.15;
/// The original's random size range of one particle.
/// The original's blood lifetime in ticks: 25 to 75 seconds, scaled by the duration setting.
pub const BLOOD_TICKS_MIN: i32 = 500;
pub const BLOOD_TICKS_MAX: i32 = 1500;
/// The original's blood colour, `0.3, 0.01, 0.01`. Its colour's own 0.7 alpha never reaches the
/// screen: `VisualTypeTextureClient` draws `rgba(r, g, b, opacity * 255)`, so the alpha is the
/// visual's fading opacity alone.
pub const BLOOD_RGB: u32 = 0x4D_0303;
/// The alpha every particle is drawn at before it fades: the original's `opacity * 255`, full.
pub const FULL_ALPHA: f32 = 1.0;
/// The original's heartbeat: below 6 health, the buffer `health / max * 100 + 15`, the out beat
/// at 0 and the in beat at 5, the overlay 20 ticks.
pub const HB_MAX_HEALTH: f32 = 6.0;
pub const HB_TIME_FACTOR: f32 = 100.0;
pub const HB_MIN_LENGTH: i32 = 15;
pub const HB_OVERLAY_INTENSITY: f32 = 0.5;
pub const HB_OVERLAY_TICKS: i32 = 20;
/// `HeartbeatHandler.heartbeatBlurIntensity`: each beat blurs by `min(0.7, intensity) × 50` px.
pub const HB_BLUR_PEAK: f32 = 50.0;
/// `HeartbeatHandler.heartbeatBlurDuration`: the beat's blur fades over 5 ticks.
pub const HB_BLUR_TICKS: i32 = 5;
/// `SplashHandler.intensity`: eyes entering or leaving water blur by 5 to 10 px,
pub const SPLASH_BLUR: (f32, f32) = (5.0, 10.0);
/// over `SplashHandler.duration`: 10 ticks.
pub const SPLASH_BLUR_TICKS: i32 = 10;
/// Elemental splashes fade over five seconds, like the original's short curves.
/// One tick in milliseconds: what the frame clock of an animated texture counts.
pub const MS_PER_TICK: u32 = 50;
/// No more than this many particles at once, whatever the settings say.
pub const HARD_LIMIT: usize = 700;

/// The rotation cells `tools/gen_assets.py` stored per texture: 0, 45, 90 and 135 degrees.

// -------------------------------------------------------------------------------------------
// Settings. The original's config screen, in its order and with its value lists.

pub const AMOUNTS: [u32; 6] = [25, 50, 75, 100, 150, 200];
pub const DURATIONS: [u32; 5] = [25, 50, 100, 150, 200];
pub const CAPS: [u32; 4] = [16, 32, 48, 96];

/// What the player chose on the mod's page in **Options > Mods**.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub blood: bool,
    /// The original's `hitEffectIntensity`: a red flash on every hit, off by default there.
    pub hit_flash: bool,
    /// `hitEffectIntensity`, in percent: the overlay's peak is this times 0.2.
    pub hit_flash_intensity: u32,
    /// Percent of the original's particle count.
    pub amount: u32,
    /// Percent of the original's lifetime.
    pub duration: u32,
    /// Most particles one hit may add.
    pub max: u32,
    pub pulse: bool,
    pub pulse_strength: u32,
    pub sounds: bool,
    pub rain: bool,
    /// The bundled shader pack draws the blur and the desaturation, instead of this mod's flat
    /// washes over the HUD. Off until the player picks that pack: without it there would be no
    /// blur at all.
    pub shader_pack: bool,
    /// `DeathMessages.enabled`: the quip under "You died!".
    pub death_messages: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            blood: true,
            hit_flash: false,
            hit_flash_intensity: 100,
            amount: 100,
            duration: 100,
            max: 48,
            pulse: true,
            pulse_strength: 100,
            sounds: true,
            rain: true,
            shader_pack: false,
            death_messages: true,
        }
    }
}

impl Options {
    /// A value of `list` by the index the settings kept, falling back to `index` 0 of the
    /// original's default when the choice is out of range.
    fn pick(list: &[u32], choice: u32, default: usize) -> u32 {
        list.get(choice as usize)
            .copied()
            .unwrap_or(list[default.min(list.len().saturating_sub(1))])
    }

    /// The settings the host read, as indices of [`AMOUNTS`], [`DURATIONS`] and [`CAPS`].
    #[allow(clippy::too_many_arguments)]
    pub fn from_choices(
        blood: bool,
        hit_flash: bool,
        amount: u32,
        duration: u32,
        max: u32,
        pulse: bool,
        pulse_strength: u32,
        sounds: bool,
        rain: bool,
        shader_pack: bool,
        death_messages: bool,
    ) -> Self {
        let default = Self::default();
        Self {
            blood,
            hit_flash,
            hit_flash_intensity: default.hit_flash_intensity,
            amount: Self::pick(
                &AMOUNTS,
                amount,
                AMOUNTS
                    .iter()
                    .position(|&v| v == default.amount)
                    .unwrap_or(0),
            ),
            duration: Self::pick(
                &DURATIONS,
                duration,
                DURATIONS
                    .iter()
                    .position(|&v| v == default.duration)
                    .unwrap_or(0),
            ),
            max: Self::pick(
                &CAPS,
                max,
                CAPS.iter().position(|&v| v == default.max).unwrap_or(0),
            ),
            pulse,
            pulse_strength: Self::pick(
                &AMOUNTS,
                pulse_strength,
                AMOUNTS
                    .iter()
                    .position(|&v| v == default.pulse_strength)
                    .unwrap_or(0),
            ),
            sounds,
            rain,
            shader_pack,
            death_messages,
        }
    }
}

// -------------------------------------------------------------------------------------------
// Random numbers: xorshift64*, because the original's `java.util.Random` is not ours to borrow.

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `0.0..1.0`.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }

    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % ((i64::from(hi) - i64::from(lo) + 1) as u64)) as i32
    }

    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }

    /// An index in `0..n`; 0 for an empty list.
    pub fn pick(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
}

// -------------------------------------------------------------------------------------------
// Geometry and colours.

/// The original's placement rule: `f = 1 - rand^2`, the centre pushed to either side by `f`,
/// so about 71% of the particles land in the outer half. Answers a fraction of the screen.
pub fn biased_centre(rng: &mut Rng) -> f32 {
    let f = 1.0 - rng.next_f32().powi(2);
    if rng.range_i32(0, 1) == 0 {
        (0.5 + 0.5 * f).min(1.0)
    } else {
        (0.5 - 0.5 * f).max(0.0)
    }
}

/// A `0xRRGGBB` colour and an alpha fraction as the `0xAARRGGBB` a draw call takes.
pub fn argb(rgb: u32, alpha: f32) -> u32 {
    let a = (alpha.clamp(0.0, 1.0) * 255.0).round() as u32;
    (a << 24) | (rgb & 0x00FF_FFFF)
}

// -------------------------------------------------------------------------------------------
// What the original drew for each kind of damage.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Splatter,
    Slash,
    Impact,
    Pierce,
    Blob,
    Fire,
    Ice,
    Wither,
    Shock,
    Dust,
    Drop,
    Heat,
    Break,
    Parasite,
    Sand,
}

impl Kind {
    /// The textures this kind picks from, one at random per particle.
    pub fn sprites(self) -> &'static [Sprite] {
        match self {
            Kind::Splatter => sprites::SPLATTER,
            Kind::Slash => sprites::SLASH,
            Kind::Impact => sprites::IMPACT,
            Kind::Pierce => sprites::PIERCE,
            Kind::Blob => sprites::BLOB,
            Kind::Fire => sprites::FIRE,
            Kind::Ice => sprites::ICE,
            Kind::Wither => sprites::WITHER,
            Kind::Shock => sprites::SHOCK,
            Kind::Dust => sprites::DUST,
            Kind::Drop => sprites::DROP,
            Kind::Heat => sprites::HEAT,
            Kind::Break => sprites::BREAK,
            Kind::Parasite => sprites::PARASITE,
            Kind::Sand => sprites::SAND,
        }
    }

    /// The original's tint: `VisualTypeParticleColored`'s colour, or white for the plain
    /// `VisualTypeParticle`s and the colour the handler passes for fire (black).
    pub fn rgb(self) -> u32 {
        match self {
            Kind::Splatter | Kind::Slash | Kind::Impact | Kind::Pierce => BLOOD_RGB,
            // `new Color(0, 0, 255)`
            Kind::Blob => 0x00_00FF,
            // Fire is drawn with the handler's own `new Color(0, 0, 0)`.
            Kind::Fire | Kind::Wither => 0x00_0000,
            Kind::Heat | Kind::Break | Kind::Sand => 0xFF_FFFF,
            // `new Color(200, 255, 255)`
            Kind::Ice => 0xC8_FFFF,
            // `new Color(120, 120, 255)`
            Kind::Shock => 0x78_78FF,
            Kind::Dust => 0xFF_FFFF,
            // `new Color(41, 76, 149)`
            Kind::Drop => 0x29_4C95,
            // `new Color(0, 126, 0)`
            Kind::Parasite => 0x00_7E00,
        }
    }

    /// Blood keeps the original's lifetime and 0.7 alpha; the rest are short splashes.
    pub fn is_blood(self) -> bool {
        matches!(
            self,
            Kind::Splatter | Kind::Slash | Kind::Impact | Kind::Pierce
        )
    }

    /// Every kind is drawn at the opacity alone (`rgba(r, g, b, opacity * 255)`), so blood is
    /// as solid as the rest until it fades.
    pub fn base_alpha(self) -> f32 {
        FULL_ALPHA
    }

    /// The original's `scale` range: `VisualTypeParticle`'s default 0.2 to 1.0, sand and rain
    /// drops 0.1 to 0.5, wither's own 1.5 to 2.5.
    pub fn scale(self) -> (f32, f32) {
        match self {
            Kind::Sand | Kind::Drop => (0.1, 0.5),
            Kind::Wither => (1.5, 2.5),
            _ => (0.2, 1.0),
        }
    }

    /// How many splashes one hit makes and how long they last, in ticks, as the handler's
    /// own config has them. Blood comes from its damage instead (see [`blood_count`]).
    pub fn splash(self) -> (usize, i32, i32) {
        match self {
            // fireSplashes 1, fireDuration 100 to 1000
            Kind::Fire => (1, 100, 1000),
            // lightningSplashes 10, lightningDuration 10 to 15
            Kind::Shock => (10, 10, 15),
            // effectSplashes 4, effectDuration 10 to 15 (wither uses the same count)
            Kind::Wither | Kind::Parasite | Kind::Heat | Kind::Blob | Kind::Ice | Kind::Break => {
                (4, 10, 15)
            }
            // ExplosionHandler: dustAmount 0 to 30 over damage 0 to 20, dustDuration 500 to 1000
            Kind::Dust => (0, 500, 1000),
            // RainHandler: duration 100
            Kind::Drop => (1, 100, 100),
            // SandSplatHandler: duration 100, one per tick at most
            Kind::Sand => (1, 100, 100),
            _ => (1, BLOOD_TICKS_MIN, BLOOD_TICKS_MAX),
        }
    }

    /// The original marks the drowning, lightning, wither and parasite splashes `ignoreWater`: they
    /// are not faded faster with the eyes under water.
    pub fn ignore_water(self) -> bool {
        matches!(
            self,
            Kind::Blob | Kind::Shock | Kind::Wither | Kind::Parasite
        )
    }

    /// `VisualTypeParticleColored`'s `heat` is the one kind the original never turns.
    pub fn can_rotate(self) -> bool {
        self != Kind::Heat
    }
}

/// The full-screen overlays the original fades in and out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayKind {
    LowHealth,
    Tunnel,
    Potion,
    Damaged,
}

impl OverlayKind {
    pub fn sprite(self) -> &'static str {
        match self {
            OverlayKind::LowHealth => sprites::overlay::LOW_HEALTH,
            OverlayKind::Tunnel => sprites::overlay::TUNNEL,
            OverlayKind::Potion => sprites::overlay::POTION,
            OverlayKind::Damaged => sprites::overlay::DAMAGED,
        }
    }
}

/// The damage types the client is told about, mapped as the original's `DamageHandler` mapped
/// them: first match wins, and anything unknown is its default splatter. Every branch that turns
/// on the attacker's held item or species needs the attacker, so it is mapped to the shape such
/// a hit draws most often (swords are the sharp list's common case).
pub fn classify(damage_type: &str) -> (Kind, Option<OverlayKind>) {
    let t = damage_type
        .strip_prefix("minecraft:")
        .unwrap_or(damage_type);
    if t.contains("parasites") {
        return (Kind::Parasite, None);
    }
    if t.contains("hyperthermia") {
        return (Kind::Heat, None);
    }
    match t {
        // An `Arrow`'s `directEntity` is the pierce branch.
        "arrow" | "spectral_arrow" | "tipped_arrow" => (Kind::Pierce, None),
        // `DamageTypes.CACTUS`
        "cactus" => (Kind::Pierce, None),
        // `DamageTypeTags.IS_FALL`
        "fall" | "falling_block" | "falling_anvil" | "falling_stalactite" => (Kind::Impact, None),
        // `DamageTypeTags.IS_DROWNING`
        "drown" => (Kind::Blob, None),
        // `DamageTypeTags.IS_FREEZING`
        "freeze" => (Kind::Ice, None),
        // `DamageTypes.WITHER` and `WITHER_SKULL`
        "wither" | "wither_skull" => (Kind::Wither, None),
        // `DamageTypeTags.IS_LIGHTNING`
        "lightning_bolt" => (Kind::Shock, None),
        // `DamageTypes.FLY_INTO_WALL`
        "fly_into_wall" => (Kind::Break, None),
        // `DamageTypeTags.IS_FIRE` and `DamageTypes.ON_FIRE`
        "in_fire" | "on_fire" | "lava" | "hot_floor" | "campfire" | "fireball"
        | "unattended_fire" | "sweet_berry_bush" | "fireworks" => (Kind::Fire, None),
        // `DamageTypes.STARVE` and the addons' dehydration: the tunnel overlay, no splashes.
        "starve" => (Kind::Splatter, Some(OverlayKind::Tunnel)),
        // A living attacker: the original picks the shape from the weapon in its hand and, with
        // none, from its species. A sword is the common case, so a melee hit draws the slash.
        "player_attack" | "sweep_attack" | "mob_attack" | "mob_attack_no_aggro" => {
            (Kind::Slash, None)
        }
        // `thorns` has no direct entity, a bee's sting and the warden's boom have no weapon in
        // hand and are not one of the species the original names, and a mace is in none of its
        // three item lists: all four fall to the handler's last branch, the splatter.
        "thorns" | "sting" | "sonic_boom" | "mace_smash" => (Kind::Splatter, None),
        _ => (Kind::Splatter, None),
    }
}

// -------------------------------------------------------------------------------------------
// Live state.

/// One splat on the screen. Positions and sizes are screen fractions, so a window resize keeps
/// the splatters where they were.
#[derive(Clone, Debug)]
pub struct Particle {
    pub sprite: &'static Sprite,
    /// Which stored angle this one shows, `0..sprite.angles`.
    pub angle: u8,
    /// The frame a group without a frame clock keeps, as the original's random variant.
    pub variant: u16,
    /// Centre, `0..1` of the GUI.
    pub x: f32,
    pub y: f32,
    /// Side as a fraction of the GUI height, before the texture's rotation padding.
    pub height: f32,
    pub age: f32,
    pub life: f32,
    pub rgb: u32,
    pub alpha: f32,
    /// `ignoreWater` in the original: faded ten times as fast under water otherwise.
    pub ignore_water: bool,
}

impl Particle {
    /// The alpha the splat is drawn at now: the original fades linearly to nothing.
    pub fn opacity(&self) -> f32 {
        if self.life <= 0.0 {
            return 0.0;
        }
        (1.0 - self.age / self.life).clamp(0.0, 1.0) * self.alpha
    }

    /// The animation frame the splat shows now. The original reads the wall clock
    /// (`TextureCacheAnimation`: `System.nanoTime / 3000000 / speed`), so every particle of a
    /// kind shows the same frame at the same moment, whenever it was made; `clock_ms` is that
    /// clock, in milliseconds since the mod's first tick.
    pub fn frame(&self, clock_ms: u32) -> u16 {
        self.sprite.frame_at(clock_ms, self.variant)
    }

    /// The cell of the frame and angle the splat shows now.
    pub fn cell(&self, clock_ms: u32) -> [f32; 4] {
        self.sprite
            .cell_of(self.frame(clock_ms), u16::from(self.angle))
    }

    /// The texture the splat draws from now.
    pub fn name(&self, clock_ms: u32) -> &'static str {
        self.sprite
            .name_of(self.frame(clock_ms), u16::from(self.angle))
    }
}

/// One full-screen overlay, fading from `peak` to nothing over `life` ticks.
#[derive(Clone, Debug, PartialEq)]
pub struct Overlay {
    pub kind: OverlayKind,
    pub age: f32,
    pub life: f32,
    pub peak: f32,
    /// The original's `visual.color`: the potion's own colour, white for the rest.
    pub rgb: u32,
}

impl Overlay {
    pub fn opacity(&self) -> f32 {
        if self.life <= 0.0 {
            return 0.0;
        }
        (1.0 - self.age / self.life).clamp(0.0, 1.0) * self.peak
    }
}

/// A hit the world announced, waiting for the client tick that sees the health drop: its
/// damage type as the visuals for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hit {
    pub kind: Kind,
    pub overlay: Option<OverlayKind>,
    /// The starvation tunnel is the one hit the original draws without splashes.
    pub splashes: bool,
}

/// The original's `HeartbeatHandler`: below 6 health the buffer counts down each tick, the out
/// beat at 0 and the in beat at 5, its length `health / max * 100 + 15`.
#[derive(Clone, Debug, Default)]
pub struct Heartbeat {
    buffer: i32,
}

impl Heartbeat {
    /// How strong the red pulse is at this health, 0 at 6 health and above.
    pub fn intensity(health: f32, max_health: f32) -> f32 {
        if max_health <= 0.0 {
            return 0.0;
        }
        ((HB_MAX_HEALTH - health) / max_health * 2.0).max(0.0)
    }

    /// One tick: `Some(true)` on the out beat, `Some(false)` on the in beat, `None` between.
    pub fn tick(&mut self, health: f32, max_health: f32) -> Option<bool> {
        let mut beat = None;
        if health < HB_MAX_HEALTH && health > 0.0 {
            if self.buffer <= 0 {
                self.buffer = (health / max_health * HB_TIME_FACTOR + HB_MIN_LENGTH as f32) as i32;
                beat = Some(true);
            } else if self.buffer == 5 {
                beat = Some(false);
            }
        }
        self.buffer = self.buffer.saturating_sub(1);
        beat
    }

    /// The overlay's alpha at the out beat.
    pub fn peak(health: f32, max_health: f32, strength: u32) -> f32 {
        let k = Self::intensity(health, max_health).min(0.7);
        k * HB_OVERLAY_INTENSITY * (strength as f32 / 100.0)
    }
}

/// `DeathMessages.deathMessages`, in the original's order: what it says under "You died!".
pub const DEATH_MESSAGES: [&str; 9] = [
    "Do you really want to respawn? think of it again.",
    "Life is hard. Deal with it!",
    "You are dead ... wait you already know that.",
    "Did I let the stove on...?",
    "Should have shot back first...",
    "Yep, that's messed up...",
    "Rage incomming!",
    "I think you dropped something.",
    "Time for a break?",
];

/// `DeathMessages.pickRandomDeathMessage`: a message, never the last of the list when there are
/// more than one (`nextInt(size - 1)`, as the original draws it), `None` for an empty list.
pub fn death_message(rng: &mut Rng) -> Option<&'static str> {
    match DEATH_MESSAGES.len() {
        0 => None,
        1 => Some(DEATH_MESSAGES[0]),
        n => Some(DEATH_MESSAGES[rng.pick(n - 1)]),
    }
}

/// The original's `healthScaler`: 3.0 at 0 health, down to 1.5 at 12 and above.
pub fn health_scaler(x: f32) -> f32 {
    if x <= 0.0 {
        3.0
    } else if x >= 12.0 {
        1.5
    } else {
        3.0 - x / 12.0 * 1.5
    }
}

/// The original's blood count: `min(5000, (int)(damage * healthScaler(health)))`, then the
/// amount setting.
pub fn blood_count(health: f32, damage: f32, amount_percent: u32) -> usize {
    if !damage.is_finite() || damage <= 0.0 || !health.is_finite() {
        return 0;
    }
    let base = ((damage * health_scaler(health)) as i32).clamp(0, 5000);
    (base as f32 * amount_percent as f32 / 100.0)
        .round()
        .max(0.0) as usize
}

/// What the mod keeps between hooks. One `Mutex<State>` in the mod: the client's hooks and, in
/// single player, the world's server hooks both reach it.
#[derive(Clone, Debug)]
pub struct State {
    pub options: Options,
    pub visuals: Vec<Particle>,
    pub overlay: Option<Overlay>,
    pub health: f32,
    pub max_health: f32,
    last_health: Option<f32>,
    pub hb: Heartbeat,
    hits: Vec<Hit>,
    pub rng: Rng,
    pub tick: u64,
    /// Sounds the server side asked for; the next client tick plays them.
    pub pending: Vec<Sound>,
    /// The local player's position, kept for the world's explosion and potion hooks.
    pub pos: [f64; 3],
    /// A blast the world announced, resolved on the next client tick (its occlusion is a ray).
    pub pending_blast: Option<([f64; 3], f32)>,
    /// Milliseconds since the first tick: the clock the original's frame animations keep, which
    /// every particle of a kind shares.
    pub clock_ms: u32,
    /// The quip the death screen shows, picked when that screen opened.
    pub death_quip: Option<&'static str>,
    /// The original's blur visuals alive this frame (`ExplosionHandler`, `HeartbeatHandler`,
    /// `SplashHandler`): each is a post chain of its own, so their radii add up.
    pub blurs: Vec<Blur>,
    /// `SplashHandler.wasInWater`: the eyes' last water state, so a change is a splash.
    pub was_in_water: bool,
    /// `SaturationHandler`'s desaturation, tracked towards the food level's aim.
    pub desat: f32,
    /// `SlenderHandler`'s overlay opacity, from the nearest enderman.
    pub slender: f32,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub const fn new() -> Self {
        Self {
            options: Options {
                shader_pack: false,
                death_messages: true,
                blood: true,
                hit_flash: false,
                hit_flash_intensity: 100,
                amount: 100,
                duration: 100,
                max: 48,
                pulse: true,
                pulse_strength: 100,
                sounds: true,
                rain: true,
            },
            visuals: Vec::new(),
            overlay: None,
            health: FALLBACK_MAX_HEALTH,
            max_health: FALLBACK_MAX_HEALTH,
            last_health: None,
            hb: Heartbeat { buffer: 0 },
            hits: Vec::new(),
            rng: Rng::new(0x5EED_1E55),
            tick: 0,
            pending: Vec::new(),
            pos: [0.0; 3],
            pending_blast: None,
            clock_ms: 0,
            death_quip: None,
            blurs: Vec::new(),
            was_in_water: false,
            // `SaturationHandler` adds its visual with the opacity set to 1.0 and fades from there,
            // and 1.0 is the `Saturation` a normal picture has.
            desat: 1.0,
            slender: 0.0,
        }
    }

    /// A hit the world announced, waiting for the client tick that sees the health drop. The
    /// world's entity ids are its own, so the newest waiting hit is the one a drop belongs to.
    pub fn push_hit(&mut self, damage_type: &str) {
        let (kind, overlay) = classify(damage_type);
        if self.hits.len() >= 8 {
            self.hits.remove(0);
        }
        self.hits.push(Hit {
            kind,
            overlay,
            splashes: overlay != Some(OverlayKind::Tunnel),
        });
    }

    /// The hit to draw for a health drop: the announced one, else the original's default
    /// splatter (a client joining someone else's server never hears the world's side).
    fn take_hit(&mut self) -> Hit {
        self.hits.pop().unwrap_or(Hit {
            kind: Kind::Splatter,
            overlay: None,
            splashes: true,
        })
    }

    pub fn clear(&mut self) {
        self.visuals.clear();
        self.overlay = None;
        self.hits.clear();
        self.blurs.clear();
        self.was_in_water = false;
        self.pending_blast = None;
        self.last_health = None;
    }
}

/// What one client tick reads from the game.
#[derive(Clone, Copy, Debug)]
pub struct Input {
    /// A world is open and the player is not in a menu-only state.
    pub in_world: bool,
    pub pos: [f64; 3],
    pub health: f32,
    pub max_health: f32,
    /// Vanilla's rain level, `0..1`.
    pub rain: f32,
    /// `Level.isRainingAt` at the player's feet, or at the top of their box.
    pub rain_visible: bool,
    /// The player's pitch in degrees.
    pub pitch: f32,
    /// `doEffectsInCreative` is off in the original, so a creative player sees none of it.
    pub creative: bool,
    pub on_ground: bool,
    /// The block below the player is in `minecraft:sand`.
    pub on_sand: bool,
    pub sprinting: bool,
    /// The food level, 0 to 20 (`SaturationHandler`'s desaturation).
    pub food: i32,
    /// The eyes are under water: particles the kind does not exempt fade ten times as fast.
    pub eyes_in_water: bool,
    /// The nearest enderman's distance, when one is within the handler's 16 blocks.
    pub enderman: Option<f32>,
}

/// What the bundled shader pack draws this frame, as `frame.mods[0]`: the blur's radius in
/// pixels, exactly as the original hands its own post chain (`max(1, floor(opacity))` of each
/// visual, and a chain per visual, so they add up), and the `Saturation` its `color_convolve`
/// pass gets, 1.0 being an untouched picture.
pub fn shader_strength(state: &State) -> [f32; 4] {
    [blur_radius(state), state.desat.clamp(0.0, 1.0), 0.0, 0.0]
}

/// The radius of everything blurring the frame, in the original's pixels: each visual's peak
/// fading linearly to nothing over its life.
pub fn blur_radius(state: &State) -> f32 {
    let total: f32 = state
        .blurs
        .iter()
        .map(|blur| blur.peak * (1.0 - (blur.age / blur.life).clamp(0.0, 1.0)))
        .sum();
    if total <= 0.0 {
        0.0
    } else {
        // `VisualTypeBlurClient.changeProperties`: `Math.max(1.0f, Math.floor(intensity))`.
        total.floor().max(1.0)
    }
}

/// One blur the original would have given its own post chain.
#[derive(Clone, Copy, Debug)]
pub struct Blur {
    /// What it starts at, in pixels of the world's picture.
    pub peak: f32,
    pub age: f32,
    pub life: f32,
}

/// Adds a blur: `peak` pixels fading linearly to nothing over `life` ticks, as
/// `DecimalCurve(0.0, peak, life, 0.0)` does.
fn add_blur(state: &mut State, peak: f32, life: f32) {
    if peak <= 0.0 || life <= 0.0 {
        return;
    }
    // A whole frame at most: a burst of blasts cannot make the queue unbounded.
    if state.blurs.len() >= 8 {
        state.blurs.remove(0);
    }
    state.blurs.push(Blur {
        peak,
        age: 0.0,
        life,
    });
}

/// `HeartbeatHandler`'s beat: `min(0.7, intensity) × heartbeatBlurIntensity` pixels.
fn beat_blur(health: f32, max_health: f32) -> f32 {
    Heartbeat::intensity(health, max_health).min(0.7) * HB_BLUR_PEAK
}

/// A sound the tick asks the game to play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    HeartOut,
    HeartIn,
    /// The blast's ringing, over the ticks the original mutes everything else for.
    Ring(u32),
}

impl Sound {
    /// The sound event (`sounds.json` of this mod, the original's own files).
    pub fn event(self) -> &'static str {
        match self {
            Sound::HeartOut => "enhancedvisuals:heartbeatout",
            Sound::HeartIn => "enhancedvisuals:heartbeatin",
            Sound::Ring(_) => "enhancedvisuals:ringing",
        }
    }

    /// The pitch it plays at.
    pub fn pitch(self) -> f32 {
        match self {
            Sound::HeartOut => 1.0,
            Sound::HeartIn => 1.15,
            Sound::Ring(_) => 1.0,
        }
    }
}

/// One client tick: age what is on screen, add what the state asks for, and answer the sounds
/// to play now.
pub fn client_tick(state: &mut State, input: &Input) -> Vec<Sound> {
    let mut sounds = std::mem::take(&mut state.pending);
    state.tick = state.tick.wrapping_add(1);
    state.clock_ms = state.clock_ms.wrapping_add(MS_PER_TICK);

    // `waterSubstractFactor` is 10 in the original: with the eyes under water, every particle
    // the kind does not exempt ages ten ticks a tick.
    let water = if input.eyes_in_water { 10.0 } else { 1.0 };
    for particle in &mut state.visuals {
        particle.age += if particle.ignore_water { 1.0 } else { water };
    }
    for blur in &mut state.blurs {
        blur.age += 1.0;
    }
    state.blurs.retain(|blur| blur.age < blur.life);
    state
        .visuals
        .retain(|particle| particle.age < particle.life);
    if let Some(overlay) = &mut state.overlay {
        overlay.age += 1.0;
        if overlay.age >= overlay.life {
            state.overlay = None;
        }
    }

    if !input.in_world {
        state.clear();
        state.health = input.health;
        state.max_health = input.max_health.max(1.0);
        return sounds;
    }

    // `doEffectsInCreative` is false: a creative player is shown nothing.
    if input.creative {
        state.clear();
        state.desat = 1.0;
        state.slender = 0.0;
        state.health = input.health;
        state.max_health = input.max_health.max(1.0);
        return sounds;
    }

    state.health = input.health;
    state.max_health = input.max_health.max(1.0);
    state.pos = input.pos;

    // `EVManagerClient.onTick`: `if (!player.isAlive()) clearEverything();`, at the end of the
    // tick, so the killing blow's visuals go with the death screen.
    if input.health <= 0.0 {
        state.clear();
        state.health = input.health;
        return sounds;
    }

    // A health drop is the hit: the world's hook names its damage type; a client that only
    // hears about health sees a plain drop.
    if let Some(last) = state.last_health {
        let drop = last - input.health;
        if drop > 0.001 && state.options.blood {
            let hit = state.take_hit();
            spawn_damage(state, hit, drop, input.health);
        }
    }
    state.last_health = Some(input.health);

    if state.options.pulse && input.health > 0.0 {
        if let Some(out) = state.hb.tick(input.health, state.max_health) {
            if out {
                let peak =
                    Heartbeat::peak(input.health, state.max_health, state.options.pulse_strength);
                if peak > 0.001 {
                    state.overlay = Some(Overlay {
                        kind: OverlayKind::LowHealth,
                        age: 0.0,
                        life: HB_OVERLAY_TICKS as f32,
                        peak,
                        rgb: 0xFF_FFFF,
                    });
                }
                sounds.push(Sound::HeartOut);
                add_blur(
                    state,
                    beat_blur(state.health, state.max_health),
                    HB_BLUR_TICKS as f32,
                );
            } else {
                sounds.push(Sound::HeartIn);
                add_blur(
                    state,
                    beat_blur(state.health, state.max_health),
                    HB_BLUR_TICKS as f32,
                );
            }
        }
    }

    // `SandSplatHandler`: on the ground on sand, `(int)(Math.random() * 1.5)` splashes a tick
    // while sprinting and none while walking.
    if input.on_ground && input.on_sand {
        let modifier = if input.sprinting { 1.5 } else { 0.0 };
        let count = (state.rng.next_f32() * modifier) as usize;
        if count > 0 {
            spawn_kind(state, Kind::Sand, count, None);
        }
    }

    // `SplashHandler`: eyes entering or leaving water blur by 5 to 10 pixels over 10 ticks.
    if input.eyes_in_water != state.was_in_water {
        let peak = SPLASH_BLUR.0 + state.rng.next_f32() * (SPLASH_BLUR.1 - SPLASH_BLUR.0);
        add_blur(state, peak, SPLASH_BLUR_TICKS as f32);
    }
    state.was_in_water = input.eyes_in_water;

    // `SaturationHandler`: a `DecimalCurve(2.0, 0.0, 8.0, 1.0)` of the food level, faded by
    // 0.0005 a tick.
    let aim = if input.food <= 2 {
        0.0
    } else if input.food >= 8 {
        1.0
    } else {
        (input.food - 2) as f32 / 6.0
    };
    let step = 0.0005;
    state.desat = if state.desat < aim {
        (state.desat + step).min(aim)
    } else {
        (state.desat - step).max(aim)
    };

    // `SlenderHandler`: `distanceFactor` 0.25 of `1 / (distance / 3)^2`, capped at 3.5 and at
    // 0.3 of opacity.
    let modifier = input
        .enderman
        .map(|distance| (1.0 / (distance.max(0.1) / 3.0).powi(2)).min(3.5))
        .unwrap_or(0.0);
    state.slender = if modifier > 0.0 {
        (0.25 * modifier).clamp(0.0, 0.3)
    } else {
        0.0
    };

    if state.options.rain && input.rain > 0.2 && input.rain_visible {
        spawn_rain(state, input.pitch);
    }

    if state.visuals.len() > HARD_LIMIT {
        let excess = state.visuals.len() - HARD_LIMIT;
        state.visuals.drain(0..excess);
    }

    sounds
}

/// Adds the splatters of one hit, the original's counts and placement. Blood's count comes from
/// the damage and the health left; every other kind has its own splash count and lifetime, and
/// the whole batch shares one lifetime, as the original's `addParticlesFadeOut` does.
fn spawn_damage(state: &mut State, hit: Hit, damage: f32, health_after: f32) {
    if hit.splashes {
        if hit.kind.is_blood() {
            let count = blood_count(health_after, damage, state.options.amount)
                .min(state.options.max as usize);
            spawn_kind(state, hit.kind, count, None);
        } else {
            let (count, low, high) = hit.kind.splash();
            let life = state.rng.range_i32(low, high) as f32;
            spawn_kind(state, hit.kind, count, Some(life));
        }
    }
    if let Some(kind) = hit.overlay {
        let (life, peak, rgb) = match kind {
            // `addVisualFadeOut(tunnel, this, tunnelDuration)`: 10 to 15 ticks from fully on.
            OverlayKind::Tunnel => (state.rng.range_i32(10, 15) as f32, 1.0, 0xFF_FFFF),
            // Nor is the low-health pulse drawn here: `HeartbeatHandler` owns it.
            OverlayKind::LowHealth => (HB_OVERLAY_TICKS as f32, 0.5, 0xFF_FFFF),
            _ => (10.0, 0.7, 0xFF_FFFF),
        };
        state.overlay = Some(Overlay {
            kind,
            age: 0.0,
            life,
            peak: peak * state.options.duration as f32 / 100.0,
            rgb,
        });
    } else if state.options.hit_flash {
        // `hitEffectIntensity` (0 by default): peak, over `hitDuration` 1 to 10 ticks.
        let intensity = state.options.hit_flash_intensity as f32 / 100.0;
        if intensity > 0.0 {
            state.overlay = Some(Overlay {
                kind: OverlayKind::Damaged,
                age: 0.0,
                life: state.rng.range_i32(1, 10) as f32,
                peak: intensity * 0.2,
                rgb: 0xFF_FFFF,
            });
        }
    }
}

/// `count` splats of `kind`: the original's random texture, angle, scale and lifetime, and the
/// handler's own particle size (`getHeight` = 15% of the GUI's height, times the scale).
fn spawn_kind(state: &mut State, kind: Kind, count: usize, life: Option<f32>) {
    if count == 0 {
        return;
    }
    // One lifetime for the batch, as one `addParticlesFadeOut` gives its particles one curve.
    let (_, low, high) = kind.splash();
    let life = life.unwrap_or_else(|| state.rng.range_i32(low, high) as f32);
    let (scale_low, scale_high) = kind.scale();
    for _ in 0..count {
        let textures = kind.sprites();
        let sprite = &textures[state.rng.pick(textures.len())];
        let angle = if kind.can_rotate() {
            state.rng.pick(sprite.angles as usize) as u8
        } else {
            0
        };
        state.visuals.push(Particle {
            sprite,
            angle,
            variant: state.rng.pick(sprite.frames.max(1) as usize) as u16,
            x: biased_centre(&mut state.rng),
            y: biased_centre(&mut state.rng),
            height: SPLAT_HEIGHT * state.rng.range_f32(scale_low, scale_high),
            age: 0.0,
            life,
            rgb: kind.rgb(),
            alpha: kind.base_alpha(),
            ignore_water: kind.ignore_water(),
        });
    }
}

/// Rain drops, as the original's `RainHandler`: the count from the look angle (up gives more),
/// on every other tick while it rains.
fn spawn_rain(state: &mut State, pitch: f32) {
    // `amountDependOnAngle`: looking up multiplies the drops, looking down divides them.
    let factor = if pitch < 0.0 {
        1.0 + (pitch / -90.0) as f32 * 2.0
    } else {
        0.1 + 1.0 - pitch / 90.0
    };
    let count = if factor < 1.0 {
        if state.rng.next_f32() < factor {
            1
        } else {
            0
        }
    } else {
        factor as usize
    };
    spawn_kind(state, Kind::Drop, count.min(4), None);
}

/// An explosion the world announced: dust and the ringing, scaled by the distance and by
/// `seen_percent` (`ServerExplosion.getSeenPercent`: how much of the player the blast can see).
pub fn explosion(state: &mut State, distance: f32, radius: f32, seen_percent: f32) {
    // `d10 = (1 - d12) * getSeenPercent`, then the damage `ServerExplosion` gives.
    let reach = (radius * 2.0).max(1.0);
    let seen = (1.0 - distance / reach).clamp(0.0, 1.0) * seen_percent.clamp(0.0, 1.0);
    if seen <= 0.0 {
        return;
    }
    let damage = (((seen * seen + seen) / 2.0) * 7.0 * reach + 1.0) as f32;
    if damage <= 0.0 {
        return;
    }
    // `dustAmount = IntCurve(0, 0, 20, 30)`: 0 at no damage, 30 at 20.
    let count = (damage / 20.0).clamp(0.0, 1.0) * 30.0;
    let count = count.round() as usize;
    let life = state.rng.range_i32(500, 1000) as f32;
    spawn_kind(state, Kind::Dust, count, Some(life));

    // `maxBlur = DecimalCurve(0, 50, 10, 100)` over `explosionBlurTime = IntCurve(0, 10, 20, 20)`.
    let blur = 50.0 + (damage / 10.0).clamp(0.0, 1.0) * 50.0;
    let blur_ticks = 10.0 + (damage / 20.0).clamp(0.0, 1.0) * 10.0;
    add_blur(state, blur, blur_ticks);

    // `explosionSoundTime = IntCurve(0, 50, 20, 200)`: the ringing fades over that many ticks
    // while everything else is muffled by the same curve.
    let ticks = 50.0 + (damage / 20.0).clamp(0.0, 1.0) * 150.0;
    state.pending.push(Sound::Ring(ticks.round() as u32));
}

/// A splash potion that broke within five blocks: the original's potion overlay, 1.5 to 3
/// seconds, at up to full alpha.
pub fn potion(state: &mut State, distance: f32, rgb: u32) {
    // `modifier = 1 - min(5, distance) / 5`, the peak `min(1, modifier * 2)`, 30 to 60 ticks.
    let modifier = 1.0 - distance.min(5.0) / 5.0;
    if modifier <= 0.0 {
        return;
    }
    let life = state.rng.range_i32(30, 60) as f32;
    state.overlay = Some(Overlay {
        kind: OverlayKind::Potion,
        age: 0.0,
        life,
        peak: (2.0 * modifier).min(1.0),
        rgb,
    });
}

// -------------------------------------------------------------------------------------------
// What the renderer draws this frame.

/// One `sprite` call: a grid cell of a texture, tinted, placed on the GUI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawItem {
    pub sprite: &'static str,
    /// The cell, in the texture's pixels.
    pub src: [f32; 4],
    /// Centre in `0..1` of the GUI.
    pub centre: [f32; 2],
    /// Side as a fraction of the GUI height, already including the cell's rotation padding.
    pub size: f32,
    /// `0xAARRGGBB`.
    pub tint: u32,
    /// A full-screen overlay: covers the GUI whatever `centre` and `size` say.
    pub full: bool,
    /// A flat fill of `tint` over the whole GUI, for the two effects the original drew with a
    /// shader this platform gives a mod no way to run.
    pub fill: bool,
}

impl DrawItem {
    /// The `dst` rectangle of a `sprite` call, in GUI pixels, for a GUI `width` by `height`.
    pub fn dest(&self, width: f32, height: f32) -> [f32; 4] {
        if self.full {
            return [0.0, 0.0, width, height];
        }
        let side = self.size * height;
        [
            self.centre[0] * width - side * 0.5,
            self.centre[1] * height - side * 0.5,
            side,
            side,
        ]
    }
}

/// Everything on screen now, in draw order: the overlay first, then the splatters above it,
/// as the original drew them.
pub fn draw_list(state: &State) -> Vec<DrawItem> {
    let mut items = Vec::with_capacity(state.visuals.len() + 4);
    let side = sprites::OVERLAY_SIDE;
    let full = |sprite: &'static str, rgb: u32, alpha: f32| DrawItem {
        sprite,
        src: [0.0, 0.0, side, side],
        centre: [0.5, 0.5],
        size: 1.0,
        tint: argb(rgb, alpha),
        full: true,
        fill: false,
    };
    // The original's overlays, back to front as it drew them: the desaturation and the haze are
    // `VisualCategory::shader` and come first, then the overlay category, then the particles.
    // `desaturate`: a grey wash mixes the frame towards grey, which is what the shader does.
    // With the pack picked (`shader_pack`) the pack mixes it, and a wash on top would count the
    // effect twice. `state.desat` is the shader's `Saturation`: 1.0 is the untouched picture.
    let desat = (1.0 - state.desat).clamp(0.0, 1.0);
    if desat > 0.002 && !state.options.shader_pack {
        items.push(DrawItem {
            sprite: "",
            src: [0.0; 4],
            centre: [0.5, 0.5],
            size: 1.0,
            tint: argb(0x80_8080, desat * 0.6),
            full: true,
            fill: true,
        });
    }
    let radius = blur_radius(state);
    if radius > 0.0 && !state.options.shader_pack {
        // `blur`: a haze where the original blurred, as deep as the blur is wide, for a picture
        // the pack cannot touch. A splash's 5 px must still read as *something*, and a blast's
        // 100 px must not turn the screen grey, so the curve saturates at 0.4.
        let alpha = (radius / 150.0).clamp(0.0, 0.4);
        if alpha > 0.002 {
            items.push(DrawItem {
                sprite: "",
                src: [0.0; 4],
                centre: [0.5, 0.5],
                size: 1.0,
                tint: argb(0xF0_F0F0, alpha),
                full: true,
                fill: true,
            });
        }
    }
    if state.slender > 0.002 {
        // `VisualTypeOverlay("slender", 50)`: four files cycled at 150 ms a frame, on the clock
        // every animation of the mod shares.
        let frames = sprites::overlay::SLENDER;
        let frame =
            (state.clock_ms / sprites::overlay::SLENDER_FRAMETIME_MS) as usize % frames.len();
        items.push(full(frames[frame], 0xFF_FFFF, state.slender));
    }
    if let Some(overlay) = &state.overlay {
        items.push(full(overlay.kind.sprite(), overlay.rgb, overlay.opacity()));
    }
    for particle in &state.visuals {
        let alpha = particle.opacity();
        if alpha <= 0.002 {
            continue;
        }
        items.push(DrawItem {
            sprite: particle.name(state.clock_ms),
            src: particle.cell(state.clock_ms),
            centre: [particle.x, particle.y],
            size: particle.height * particle.sprite.padding(),
            tint: argb(particle.rgb, alpha),
            full: false,
            fill: false,
        });
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with(options: Options) -> State {
        let mut state = State::new();
        state.options = options;
        // A world that has been open a while at full food: the hunger visual has faded out, and
        // the picture is untouched (`Saturation` 1.0).
        state.desat = 1.0;
        state
    }

    #[test]
    fn a_sword_hit_is_a_slash_and_an_arrow_is_a_pierce() {
        assert_eq!(classify("minecraft:player_attack"), (Kind::Slash, None));
        assert_eq!(classify("mob_attack"), (Kind::Slash, None));
        // Only an `Arrow` is a pierce: a trident's direct entity is not a living one, so the
        // original falls through to its default splatter.
        assert_eq!(classify("arrow"), (Kind::Pierce, None));
        assert_eq!(classify("minecraft:mob_projectile"), (Kind::Splatter, None));
        assert_eq!(classify("fall"), (Kind::Impact, None));
        // Fire, freezing and drowning draw splashes only; the heat, freeze and water overlays
        // belong to the temperature addons this port has no counterpart for.
        assert_eq!(classify("lava"), (Kind::Fire, None));
        assert_eq!(classify("freeze"), (Kind::Ice, None));
        assert_eq!(classify("drown"), (Kind::Blob, None));
        assert_eq!(classify("fly_into_wall"), (Kind::Break, None));
        assert_eq!(classify("wither"), (Kind::Wither, None));
        assert_eq!(
            classify("starve"),
            (Kind::Splatter, Some(OverlayKind::Tunnel))
        );
        assert_eq!(classify("lightning_bolt"), (Kind::Shock, None));
        // A bee's sting, thorns, the warden's boom and a mace are splatters: the original's
        // living-attacker branches find no weapon and no species it knows.
        assert_eq!(classify("sting"), (Kind::Splatter, None));
        assert_eq!(classify("thorns"), (Kind::Splatter, None));
        assert_eq!(classify("sonic_boom"), (Kind::Splatter, None));
        assert_eq!(classify("mace_smash"), (Kind::Splatter, None));
        // Anything the original did not name is its default splatter.
        assert_eq!(classify("mod:stubbed_toe"), (Kind::Splatter, None));
    }

    #[test]
    fn blood_is_the_originals_colour() {
        // `new Color(0.3f, 0.01f, 0.01f, 0.7f)`: its 0.7 alpha is the colour's, and the draw call
        // takes the visual's opacity instead, so every kind starts at full alpha.
        assert_eq!(Kind::Splatter.rgb(), 0x4D_0303);
        assert_eq!(Kind::Splatter.base_alpha(), 1.0);
        assert_eq!(Kind::Fire.base_alpha(), 1.0);
        assert!(Kind::Splatter.is_blood() && !Kind::Dust.is_blood());
    }

    #[test]
    fn colours_reach_the_game_as_argb() {
        // 0xAARRGGBB: alpha high byte, then red, green, blue.
        assert_eq!(argb(BLOOD_RGB, 0.7), 0xB3_4D_0303);
        assert_eq!(argb(0x12_3456, 1.0), 0xFF_12_3456);
        assert_eq!(argb(0x12_3456, 0.0), 0x00_12_3456);
    }

    #[test]
    fn the_health_scaler_is_three_at_zero_and_one_and_a_half_from_twelve() {
        assert!((health_scaler(0.0) - 3.0).abs() < 1e-6);
        assert!((health_scaler(6.0) - 2.25).abs() < 1e-6);
        assert!((health_scaler(12.0) - 1.5).abs() < 1e-6);
        assert!((health_scaler(40.0) - 1.5).abs() < 1e-6);
    }

    #[test]
    fn a_bigger_hit_draws_more_blood_until_the_cap() {
        // 4 damage at 20 health: 4 * 1.5 = 6 splats.
        assert_eq!(blood_count(20.0, 4.0, 100), 6);
        assert_eq!(blood_count(20.0, 4.0, 200), 12);
        assert_eq!(blood_count(20.0, 0.0, 100), 0);
        // A nearly dead player bleeds about three times as much for the same hit; like the
        // original's `(int)` cast, a part of a splat is dropped.
        assert_eq!(blood_count(1.0, 4.0, 100), 11);
        assert_eq!(blood_count(20.0, f32::INFINITY, 100), 0);
    }

    #[test]
    fn particles_keep_the_originals_lifetime_and_fade_out() {
        let options = Options::default();
        let mut state = state_with(options);
        state.health = 20.0;
        state.max_health = 20.0;
        state.last_health = Some(20.0);
        client_tick(
            &mut state,
            &Input {
                creative: false,
                on_ground: false,
                on_sand: false,
                sprinting: false,
                food: 20,
                eyes_in_water: false,
                enderman: None,
                in_world: true,
                pos: [0.0; 3],
                health: 16.0,
                max_health: 20.0,
                rain: 0.0,
                rain_visible: false,
                pitch: 0.0,
            },
        );
        assert!(!state.visuals.is_empty());
        let lifetimes: Vec<f32> = state.visuals.iter().map(|p| p.life).collect();
        assert!(lifetimes
            .iter()
            .all(|&life| (500.0..=1500.0).contains(&life)));
        // Blood is drawn at full opacity, as the original's `rgba(r, g, b, opacity * 255)` does,
        // and fades from there.
        assert!(state
            .visuals
            .iter()
            .all(|p| (p.opacity() - FULL_ALPHA).abs() < 1e-6));
        // A splat fades linearly: half way through its life it is at half its opacity.
        let life = state.visuals[0].life;
        state.visuals[0].age = life * 0.5;
        assert!((state.visuals[0].opacity() - FULL_ALPHA * 0.5).abs() < 1e-6);
        state.visuals[0].age = 0.0;
        // Each splat is gone exactly at its own lifetime, so once the longest has run out the
        // screen is clear again.
        let longest = state.visuals.iter().map(|p| p.life).fold(0.0_f32, f32::max);
        for _ in 0..(longest as i32) {
            client_tick(
                &mut state,
                &Input {
                    creative: false,
                    on_ground: false,
                    on_sand: false,
                    sprinting: false,
                    food: 20,
                    eyes_in_water: false,
                    enderman: None,
                    in_world: true,
                    pos: [0.0; 3],
                    health: 16.0,
                    max_health: 20.0,
                    rain: 0.0,
                    rain_visible: false,
                    pitch: 0.0,
                },
            );
        }
        assert!(state.visuals.is_empty());
    }

    #[test]
    fn a_hit_at_one_tick_leaves_no_blood_behind() {
        let mut state = state_with(Options::default());
        state.health = 20.0;
        state.last_health = Some(20.0);
        client_tick(
            &mut state,
            &Input {
                creative: false,
                on_ground: false,
                on_sand: false,
                sprinting: false,
                food: 20,
                eyes_in_water: false,
                enderman: None,
                in_world: false,
                pos: [0.0; 3],
                health: 20.0,
                max_health: 20.0,
                rain: 0.0,
                rain_visible: false,
                pitch: 0.0,
            },
        );
        assert!(state.visuals.is_empty());
    }

    #[test]
    fn blood_off_draws_no_splatters() {
        let mut state = state_with(Options {
            blood: false,
            ..Options::default()
        });
        state.last_health = Some(20.0);
        client_tick(
            &mut state,
            &Input {
                creative: false,
                on_ground: false,
                on_sand: false,
                sprinting: false,
                food: 20,
                eyes_in_water: false,
                enderman: None,
                in_world: true,
                pos: [0.0; 3],
                health: 12.0,
                max_health: 20.0,
                rain: 0.0,
                rain_visible: false,
                pitch: 0.0,
            },
        );
        assert!(state.visuals.is_empty());
    }

    #[test]
    fn the_server_names_the_damage_type_of_the_next_drop() {
        let mut state = state_with(Options::default());
        state.last_health = Some(20.0);
        state.push_hit("minecraft:arrow");
        client_tick(
            &mut state,
            &Input {
                creative: false,
                on_ground: false,
                on_sand: false,
                sprinting: false,
                food: 20,
                eyes_in_water: false,
                enderman: None,
                in_world: true,
                pos: [0.0; 3],
                health: 17.0,
                max_health: 20.0,
                rain: 0.0,
                rain_visible: false,
                pitch: 0.0,
            },
        );
        assert!(state
            .visuals
            .iter()
            .all(|p| p.sprite.names[0].starts_with("enhancedvisuals:gui/ev/pierce")));
        // The hit is spent: the next drop is the default splatter.
        assert!(state.hits.is_empty());
    }

    #[test]
    fn a_drop_without_a_told_hit_is_a_splatter() {
        let mut state = state_with(Options::default());
        state.last_health = Some(20.0);
        client_tick(
            &mut state,
            &Input {
                creative: false,
                on_ground: false,
                on_sand: false,
                sprinting: false,
                food: 20,
                eyes_in_water: false,
                enderman: None,
                in_world: true,
                pos: [0.0; 3],
                health: 18.0,
                max_health: 20.0,
                rain: 0.0,
                rain_visible: false,
                pitch: 0.0,
            },
        );
        assert!(state
            .visuals
            .iter()
            .all(|p| p.sprite.names[0].starts_with("enhancedvisuals:gui/ev/splatter")));
    }

    #[test]
    fn the_heart_beats_out_then_in_and_stops_when_healed() {
        let mut hb = Heartbeat::default();
        // Rule out a beat at full health.
        for _ in 0..40 {
            assert_eq!(hb.tick(20.0, 20.0), None);
        }
        // At 4 health the buffer starts at 4 / 20 * 100 + 15 = 35 ticks.
        assert_eq!(hb.tick(4.0, 20.0), Some(true));
        // The in beat lands when the buffer reaches 5, 29 ticks later, then the next out beat
        // restarts the buffer.
        let mut beats = Vec::new();
        for _ in 0..80 {
            if let Some(beat) = hb.tick(4.0, 20.0) {
                beats.push(beat);
            }
        }
        assert_eq!(beats, [false, true, false, true]);
        assert!(Heartbeat::intensity(4.0, 20.0) > 0.0);
        assert_eq!(Heartbeat::intensity(20.0, 20.0), 0.0);
    }

    #[test]
    fn the_pulse_never_exceeds_the_originals_cap() {
        // `2 * (6 - health) / max` is capped at 0.7 before the halving, so a 10-health maximum
        // reaches the cap and a 20-health one does not.
        assert!((Heartbeat::peak(0.1, 10.0, 100) - 0.35).abs() < 1e-6);
        assert!((Heartbeat::peak(0.1, 10.0, 50) - 0.175).abs() < 1e-6);
        assert!((Heartbeat::peak(0.1, 20.0, 100) - 0.295).abs() < 1e-6);
        assert_eq!(Heartbeat::peak(20.0, 20.0, 100), 0.0);
    }

    #[test]
    fn splatters_land_at_the_edges_more_often_than_in_the_middle() {
        let mut rng = Rng::new(11);
        let mut outer = 0;
        let n = 10_000;
        for _ in 0..n {
            let x = biased_centre(&mut rng);
            let y = biased_centre(&mut rng);
            if (x - 0.5).abs() > 0.25 || (y - 0.5).abs() > 0.25 {
                outer += 1;
            }
        }
        let share = outer as f32 / n as f32;
        // 1 - rand^2 puts about 71% of the centres in the outer half of one axis.
        assert!((0.85..0.95).contains(&share), "outer share {share}");
    }

    #[test]
    fn a_particle_shows_the_frame_its_age_asks_for() {
        let sprite = &sprites::WITHER[0];
        let particle = Particle {
            sprite,
            angle: 2,

            variant: 0,
            x: 0.5,
            y: 0.5,
            height: SPLAT_HEIGHT,
            age: 0.0,
            life: 100.0,
            rgb: 0,
            alpha: 1.0,
            ignore_water: false,
        };
        // A wither frame lasts 150 ms (its `.ani` names 50: 3 ms each), on the clock the whole
        // mod shares, and the cells run angle by angle: frame 0 with angle 2 is the third cell.
        assert_eq!(particle.frame(0), 0);
        assert_eq!(
            particle.cell(0),
            [2.0 * sprite.cell, 0.0, sprite.cell, sprite.cell]
        );
        assert_eq!(particle.frame(149), 0, "a frame lasts 150 ms");
        assert_eq!(particle.frame(150), 1);
        // Eight angles on from the first cell: the group's second texture, its first cell.
        assert_eq!(particle.name(150), "enhancedvisuals:gui/ev/wither_0_1");
        assert_eq!(particle.cell(150), [0.0, 0.0, sprite.cell, sprite.cell]);
        // Frame 4 of 3 wraps to 1.
        assert_eq!(particle.frame(600), 1);
        // A group with no `.ani` still animates: three frames at 3 ms each.
        let heat = &sprites::HEAT[0];
        assert_eq!(heat.frametime_ms, 3);
        assert_eq!(heat.frame_at(2, 0), 0);
        assert_eq!(heat.frame_at(3, 0), 1);
        assert_eq!(heat.frame_at(9, 0), 0);
    }

    #[test]
    fn every_texture_fits_the_hud_atlas_cap() {
        // The game's HUD atlas refuses a non-particle sprite wider than 1024 px or taller than
        // 512: it drops the texture, and a dropped texture draws nothing at all.
        for kind in [
            Kind::Splatter,
            Kind::Slash,
            Kind::Impact,
            Kind::Pierce,
            Kind::Blob,
            Kind::Fire,
            Kind::Ice,
            Kind::Wither,
            Kind::Shock,
            Kind::Dust,
            Kind::Drop,
            Kind::Heat,
            Kind::Break,
            Kind::Parasite,
            Kind::Sand,
        ] {
            for sprite in kind.sprites() {
                let name = sprite.names[0];
                let (w, h) = (
                    sprite.cell * f32::from(sprites::COLUMNS),
                    sprite.cell * f32::from(sprites::ROWS),
                );
                assert!(w <= 1024.0 && h <= 512.0, "{name} would be {w}x{h}");
                let cells = u32::from(sprites::COLUMNS) * u32::from(sprites::ROWS);
                let needed = u32::from(sprite.frames) * u32::from(sprite.angles);
                assert_eq!(
                    sprite.names.len() as u32,
                    needed.div_ceil(cells),
                    "{name}: one texture per {cells} cells"
                );
                for frame in 0..sprite.frames {
                    for angle in 0..sprite.angles {
                        assert!(!sprite.name_of(frame, angle).is_empty(), "{name}");
                        let src = sprite.cell_of(frame, angle);
                        assert!(src[0] + src[2] <= w, "{name} cell is outside its texture");
                        assert!(src[1] + src[3] <= h, "{name} cell is outside its texture");
                    }
                }
            }
        }
    }

    #[test]
    fn drawn_splats_keep_the_originals_size_after_rotation_padding() {
        let mut state = state_with(Options::default());
        state.visuals.push(Particle {
            sprite: &sprites::SPLATTER[0],
            angle: 1,

            variant: 0,
            x: 0.5,
            y: 0.5,
            height: SPLAT_HEIGHT,
            age: 0.0,
            life: 100.0,
            rgb: BLOOD_RGB,
            alpha: FULL_ALPHA,
            ignore_water: false,
        });
        let item = draw_list(&state)[0];
        let padding = sprites::SPLATTER[0].padding();
        assert!(padding > 1.0, "a turned square needs a bigger cell");
        // The cell is drawn bigger by the padding, so the original texture still covers 15% of
        // a 1000 pixel GUI whatever the rotation needed.
        let dest = item.dest(1000.0, 1000.0);
        assert!(
            (dest[2] / padding - 150.0).abs() < 0.5,
            "drew {} px",
            dest[2] / padding
        );
    }

    #[test]
    fn the_overlay_is_drawn_below_the_splatters_and_covers_the_screen() {
        let mut state = state_with(Options::default());
        state.overlay = Some(Overlay {
            kind: OverlayKind::Tunnel,
            age: 0.0,
            life: 100.0,
            peak: 0.7,
            rgb: 0xFF_FFFF,
        });
        state.visuals.push(Particle {
            sprite: &sprites::ICE[0],
            angle: 0,

            variant: 0,
            x: 0.2,
            y: 0.3,
            height: SPLAT_HEIGHT,
            age: 0.0,
            life: 100.0,
            rgb: Kind::Ice.rgb(),
            alpha: 1.0,
            ignore_water: false,
        });
        let items = draw_list(&state);
        assert_eq!(items.len(), 2);
        assert!(items[0].full);
        assert_eq!(items[0].dest(320.0, 240.0), [0.0, 0.0, 320.0, 240.0]);
        assert_eq!(items[0].tint >> 24, (0.7 * 255.0f32).round() as u32);
        assert!(!items[1].full);
    }

    #[test]
    fn an_explosion_dusts_and_rings_by_distance() {
        let mut near = state_with(Options::default());
        explosion(&mut near, 0.0, 4.0, 1.0);
        assert!(!near.visuals.is_empty());
        assert_eq!(near.pending.len(), 1);
        assert!(matches!(near.pending[0], Sound::Ring(_)));
        assert!(blur_radius(&near) >= 50.0, "the blast's blur");

        // A wall between the blast and the player: `getSeenPercent` answers 0.
        let mut hidden = state_with(Options::default());
        explosion(&mut hidden, 0.0, 4.0, 0.0);
        assert!(hidden.visuals.is_empty());
        assert!(hidden.pending.is_empty());

        let mut far = state_with(Options::default());
        explosion(&mut far, 8.0, 4.0, 1.0);
        assert!(far.visuals.is_empty());

        // The dust and the blur age away with the ticks.
        let mut state = near;
        state.last_health = Some(20.0);
        let mut ticks = 0;
        while (!state.blurs.is_empty() || !state.visuals.is_empty()) && ticks < 1200 {
            client_tick(
                &mut state,
                &Input {
                    creative: false,
                    on_ground: false,
                    on_sand: false,
                    sprinting: false,
                    food: 20,
                    eyes_in_water: false,
                    enderman: None,
                    in_world: true,
                    pos: [0.0; 3],
                    health: 20.0,
                    max_health: 20.0,
                    rain: 0.0,
                    rain_visible: false,
                    pitch: 0.0,
                },
            );
            ticks += 1;
        }
        assert!(ticks < 1200, "the blast's dust lasted {ticks} ticks");
    }

    #[test]
    fn the_death_screen_quips_skip_the_last_message() {
        // `DeathMessages.pickRandomDeathMessage` asks for `nextInt(size - 1)`, so the last
        // message is never the one shown.
        let mut rng = Rng::new(0x5EED);
        let last = DEATH_MESSAGES[DEATH_MESSAGES.len() - 1];
        for _ in 0..200 {
            let quip = death_message(&mut rng).expect("a quip");
            assert_ne!(quip, last, "the original never picks the last message");
            assert!(DEATH_MESSAGES.contains(&quip));
        }
    }

    #[test]
    fn a_death_clears_the_screen() {
        let mut state = state_with(Options::default());
        state.visuals.push(Particle {
            sprite: &sprites::SPLATTER[0],
            angle: 0,
            variant: 0,
            x: 0.5,
            y: 0.5,
            height: SPLAT_HEIGHT,
            age: 0.0,
            life: 100.0,
            rgb: BLOOD_RGB,
            alpha: FULL_ALPHA,
            ignore_water: false,
        });
        let mut input = Input {
            creative: false,
            on_ground: false,
            on_sand: false,
            sprinting: false,
            food: 20,
            eyes_in_water: false,
            enderman: None,
            in_world: true,
            pos: [0.0; 3],
            health: 0.0,
            max_health: 20.0,
            rain: 0.0,
            rain_visible: false,
            pitch: 0.0,
        };
        client_tick(&mut state, &input);
        assert!(state.visuals.is_empty(), "`clearEverything` on death");
        input.health = 20.0;
        client_tick(&mut state, &input);
        assert!(state.visuals.is_empty());
    }

    #[test]
    fn water_blurs_when_the_eyes_go_in_or_come_out() {
        let mut state = state_with(Options::default());
        let mut input = Input {
            creative: false,
            on_ground: false,
            on_sand: false,
            sprinting: false,
            food: 20,
            eyes_in_water: false,
            enderman: None,
            in_world: true,
            pos: [0.0; 3],
            health: 20.0,
            max_health: 20.0,
            rain: 0.0,
            rain_visible: false,
            pitch: 0.0,
        };
        client_tick(&mut state, &input);
        assert_eq!(blur_radius(&state), 0.0, "dry eyes blur nothing");

        // `SplashHandler`: 5 to 10 px for 10 ticks, on the way in and on the way out.
        input.eyes_in_water = true;
        client_tick(&mut state, &input);
        let in_water = blur_radius(&state);
        assert!((5.0..=10.0).contains(&in_water), "under water: {in_water}");
        for _ in 0..10 {
            client_tick(&mut state, &input);
        }
        assert_eq!(blur_radius(&state), 0.0, "the splash fades over 10 ticks");

        input.eyes_in_water = false;
        client_tick(&mut state, &input);
        assert!(
            (5.0..=10.0).contains(&blur_radius(&state)),
            "surfacing splashes too"
        );
    }

    #[test]
    fn the_low_health_beat_blurs_the_picture() {
        // `HeartbeatHandler`: each beat blurs by `min(0.7, intensity) * 50`, up to 35 px.
        let mut state = state_with(Options::default());
        let input = Input {
            creative: false,
            on_ground: false,
            on_sand: false,
            sprinting: false,
            food: 20,
            eyes_in_water: false,
            enderman: None,
            in_world: true,
            pos: [0.0; 3],
            health: 2.0,
            max_health: 20.0,
            rain: 0.0,
            rain_visible: false,
            pitch: 0.0,
        };
        client_tick(&mut state, &input);
        let radius = blur_radius(&state);
        // (6 - 2) / 20 * 2 = 0.4, so 0.4 * 50 = 20 px.
        assert_eq!(radius, 20.0);
        assert!(state.overlay.is_some(), "and the red pulse with it");
    }

    #[test]
    fn the_shader_pack_is_handed_the_blurs_and_the_desaturation() {
        let mut state = state_with(Options::default());
        // At rest the pack changes nothing: no blur, and the picture's own saturation.
        assert_eq!(shader_strength(&state), [0.0, 1.0, 0.0, 0.0]);

        explosion(&mut state, 0.0, 4.0, 1.0);
        let [blur, sat, ..] = shader_strength(&state);
        assert!(blur >= 50.0, "a blast blurs this frame, in pixels");
        // A full stomach is the untouched picture: `Saturation` 1.0.
        assert_eq!(sat, 1.0);

        // The blur fades with the ticks as the haze does, and the pack sees it fade.
        let first = blur;
        for _ in 0..40 {
            client_tick(
                &mut state,
                &Input {
                    creative: false,
                    on_ground: false,
                    on_sand: false,
                    sprinting: false,
                    food: 20,
                    eyes_in_water: false,
                    enderman: None,
                    in_world: true,
                    pos: [0.0; 3],
                    health: 20.0,
                    max_health: 20.0,
                    rain: 0.0,
                    rain_visible: false,
                    pitch: 0.0,
                },
            );
        }
        assert!(shader_strength(&state)[0] < first);

        // The desaturation comes from the food level's curve.
        state.desat = 0.6;
        assert_eq!(shader_strength(&state)[1], 0.6);

        // With the pack on, the mod draws neither wash: the pack draws both.
        let mut packed = state_with(Options {
            shader_pack: true,
            ..Options::default()
        });
        packed.desat = 0.7;
        packed.blurs.push(Blur {
            peak: 80.0,
            age: 0.0,
            life: 20.0,
        });
        let items = draw_list(&packed);
        assert!(
            items.iter().all(|item| !item.fill),
            "the pack's washes stay off the HUD"
        );
        assert_eq!(shader_strength(&packed)[0], 80.0);
    }

    #[test]
    fn a_splash_potion_fades_in_over_seconds() {
        let mut state = state_with(Options::default());
        potion(&mut state, 0.0, 0xFF_FFFF);
        let overlay = state.overlay.expect("a potion overlay");
        assert_eq!(overlay.kind, OverlayKind::Potion);
        assert!((30.0..=60.0).contains(&overlay.life));
        assert!((overlay.peak - 1.0).abs() < 1e-6);

        let mut far = state_with(Options::default());
        potion(&mut far, 6.0, 0xFF_FFFF);
        assert!(far.overlay.is_none());
    }

    #[test]
    fn rain_drops_fall_more_often_when_looking_up() {
        let mut up = state_with(Options::default());
        let mut down = state_with(Options::default());
        for _ in 0..40 {
            spawn_rain(&mut up, -80.0);
            spawn_rain(&mut down, 80.0);
        }
        assert!(up.visuals.len() > down.visuals.len());
        assert!(up
            .visuals
            .iter()
            .all(|p| p.sprite.names[0].starts_with("enhancedvisuals:gui/ev/drop")));
    }

    #[test]
    fn settings_choices_clamp_to_the_originals_lists() {
        let options =
            Options::from_choices(true, false, 3, 2, 2, true, 3, false, true, false, true);
        assert_eq!(options.amount, 100);
        assert_eq!(options.duration, 100);
        assert_eq!(options.max, 48);
        assert_eq!(options.pulse_strength, 100);
        assert!(!options.sounds);
        // An index past the end reads the original's default, not a panic.
        let options =
            Options::from_choices(true, false, 99, 99, 99, true, 99, true, true, true, true);
        assert_eq!(options.amount, 100);
        assert_eq!(options.max, 48);
    }

    #[test]
    fn the_options_hold_a_hit_to_the_configured_cap() {
        let mut state = state_with(Options {
            max: 16,
            ..Options::default()
        });
        state.health = 20.0;
        state.last_health = Some(20.0);
        client_tick(
            &mut state,
            &Input {
                creative: false,
                on_ground: false,
                on_sand: false,
                sprinting: false,
                food: 20,
                eyes_in_water: false,
                enderman: None,
                in_world: true,
                pos: [0.0; 3],
                health: 0.5,
                max_health: 20.0,
                rain: 0.0,
                rain_visible: false,
                pitch: 0.0,
            },
        );
        assert_eq!(state.visuals.len(), 16);
    }

    #[test]
    fn the_sounds_are_the_originals_own_files() {
        assert_eq!(Sound::HeartOut.event(), "enhancedvisuals:heartbeatout");
        assert_eq!(Sound::HeartIn.event(), "enhancedvisuals:heartbeatin");
        assert_eq!(Sound::Ring(5).event(), "enhancedvisuals:ringing");
    }
}
