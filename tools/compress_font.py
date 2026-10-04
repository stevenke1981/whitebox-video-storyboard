#!/usr/bin/env python3
"""Regenerate assets/fonts/NotoSansCJKtc-Subset.otf.br (embedded into the binary).

The app embeds the Brotli-compressed font (≈5.0 MB instead of 7.5 MB) and
decompresses it once at startup (~0.1 s). Run after changing the .otf:

    pip install brotli && python tools/compress_font.py
"""
from pathlib import Path

import brotli

src = Path(__file__).resolve().parent.parent / "assets" / "fonts" / "NotoSansCJKtc-Subset.otf"
data = src.read_bytes()
out = src.with_suffix(".otf.br")
out.write_bytes(brotli.compress(data, quality=11, lgwin=24))
print(f"{src.name}: {len(data):,} -> {out.name}: {out.stat().st_size:,} bytes")
