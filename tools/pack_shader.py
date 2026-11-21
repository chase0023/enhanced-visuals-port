#!/usr/bin/env python3
"""Build the companion shader pack from `shaderpack/src` into a `.hshader`.

A mod package may hold no other `.toml` than its manifest, so the pack's manifest lives here as
`horizon-shaders.toml.in` and is assembled in a temporary folder for `horizon-shader pack`, which
checks the pack and writes the file the player drops into `shaderpacks/`.

    python tools/pack_shader.py

Needs `horizon-shader` on `PATH`, or its path in `HORIZON_SHADER`, built from the pinned Horizon
checkout: `cargo build -p horizon-shader-api --bin horizon-shader`.
"""

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "shaderpack" / "src"
PACK = "native-enhancedvisuals"
OUT = ROOT / "shaderpack" / f"{PACK}.hshader"
#: The same file inside the mod's assets, so it can install itself into `shaderpacks/`: a mod
#: package carries every file under `assets/`, and nothing outside it.
BUNDLED = ROOT / "assets" / "enhancedvisuals" / f"{PACK}.hshader"


def tool() -> str:
    named = os.environ.get("HORIZON_SHADER") or shutil.which("horizon-shader")
    if not named:
        sys.exit(
            "horizon-shader not found: build it in the pinned Horizon checkout and put it on PATH, "
            "or set HORIZON_SHADER to its path"
        )
    return named


def main() -> None:
    with tempfile.TemporaryDirectory() as temp:
        staging = Path(temp) / PACK
        shutil.copytree(SOURCE, staging)
        (staging / "horizon-shaders.toml").write_bytes(
            (SOURCE / "horizon-shaders.toml.in").read_bytes()
        )
        (staging / "horizon-shaders.toml.in").unlink()
        command = [tool(), "pack", str(staging), "--out", str(OUT)]
        result = subprocess.run(command, capture_output=True, text=True)
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        if result.returncode != 0:
            sys.exit(result.returncode)
    BUNDLED.parent.mkdir(parents=True, exist_ok=True)
    BUNDLED.write_bytes(OUT.read_bytes())
    print(f"wrote {OUT.relative_to(ROOT)} and {BUNDLED.relative_to(ROOT)} "
          f"({OUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
