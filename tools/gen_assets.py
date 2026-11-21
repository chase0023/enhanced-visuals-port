#!/usr/bin/env python3
"""Builds this mod's textures and `src/sprites.rs` from the original EnhancedVisuals jar.

EnhancedVisuals draws each splat as one textured quad turned by a random angle. A Horizon mod
draws through the game's `DrawContext`, which blits axis-aligned rectangles, so every frame is
pre-turned and packed into a grid here: one cell per (frame, rotation). The mod picks a cell with
the `src` rectangle of one `sprite` call, and the game does no rotation of its own.

Grid layout of one texture: a column per rotation, a row per animation frame

    +---------+---------+---------+---------+   cell = content * sqrt(2), rounded up, so a
    | rot 0   | rot 45  | rot 90  | rot 135 |   turned square is never cut at its corners.
    | frame 0 |         |         |         |
    +---------+---------+---------+---------+   The game's HUD atlas drops any non-particle
    | frame 1 |         |         |         |   sprite wider than 1024 px or taller than 512,
    +---------+---------+---------+---------+   so frames are split over as many textures as
                                                that takes.

Usage:  python tools/gen_assets.py [path/to/EnhancedVisuals.jar]
"""

import json
import math
import re
import sys
import zipfile
from pathlib import Path

from PIL import Image

HERE = Path(__file__).resolve().parent.parent
ASSETS = HERE / "assets" / "enhancedvisuals"
JAR = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(
    "C:/Users/Junniel F De Veyra/Downloads/EnhancedVisuals_FABRIC_v1.8.35_mc26.3.jar")

#: The stored rotations: 45-degree steps. The original turns each splat by a random 0-360
#: degrees; the game's `DrawContext` blits axis-aligned rectangles, so the turns are baked in
#: and a particle picks the nearest stored one.
ANGLES = (0, 45, 90, 135, 180, 225, 270, 315)
#: Blood is the effect players look at most, so its splats keep the original 128 px.
BLOOD_CONTENT = 128
OTHER_CONTENT = 96


def cell_for(content):
    return int(math.ceil(content * math.sqrt(2)))


#: A HUD atlas sprite's largest accepted size: heavier ones are dropped, never scaled.
MAX_TEXTURE = (1024, 512)
#: Grid of one texture: a column per stored angle, a row per animation frame.
COLUMNS = 5
ROWS = 2


#: kind -> (sprite group prefix, [(frames, frametime ms)]) in the order the original picks them.
GROUPS = {
    "splatter": [("splatter", 4, 1, 0)],
    "slash": [("slash", 5, 1, 0)],
    # `impact3alphatest.png` is not `impact3.png`, so the original's loader never finds a
    # fourth group and only three are ever drawn.
    "impact": [("impact", 3, 1, 0)],
    "pierce": [("pierce", 3, 1, 0)],
    "blob": [("blob", 2, 1, 0)],
    "fire": [("fire", 2, 1, 0)],
    "ice": [("ice", 5, 1, 0)],
    "wither": [(f"wither{g}", 3, 1, 0) for g in range(5)],
    "shock": [("shock0", 4, 1, 0)],
    "dust": [("dust", 3, 1, 0)],
    "drop": [("drop", 1, 1, 0)],
    "heat": [(f"heat{g}", 3, 1, 0) for g in range(2)],
    "break": [("break", 2, 1, 0)],
    "sand": [("sand", 6, 1, 0)],
    "parasite": [(f"parasite{g}", 3, 1, 0) for g in range(6)],
}

BLOOD_KINDS = {"splatter", "slash", "impact", "pierce"}
#: Kinds the original never turns (`canRotate()` is false for its heat particles).
NO_ROTATION = {"heat"}

#: The overlays the port uses: (key, the original's folder, frames, frame clock in ms). The
#: slender overlay is a `VisualTypeOverlay(name, 50)`, so its four files animate at 150 ms a
#: frame; the others are one image each.
OVERLAYS = [
    ("low_health", "lowhealth", 1, 0),
    ("tunnel", "tunnel", 1, 0),
    ("potion", "potion", 1, 0),
    ("damaged", "damaged", 1, 0),
    ("slender", "slender", 4, 150),
]
OVERLAY_SIZE = 256

