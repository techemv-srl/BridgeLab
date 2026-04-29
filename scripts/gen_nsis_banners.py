#!/usr/bin/env python3
"""
Generate NSIS installer banner BMPs from the existing BridgeLab logo.

NSIS expects:
- header image: 150x57 BMP (banner shown at the top of every wizard page)
- sidebar image: 164x314 BMP (decorative panel on Welcome and Finish pages)

Both are baked into the installer so they need to live as committed files.
This script regenerates them from src-tauri/icons/128x128.png + a dark
background tinted with the brand accent. Re-run whenever the logo changes:

    python scripts/gen_nsis_banners.py
"""

from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
ICON = ROOT / "src-tauri" / "icons" / "128x128.png"
OUT_DIR = ROOT / "src-tauri" / "icons" / "nsis"
OUT_DIR.mkdir(parents=True, exist_ok=True)

BG = (30, 30, 46)        # Catppuccin Mocha base, matches app dark theme
ACCENT = (137, 180, 250) # Catppuccin Mocha blue, brand colour


def gen_header(out: Path) -> None:
    """150x57 banner at the top of each wizard page."""
    img = Image.new("RGB", (150, 57), BG)
    if ICON.exists():
        logo = Image.open(ICON).convert("RGBA").resize((45, 45), Image.LANCZOS)
        # Paste with alpha against the dark background
        canvas = Image.new("RGBA", (150, 57), (*BG, 255))
        canvas.alpha_composite(logo, (6, 6))
        img = canvas.convert("RGB")
    img.save(out, "BMP")


def gen_sidebar(out: Path) -> None:
    """164x314 decorative panel for the Welcome / Finish wizard pages."""
    img = Image.new("RGB", (164, 314), BG)
    # A horizontal accent stripe near the bottom for branding feel.
    for y in range(254, 264):
        for x in range(164):
            img.putpixel((x, y), ACCENT)
    if ICON.exists():
        logo = Image.open(ICON).convert("RGBA").resize((110, 110), Image.LANCZOS)
        canvas = Image.new("RGBA", (164, 314), (*BG, 255))
        canvas.alpha_composite(logo, (27, 70))
        # paint accent stripe again on the RGBA layer
        for y in range(254, 264):
            for x in range(164):
                canvas.putpixel((x, y), (*ACCENT, 255))
        img = canvas.convert("RGB")
    img.save(out, "BMP")


def main() -> None:
    if not ICON.exists():
        raise SystemExit(f"missing icon: {ICON}")
    header = OUT_DIR / "header.bmp"
    sidebar = OUT_DIR / "sidebar.bmp"
    gen_header(header)
    gen_sidebar(sidebar)
    print(f"wrote {header.relative_to(ROOT)}")
    print(f"wrote {sidebar.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