PREFIX = "assets/enhancedvisuals/visuals/particle/"
OVERLAY_PREFIX = "assets/enhancedvisuals/visuals/overlay/"


def frames_of(z, jar_name):
    """The original's files for one group, and whether they animate.

    `splatter` is `splatter0..3` (stills picked at random); `wither0` is
    `wither0-0..2` (one group that animates). Both shapes are handled by trying
    the numbered files first and then the `-frame` ones. A name the original's
    own loader would miss, as `impact3alphatest` is, is not one of them.
    """
    folder = jar_name.rstrip("0123456789") or jar_name
    still = []
    for i in range(64):
        name = f"{PREFIX}{folder}/{jar_name}{i}.png"
        if name in z.namelist():
            still.append(name)
        elif still:
            break
    if still:
        return still, False
    animated = []
    for i in range(64):
        name = f"{PREFIX}{folder}/{jar_name}-{i}.png"
        if name in z.namelist():
            animated.append(name)
        elif animated:
            break
    return animated, True


def turned_cells(z, name, content, cell):
    """The four stored rotations of one frame, each in its own padded cell."""
    with z.open(name) as handle:
        source = Image.open(handle).convert("RGBA")
    if source.size != (content, content):
        source = source.resize((content, content), Image.LANCZOS)
    padded = Image.new("RGBA", (cell, cell), (0, 0, 0, 0))
    at = (cell - content) // 2
    padded.paste(source, (at, at))
    cells = []
    for angle in ANGLES:
        if angle == 0:
            cells.append(padded)
        elif angle == 90:
            cells.append(padded.transpose(Image.ROTATE_90))
        else:
            cells.append(padded.rotate(angle, resample=Image.BICUBIC, expand=False))
    return cells


def frames_ani(z, jar_name):
    """The original's `.ani` frame clock for one group, 0 when it has none."""
    folder = jar_name.rstrip("0123456789") or jar_name
    name = f"{PREFIX}{folder}/{jar_name}.ani"
    if name not in z.namelist():
        return 0
    try:
        return int(z.read(name).decode().strip())
    except ValueError:
        return 0


def build_sprite(z, jar_name, content, angles):
    """The group's cells as textures of `COLUMNS` x `ROWS` cells: angles across, frames down."""
    cell = cell_for(content)
    frames, animated = frames_of(z, jar_name)
    cells = []
    for name in frames:
        turned = turned_cells(z, name, content, cell)
        cells.extend(turned[:angles])
    per_texture = COLUMNS * ROWS
    sheets = []
    for start in range(0, len(cells), per_texture):
        chunk = cells[start:start + per_texture]
        width, height = cell * min(COLUMNS, len(chunk)), cell * ROWS
        if width > MAX_TEXTURE[0] or height > MAX_TEXTURE[1]:
            raise SystemExit(
                f"{jar_name}: a {width}x{height} texture is past the HUD atlas's "
                f"{MAX_TEXTURE[0]}x{MAX_TEXTURE[1]}; lower COLUMNS/ROWS or the content size")
        sheet = Image.new("RGBA", (width, height), (0, 0, 0, 0))
        for index, turned in enumerate(chunk):
            column, row = index % COLUMNS, index // COLUMNS
            sheet.paste(turned, (column * cell, row * cell))
        sheets.append(sheet)
    return sheets, cell, len(frames), animated


def main():
    if not JAR.is_file():
        sys.exit(f"the original jar is not at {JAR}; pass its path as the first argument")
    z = zipfile.ZipFile(JAR)
    out_dir = ASSETS / "textures" / "gui" / "ev"
    out_dir.mkdir(parents=True, exist_ok=True)

    table = {}
    for kind, groups in GROUPS.items():
        content = BLOOD_CONTENT if kind in BLOOD_KINDS else OTHER_CONTENT
        angles = ANGLES if kind not in NO_ROTATION else ANGLES[:1]
        entries = []
        for index, (jar_name, frame_count, _, frametime_ms) in enumerate(groups):
            sheets, cell, found, animated = build_sprite(z, jar_name, content, len(angles))
            names = []
            for chunk, sheet in enumerate(sheets):
                sprite = f"{kind}_{index}_{chunk}"
                sheet.save(out_dir / f"{sprite}.png", optimize=True)
                names.append(f"enhancedvisuals:gui/ev/{sprite}")
                print(f"{sprite}.png  {sheet.width}x{sheet.height}")
            if found != frame_count:
                raise SystemExit(f"{kind}: the jar has {found} frames, the table says {frame_count}")
            print(f"  {kind}: {found} frames x {len(angles)} angles over "
                  f"{len(names)} textures, cell {cell} px")
            # A `.ani` file holds the original's frame clock: 3 ms times the number it names.
            # Without one `TextureCache.parse` still animates the group, at 3 ms a frame; only a
            # group of one file is a still the particle keeps.
            ani = frames_ani(z, jar_name) or 1
            entries.append((names, found, len(angles), cell, content, ani * 3 if animated else 0))
        table[kind] = entries

    overlay_entries = {}
    overlay_dir = out_dir / "overlay"
    overlay_dir.mkdir(exist_ok=True)
    for key, folder, frames, frametime_ms in OVERLAYS:
        for frame in range(frames):
            name = f"{OVERLAY_PREFIX}{folder}/{folder}{frame}.png"
            with z.open(name) as handle:
                image = Image.open(handle).convert("RGBA")
            image = image.resize((OVERLAY_SIZE, OVERLAY_SIZE), Image.LANCZOS)
            out = key if frames == 1 else f"{key}_{frame}"
            image.save(overlay_dir / f"{out}.png", optimize=True)
            print(f"overlay/{out}.png  {OVERLAY_SIZE}x{OVERLAY_SIZE}")
        overlay_entries[key] = (
            [f"enhancedvisuals:gui/ev/overlay/{key}" if frames == 1
             else f"enhancedvisuals:gui/ev/overlay/{key}_{frame}" for frame in range(frames)],
            frametime_ms,
        )

    sprites_rs = render_rust(table, overlay_entries)
    (HERE / "src" / "sprites.rs").write_text(sprites_rs)

    # The original sounds and their `sounds.json` travel unchanged: a `.hmod` carries `assets/`.
    sounds = ASSETS / "sounds"
    sounds.mkdir(parents=True, exist_ok=True)
    for name in z.namelist():
        if name.startswith("assets/enhancedvisuals/sounds/") and not name.endswith("/"):
            (sounds / Path(name).name).write_bytes(z.read(name))
    (ASSETS / "sounds.json").write_text(z.read("assets/enhancedvisuals/sounds.json").decode())
    (HERE / "icon.png").write_bytes(z.read("enhancedvisuals.png"))
    (HERE / "LICENSE").write_bytes(z.read("LICENSE"))
    print("sounds, sounds.json, the icon and the licence copied")


def render_rust(table, overlays):
    const_of = {
        "splatter": "SPLATTER", "slash": "SLASH", "impact": "IMPACT", "pierce": "PIERCE",
        "blob": "BLOB", "fire": "FIRE", "ice": "ICE", "wither": "WITHER", "shock": "SHOCK",
        "dust": "DUST", "drop": "DROP", "heat": "HEAT", "break": "BREAK", "parasite": "PARASITE", "sand": "SAND",
    }
    lines = [
        "// @generated by tools/gen_assets.py from EnhancedVisuals 1.8.35 - do not edit.",
        "//",
        "// One `Sprite` is one particle, over as many textures as the HUD atlas takes: a column",
        "// per stored rotation and a row per animation frame, so `src` of one `sprite` call picks",
        "// the frame and the rotation without the game turning anything.",
        "",
        "/// The stored angles a texture holds, one per column.",
        "#[allow(dead_code)]",
        "pub const ROTATIONS: u16 = 8;",
        "/// The cells of one texture: `COLUMNS` angles across, `ROWS` frames down.",
        "pub const COLUMNS: u16 = 5;",
        "pub const ROWS: u16 = 2;",
        "",
        "/// A particle texture of the mod: the original's frames and stored angles, packed into",
        "/// as many textures as the game's HUD atlas takes.",
        "#[derive(Clone, Copy, Debug)]",
        "pub struct Sprite {",
        "    /// The sprite's names in the game's HUD atlas, in cell order.",
        "    pub names: &'static [&'static str],",
        "    /// Animation frames in the group.",
        "    pub frames: u16,",
        "    /// Stored angles in this kind, `1..=ROTATIONS`.",
        "    pub angles: u16,",
        "    /// One cell's side in the texture's pixels.",
        "    pub cell: f32,",
        "    /// The original texture's side in pixels, before the angle padding.",
        "    pub content: f32,",
        "    /// Milliseconds a frame is shown; 0 when the frames are variants, not animation.",
        "    pub frametime_ms: u32,",
        "}",
        "",
        "impl Sprite {",
        "    /// The animation frame shown `age_ms` milliseconds in. A group with no frame clock",
        "    /// keeps the variant it was given, as the original does.",
        "    pub fn frame_at(&self, age_ms: u32, chosen: u16) -> u16 {",
        "        if self.frames == 0 {",
        "            return 0;",
        "        }",
        "        if self.frametime_ms == 0 {",
        "            return chosen % self.frames;",
        "        }",
        "        ((age_ms / self.frametime_ms) % u32::from(self.frames)) as u16",
        "    }",
        "",
        "    /// The cell this frame and angle live in, `0..`.",
        "    fn index_of(&self, frame: u16, angle: u16) -> u32 {",
        "        let frame = if self.frames == 0 { 0 } else { frame % self.frames };",
        "        let angle = if self.angles == 0 { 0 } else { angle % self.angles };",
        "        u32::from(frame) * u32::from(self.angles) + u32::from(angle)",
        "    }",
        "",
        "    /// The texture holding this frame and angle.",
        "    pub fn name_of(&self, frame: u16, angle: u16) -> &'static str {",
        "        let cells = u32::from(COLUMNS) * u32::from(ROWS);",
        "        let name = (self.index_of(frame, angle) / cells) as usize;",
        "        self.names[name.min(self.names.len() - 1)]",
        "    }",
        "",
        "    /// The `src` rectangle of this frame and angle, in the texture's pixels.",
        "    pub fn cell_of(&self, frame: u16, angle: u16) -> [f32; 4] {",
        "        let cells = u32::from(COLUMNS) * u32::from(ROWS);",
        "        let within = self.index_of(frame, angle) % cells;",
        "        [",
        "            f32::from((within % u32::from(COLUMNS)) as u16) * self.cell,",
        "            f32::from((within / u32::from(COLUMNS)) as u16) * self.cell,",
        "            self.cell,",
        "            self.cell,",
        "        ]",
        "    }",
        "",
        "    /// How much bigger a cell is than the original texture, so a drawn splat keeps the",
        "    /// original's size whatever padding its angle needed.",
        "    pub fn padding(&self) -> f32 {",
        "        if self.content <= 0.0 { 1.0 } else { self.cell / self.content }",
        "    }",
        "}",
        "",
    ]
    for kind, entries in table.items():
        const = const_of[kind]
        lines.append(f"/// The `{kind}` textures, in the order a particle picks one.")
        lines.append(f"pub const {const}: &[Sprite] = &[")
        for names, frames, angles, cell, content, frametime in entries:
            listed = ", ".join(f'"{name}"' for name in names)
            lines.append(
                f"    Sprite {{ names: &[{listed}], frames: {frames}, angles: {angles}, "
                f"cell: {float(cell):.1f}, content: {float(content):.1f}, "
                f"frametime_ms: {frametime} }},")
        lines.append("];")
        lines.append("")
    lines.append("/// The side of every overlay texture, in its own pixels.")
    lines.append(f"pub const OVERLAY_SIDE: f32 = {float(OVERLAY_SIZE):.1f};")
    lines.append("")
    lines.append("/// The full-screen overlays. `SLENDER` is the one that animates: four files the")
    lines.append("/// original's `VisualTypeOverlay(\"slender\", 50)` cycles at 150 ms a frame.")
    lines.append("pub mod overlay {")
    for key, (names, frametime_ms) in overlays.items():
        listed = ", ".join(f'"{name}"' for name in names)
        upper = key.upper()
        if len(names) == 1:
            lines.append(f'    pub const {upper}: &str = "{names[0]}";')
        else:
            lines.append(f"    pub const {upper}: &[&str] = &[{listed}];")
            lines.append(f"    pub const {upper}_FRAMETIME_MS: u32 = {frametime_ms};")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


if __name__ == "__main__":
    main()
