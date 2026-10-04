#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
render_moviepy.py — build a placeholder video from a whitebox-video-storyboard
`layout.json` using MoviePy >= 2.0.

用法 / Usage:
    pip install "moviepy>=2" numpy pillow
    python render_moviepy.py layout.json -o out.mp4
    python render_moviepy.py layout.json -o preview.mp4 --preview      # 1/3 解析度、12fps 快速預覽
    python render_moviepy.py layout.json --frames-dir frames            # 每個場景輸出一張中間影格 PNG
    python render_moviepy.py layout.json --frames-dir f --transition-frames --no-video  # 也輸出轉場中間格 / also mid-transition frames
    python render_moviepy.py layout.json --font "C:/Windows/Fonts/msjh.ttc"

Every element in layout.json becomes one clip:
  * x, y, w, h       -> clip size + .with_position((x, y))   (pixels, origin top-left)
  * start, end       -> .with_start(start) / .with_duration(end - start) inside its scene
  * z                -> order inside CompositeVideoClip (low z = drawn first)
  * animation        -> CrossFadeIn/Out, sliding position functions, resized(pop), mask wipe (typewriter),
                        scroll_up (credits roll: bottom -> top over the element's time)
  * src (optional)   -> real ImageClip / VideoFileClip instead of a grey placeholder

Every scene may carry `transition` = {type, duration}: the transition INTO that scene.
It plays during the first `duration` seconds of the scene over a freeze frame of the
previous scene's last frame, so scene timing / total length do not change:
  crossfade, fade_black, slide_left, slide_up, wipe, zoom  (none = hard cut)

CJK fonts: TextClip needs a font *file* that contains Chinese glyphs, otherwise
you get empty boxes (tofu). Resolution order: --font, $WVS_FONT, layout["fonts"]["cjk"]
(relative to layout.json), ./fonts/*.otf next to this script, repo assets/fonts,
then common system fonts (Noto Sans CJK, Microsoft JhengHei, PingFang ...).
Note: for .ttc collections Pillow uses face index 0.
"""
from __future__ import annotations

import argparse
import json
import math
import os
import sys
import zlib
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from moviepy import (
    ColorClip,
    CompositeVideoClip,
    ImageClip,
    TextClip,
    VideoClip,
    VideoFileClip,
    concatenate_videoclips,
    vfx,
)

NO_LINE_START = set("，。、！？：；）」』】》〉,.!?:;)]}…ー～")
IMAGE_EXT = {".png", ".jpg", ".jpeg", ".webp", ".bmp", ".gif"}
VIDEO_EXT = {".mp4", ".mov", ".mkv", ".webm", ".avi", ".m4v"}
SYSTEM_FONTS = [
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    "C:/Windows/Fonts/msjh.ttc",
    "C:/Windows/Fonts/msyh.ttc",
    "C:/Windows/Fonts/mingliu.ttc",
    "/System/Library/Fonts/PingFang.ttc",
    "/System/Library/Fonts/STHeiti Medium.ttc",
    "/Library/Fonts/Arial Unicode.ttf",
]


# --------------------------------------------------------------------------- helpers
def hex_rgb(s: str, default=(128, 128, 128)) -> tuple[int, int, int]:
    s = (s or "").lstrip("#")
    try:
        if len(s) >= 6:
            return tuple(int(s[i : i + 2], 16) for i in (0, 2, 4))  # type: ignore[return-value]
        if len(s) == 3:
            return tuple(int(c, 16) * 17 for c in s)  # type: ignore[return-value]
    except ValueError:
        pass
    return default


def darker(c, f=0.7):
    return tuple(int(v * f) for v in c)


def resolve_font(cli_font: str | None, layout: dict, layout_dir: Path) -> str | None:
    cands: list[Path] = []
    if cli_font:
        cands.append(Path(cli_font))
    if os.environ.get("WVS_FONT"):
        cands.append(Path(os.environ["WVS_FONT"]))
    rel = (layout.get("fonts") or {}).get("cjk")
    if rel:
        cands.append(layout_dir / rel)
    here = Path(__file__).resolve().parent
    for d in (layout_dir / "fonts", here / "fonts", here.parent / "fonts", here.parent / "assets" / "fonts", here.parent.parent / "assets" / "fonts"):
        if d.is_dir():
            cands.extend(sorted(d.glob("*.otf")) + sorted(d.glob("*.ttf")) + sorted(d.glob("*.ttc")))
    cands.extend(Path(p) for p in SYSTEM_FONTS)
    for c in cands:
        if c.is_file():
            try:
                ImageFont.truetype(str(c), 12)
                return str(c)
            except Exception:
                continue
    print("[warn] no CJK font found / 找不到 CJK 字型 — Chinese may render as boxes; use --font", file=sys.stderr)
    return None


class Ctx:
    def __init__(self, layout: dict, layout_dir: Path, scale: float, font: str | None):
        self.layout = layout
        self.dir = layout_dir
        self.s = scale
        self.font = font
        cv = layout["canvas"]
        self.W = self.px(cv["width"], even=True)
        self.H = self.px(cv["height"], even=True)
        self._fonts: dict[int, ImageFont.FreeTypeFont] = {}

    def px(self, v, even=False) -> int:
        n = max(1, int(round(float(v) * self.s)))
        return n + (n % 2) if even else n

    def pil_font(self, size: int):
        if size not in self._fonts:
            self._fonts[size] = ImageFont.truetype(self.font, size) if self.font else ImageFont.load_default(size)
        return self._fonts[size]

    def asset(self, src: str) -> Path | None:
        if not src:
            return None
        p = Path(src)
        if not p.is_absolute():
            p = self.dir / p
        return p if p.is_file() else None


def wrap_text(ctx: Ctx, text: str, size: int, max_w: int) -> str:
    """Greedy wrap: break at spaces for Latin words, anywhere between CJK characters."""
    font = ctx.pil_font(size)
    out = []
    for para in text.split("\n"):
        tokens: list[str] = []
        for ch in para:
            if tokens and ch.isascii() and ch.isalnum() and tokens[-1][-1:].isascii() and tokens[-1][-1:].isalnum():
                tokens[-1] += ch
            else:
                tokens.append(ch)
        line = ""
        for t in tokens:
            # kinsoku: closing punctuation never starts a line, let it hang instead
            if font.getlength(line + t) > max_w and line and t not in NO_LINE_START:
                out.append(line.rstrip())
                line = "" if t == " " else t
            else:
                line += t
        out.append(line.rstrip())
    return "\n".join(out)


def _text_once(ctx: Ctx, text: str, w: int, size: int, color, align, stroke_color, sw: int):
    """TextClip whose image is tall enough for fonts with a large ascent (Noto CJK).

    MoviePy 2.x draws the first baseline at y = ascent but sizes the image from the
    text bbox, which clips descenders of CJK fonts. We therefore pass an explicit
    height = ascent + descent + (lines - 1) * line_spacing + 2 * stroke.
    Returns (clip, ink_top, ink_bottom) with ink offsets in clip pixels.
    """
    font = ctx.pil_font(size)
    interline = int(size * 0.15)
    wrapped = wrap_text(ctx, text, size, max(size, w - 2 * sw))
    n = wrapped.count("\n") + 1
    ascent, descent = font.getmetrics()
    line_spacing = font.getbbox("A", stroke_width=sw)[3] + sw + interline  # what Pillow uses
    height = int(ascent + descent + (n - 1) * line_spacing + 2 * sw + 2)
    clip = TextClip(
        font=ctx.font,
        text=wrapped,
        font_size=size,
        size=(w, height),
        color=color,
        stroke_color=stroke_color if sw > 0 else None,
        stroke_width=sw,
        method="label",
        text_align=align,
        horizontal_align=align,
        vertical_align="top",
        interline=interline,
        transparent=True,
    )
    _, t, _, b = font.getbbox("國Ag", anchor="ls")  # typical ink extent around a baseline
    ink_top = sw + ascent + t
    ink_bottom = sw + ascent + (n - 1) * line_spacing + b
    return clip, ink_top, ink_bottom


def text_clip(ctx: Ctx, text: str, w: int, h: int, size: float, color, align="center", stroke_color=None, stroke_width=0):
    """Text block for a w×h box: pre-wrapped (CJK-safe), shrunk to fit, visually centred.

    Returns a clip positioned *inside* the box (use it as a layer of a w×h composite)
    or None for empty text.
    """
    if not text or not text.strip() or w < 4 or h < 4:
        return None
    size = max(6, int(round(size)))
    sw = int(round(stroke_width))
    for _ in range(12):
        clip, top, bottom = _text_once(ctx, text, w, size, color, align, stroke_color, sw)
        if bottom - top <= h * 1.02 or size <= 8:
            break
        size = max(6, int(size * 0.9))  # shrink-to-fit like a designer would
    y = h / 2 - (top + bottom) / 2
    return clip.with_position((0, int(round(y))))


def at(clip, dx, dy):
    """Offset a clip returned by text_clip() (which is already centred in its own box)."""
    if clip is None:
        return None
    x0, y0 = clip.pos(0)
    return clip.with_position((int(x0 + dx), int(y0 + dy)))


def rgba_canvas(w, h):
    img = Image.new("RGBA", (max(1, w), max(1, h)), (0, 0, 0, 0))
    return img, ImageDraw.Draw(img)


def image_clip(img: Image.Image) -> ImageClip:
    return ImageClip(np.array(img), transparent=True)


def group(ctx: Ctx, w: int, h: int, layers: list) -> CompositeVideoClip:
    layers = [l for l in layers if l is not None]
    return CompositeVideoClip(layers, size=(w, h), bg_color=None)


def media_clip(ctx: Ctx, path: Path, w: int, h: int, duration: float, circle=False):
    ext = path.suffix.lower()
    if ext in VIDEO_EXT:
        c = VideoFileClip(str(path), audio=False)
        if c.duration < duration:
            c = c.with_effects([vfx.Loop(duration=duration)])
        c = c.subclipped(0, duration)
    else:
        c = ImageClip(str(path))
    # cover-fit then centre-crop to exactly w×h
    r = max(w / c.w, h / c.h)
    c = c.resized(r)
    c = c.cropped(x_center=c.w / 2, y_center=c.h / 2, width=w, height=h)
    if circle:
        m, d = rgba_canvas(w, h)
        d.ellipse([0, 0, w - 1, h - 1], fill=(255, 255, 255, 255))
        mask = ImageClip(np.array(m)[:, :, 3] / 255.0, is_mask=True)
        c = c.with_mask(mask)
    return c


# --------------------------------------------------------------------------- element builders
def build_element(ctx: Ctx, el: dict, scene_dur: float):
    kind = el.get("type", "shape")
    x, y, w, h = ctx.px(el["x"]), ctx.px(el["y"]), ctx.px(el["w"]), ctx.px(el["h"])
    start = float(el.get("start") or 0.0)
    end = el.get("end")
    end = scene_dur if end is None else min(float(end), scene_dur)
    dur = end - start
    if dur <= 0 or not el.get("visible", True):
        return None
    fs = float(el.get("font_size", 48)) * ctx.s
    fc = el.get("font_color", "#FFFFFF")
    bg = hex_rgb(el.get("bg_color", "#A0A0A0"))
    op = float(el.get("bg_opacity", 1.0))
    a = int(round(op * 255))
    align = el.get("align", "center")
    text = el.get("text", "")
    sc = el.get("stroke_color", "#000000")
    sw = float(el.get("stroke_width", 0)) * ctx.s
    pad = max(2, int(12 * ctx.s * min(ctx.layout["canvas"]["width"], ctx.layout["canvas"]["height"]) / 1080))
    shape = el.get("shape", "rect")
    src = ctx.asset(el.get("src", ""))
    img, d = rgba_canvas(w, h)
    layers: list = []

    def box(radius=0, ellipse=False, fill=(*bg, a)):
        if fill[3] == 0:
            return
        if ellipse:
            d.ellipse([0, 0, w - 1, h - 1], fill=fill)
        elif radius > 0:
            d.rounded_rectangle([0, 0, w - 1, h - 1], radius=min(radius, w // 2, h // 2), fill=fill)
        else:
            d.rectangle([0, 0, w - 1, h - 1], fill=fill)

    def main_text(rect=None, size=fs, al=align, color=fc):
        rx, ry, rw, rh = rect or (pad, 0, w - 2 * pad, h)
        return at(text_clip(ctx, text, rw, rh, size, color, al, sc, sw), rx, ry)

    if kind == "background":
        clip = ColorClip((w, h), color=bg).with_opacity(op)
        layers = [clip]
    elif kind in ("title", "subheading", "subtitle", "text_card", "watermark"):
        box(radius=int(18 * ctx.s) if kind == "text_card" else 0)
        layers = [image_clip(img), main_text()]
    elif kind == "options":
        opts = el.get("options") or ["A", "B", "C", "D"]
        n = len(opts)
        gap = int(14 * ctx.s)
        rh = max(4, int((h - gap * (n - 1)) / n))
        size = min(fs, rh * 0.62)
        answer = el.get("answer")
        layers = [None]
        for i, label in enumerate(opts):
            y0 = i * (rh + gap)
            fill = (120, 200, 120, max(a, 153)) if answer == i else (*bg, a)
            d.rounded_rectangle([0, y0, w - 1, y0 + rh], radius=min(rh, int(28 * ctx.s)) // 2, fill=fill)
            dd = int(rh * 0.7)
            bx, by = (rh - dd) // 2, y0 + (rh - dd) // 2
            d.ellipse([bx, by, bx + dd, by + dd], fill=(*darker(bg, 0.55), 255))
            letter = text_clip(ctx, chr(ord("A") + i % 26), dd, dd, size * 0.9, "#FFFFFF", "center")
            layers.append(at(letter, bx, by))
            t = text_clip(ctx, label, w - rh - int(pad * 1.5), rh, size, fc, align)
            layers.append(at(t, rh + pad // 2, y0))
        layers[0] = image_clip(img)
    elif kind == "lower_third":
        box(radius=int(4 * ctx.s))
        acc = max(2, min(int(16 * ctx.s), w // 6))
        d.rectangle([0, 0, acc, h - 1], fill=(250, 180, 40, 255))
        layers = [image_clip(img), main_text((acc + int(pad * 1.5), 0, w - acc - int(pad * 2.5), h))]
    elif kind in ("media_placeholder", "logo", "avatar_frame", "qr_code", "sticker") and src is not None:
        layers = [media_clip(ctx, src, w, h, dur, circle=(kind == "avatar_frame" and shape == "circle"))]
    elif kind in ("media_placeholder", "logo"):
        box()
        lc = (*darker(bg, 0.78), 255)
        d.line([0, 0, w, h], fill=lc, width=max(1, int(2 * ctx.s)))
        d.line([w, 0, 0, h], fill=lc, width=max(1, int(2 * ctx.s)))
        if kind == "media_placeholder":
            s = min(w, h) * 0.16
            cx, cy = w / 2, h / 2
            d.ellipse([cx - s * 1.25, cy - s * 1.25, cx + s * 1.25, cy + s * 1.25], fill=(255, 255, 255, 200))
            d.polygon([(cx - s * 0.5, cy - s * 0.75), (cx + s * 0.85, cy), (cx - s * 0.5, cy + s * 0.75)], fill=lc)
            label = text + (f"\n{el.get('src')}" if el.get("src") else "")
            t = text_clip(ctx, label, w - pad, int(h * 0.26), min(fs, h * 0.26 * 0.45), fc, "center")
            layers = [image_clip(img), at(t, pad // 2, int(h * 0.72))]
        else:
            layers = [image_clip(img), main_text(al="center")]
    elif kind == "avatar_frame":
        box(radius=int(min(w, h) * 0.12) if shape == "rounded" else 0, ellipse=(shape == "circle"))
        m = min(w, h)
        sil = (*darker(bg, 0.7), a)
        d.ellipse([w / 2 - m * 0.16, h / 2 - m * 0.3, w / 2 + m * 0.16, h / 2 + m * 0.02], fill=sil)
        d.rounded_rectangle([w / 2 - m * 0.3, h / 2 + m * 0.06, w / 2 + m * 0.3, h / 2 + m * 0.42], radius=int(m * 0.18), fill=sil)
        layers = [image_clip(img), main_text((0, int(h * 0.78), w, int(h * 0.2)), size=min(fs, h * 0.16), al="center")]
    elif kind == "progress_bar":
        track, dt = rgba_canvas(w, h)
        dt.rounded_rectangle([0, 0, w - 1, h - 1], radius=h // 2, fill=(*bg, a))
        fill_rgb = hex_rgb(fc, (250, 180, 40))
        track_np = np.array(track).astype(np.float32)

        def frame_rgba(t):
            p = min(1.0, max(0.0, t / dur))
            arr = track_np.copy()
            fw = int(w * p)
            if fw > 0:
                arr[:, :fw, :3] = fill_rgb
                arr[:, :fw, 3] = 255
            return arr

        rgb = VideoClip(frame_function=lambda t: frame_rgba(t)[:, :, :3].astype(np.uint8), duration=dur)
        mask = VideoClip(frame_function=lambda t: frame_rgba(t)[:, :, 3] / 255.0, is_mask=True, duration=dur)
        layers = [rgb.with_mask(mask)]
    elif kind == "countdown":
        circle = shape == "circle"
        box(radius=int(12 * ctx.s), ellipse=circle)
        ring = hex_rgb(fc, (30, 30, 30))
        lw = max(2, int(8 * ctx.s))
        if circle:
            d.ellipse([lw // 2, lw // 2, w - 1 - lw // 2, h - 1 - lw // 2], outline=(*ring, 255), width=lw)
        else:
            d.rounded_rectangle([lw // 2, lw // 2, w - 1 - lw // 2, h - 1 - lw // 2], radius=int(12 * ctx.s), outline=(*ring, 255), width=lw)
        try:
            n0 = int(text.strip())
        except ValueError:
            n0 = int(math.ceil(dur))
        n0 = max(0, n0)
        step = dur / (n0 + 1) if n0 > 0 else dur
        layers = [image_clip(img)]
        for i in range(n0 + 1):
            t = text_clip(ctx, str(n0 - i), w - 2 * pad, h, fs, fc, "center")
            if t is not None:
                layers.append(at(t, pad, 0).with_start(i * step).with_duration(step))
        g = group(ctx, w, h, layers).with_duration(dur)
        return finish(ctx, g, el, x, y, w, h, start, dur)
    elif kind == "cta_button":
        box(radius=0 if shape == "rect" else h // 2, ellipse=(shape == "circle"))
        layers = [image_clip(img), main_text()]
    elif kind == "callout_arrow":
        col = (*bg, max(a, 38))
        direction = el.get("direction", "right")
        if direction == "right":
            lab, p0, p1 = (0, 0, int(w * 0.58), h), (w * 0.58, h / 2), (w, h / 2)
        elif direction == "left":
            lab, p0, p1 = (int(w * 0.42), 0, int(w * 0.58), h), (w * 0.42, h / 2), (0, h / 2)
        elif direction == "down":
            lab, p0, p1 = (0, 0, w, int(h * 0.55)), (w / 2, h * 0.55), (w / 2, h)
        else:
            lab, p0, p1 = (0, int(h * 0.45), w, int(h * 0.55)), (w / 2, h * 0.45), (w / 2, 0)
        horiz = direction in ("left", "right")
        thick = max(3.0 * ctx.s, (h if horiz else w) * 0.12)
        head = thick * 2.6
        L = max(1e-3, math.dist(p0, p1))
        dx, dy = (p1[0] - p0[0]) / L, (p1[1] - p0[1]) / L
        hb = (p1[0] - dx * min(head, L), p1[1] - dy * min(head, L))
        d.line([p0, hb], fill=col, width=int(thick))
        d.polygon([p1, (hb[0] - dy * head * 0.6, hb[1] + dx * head * 0.6), (hb[0] + dy * head * 0.6, hb[1] - dx * head * 0.6)], fill=col)
        lx, ly, lw_, lh_ = lab
        d.rounded_rectangle([lx, ly, lx + lw_ - 1, ly + lh_ - 1], radius=int(10 * ctx.s), fill=col)
        layers = [image_clip(img), main_text((lx + pad, ly, lw_ - 2 * pad, lh_), al="center")]
    elif kind == "qr_code":
        d.rectangle([0, 0, w - 1, h - 1], fill=(*bg, 255))
        m = min(w, h)
        q0 = m * 0.06
        cell = (m - 2 * q0) / 25
        ox, oy = (w - m) / 2 + q0, (h - m) / 2 + q0
        dark = (*hex_rgb(fc, (20, 20, 20)), 255)
        rng = np.random.default_rng(zlib.crc32(el.get("id", "").encode("utf-8")))
        for gy in range(25):
            for gx in range(25):
                finder = (gx < 8 and gy < 8) or (gx >= 17 and gy < 8) or (gx < 8 and gy >= 17)
                if not finder and rng.random() < 0.4:
                    d.rectangle([ox + gx * cell, oy + gy * cell, ox + (gx + 1) * cell, oy + (gy + 1) * cell], fill=dark)
        for fx, fy in ((0, 0), (18, 0), (0, 18)):
            for k, colr in ((0, dark), (1, (*bg, 255)), (2, dark)):
                d.rectangle([ox + (fx + k) * cell, oy + (fy + k) * cell, ox + (fx + 7 - k) * cell, oy + (fy + 7 - k) * cell], fill=colr)
        layers = [image_clip(img)]
    elif kind == "sticker":
        box(ellipse=True)
        layers = [image_clip(img), main_text((0, 0, w, h), al="center")]
    else:  # "shape" and any future / unknown type -> labelled box
        box(radius=int(min(w, h) * 0.15) if shape == "rounded" else 0, ellipse=(shape == "circle"))
        layers = [image_clip(img), main_text()]

    g = group(ctx, w, h, layers).with_duration(dur)
    if kind == "watermark":
        g = g.with_opacity(0.75)
    return finish(ctx, g, el, x, y, w, h, start, dur)


def ease(p: float) -> float:
    p = min(1.0, max(0.0, p))
    return 1 - (1 - p) ** 3


def finish(ctx: Ctx, clip, el: dict, x: int, y: int, w: int, h: int, start: float, dur: float):
    """Apply position, timing and the animation hint."""
    anim = el.get("animation", "none")
    t_in = min(0.5, dur / 3)
    effects = []
    pos = (x, y)
    if anim in ("fade_in", "fade_in_out", "slide_up", "slide_left"):
        effects.append(vfx.CrossFadeIn(t_in))
    if anim in ("fade_out", "fade_in_out"):
        effects.append(vfx.CrossFadeOut(t_in))
    if anim == "slide_up":
        off = ctx.H * 0.08
        pos = lambda t: (x, y + off * (1 - ease(t / t_in)))  # noqa: E731
    elif anim == "slide_left":
        off = ctx.W * 0.1
        pos = lambda t: (x + off * (1 - ease(t / t_in)), y)  # noqa: E731
    elif anim == "pop":
        t_pop = min(0.35, dur / 3)

        def k(t):
            p = min(1.0, t / t_pop)
            return max(0.05, 0.5 + 0.5 * ease(p) + 0.12 * math.sin(math.pi * p))

        clip = clip.resized(lambda t: k(t) if t < t_pop else 1.0)
        pos = lambda t: (x + w * (1 - (k(t) if t < t_pop else 1.0)) / 2, y + h * (1 - (k(t) if t < t_pop else 1.0)) / 2)  # noqa: E731
    elif anim == "scroll_up":
        # credits roll: from just below the frame to just above it over the element's time
        span = max(dur, 0.01)
        pos = lambda t: (x, ctx.H - (ctx.H + h) * min(1.0, t / span))  # noqa: E731
    elif anim == "typewriter" and clip.mask is not None:
        t_type = min(1.5, dur * 0.6)

        def wipe(get_frame, t):
            f = get_frame(t).copy()
            cut = int(f.shape[1] * min(1.0, t / t_type))
            f[:, cut:] = 0
            return f

        clip = clip.with_mask(clip.mask.transform(wipe))
    if effects:
        clip = clip.with_effects(effects)
    return clip.with_position(pos).with_start(start).with_duration(dur)


# --------------------------------------------------------------------------- scenes
def build_scene(ctx: Ctx, scene: dict):
    dur = float(scene["duration"])
    base = ColorClip((ctx.W, ctx.H), color=hex_rgb(scene.get("background", "#EEEEEE"))).with_duration(dur)
    clips = [base]
    for el in sorted(scene.get("elements", []), key=lambda e: e.get("z", 0)):
        try:
            c = build_element(ctx, el, dur)
        except Exception as exc:  # keep going: one broken element should not kill the render
            print(f"[warn] {scene.get('id')}/{el.get('id')}: {exc}", file=sys.stderr)
            c = None
        if c is not None:
            clips.append(c)
    return CompositeVideoClip(clips, size=(ctx.W, ctx.H)).with_duration(dur)


# --------------------------------------------------------------------------- transitions
def apply_transition(ctx: Ctx, prev, cur, tr: dict | None, fps: float):
    """Return `cur` with the transition INTO it applied (same duration as `cur`)."""
    kind = (tr or {}).get("type", "none") or "none"
    d = float((tr or {}).get("duration", 0) or 0)
    if kind == "none" or d <= 0:
        return cur
    dur = cur.duration
    d = min(d, dur * 0.9)
    if kind == "fade_black":
        return cur.with_effects([vfx.FadeIn(d)])
    if prev is None:  # nothing to transition from
        return cur
    W, H = ctx.W, ctx.H
    tail = prev.to_ImageClip(t=max(0.0, prev.duration - 1.0 / fps)).with_duration(d)

    def k(t):
        return ease(t / d) if t < d else 1.0

    if kind == "crossfade":
        top = cur.with_effects([vfx.CrossFadeIn(d)])
    elif kind == "slide_left":
        top = cur.with_position(lambda t: (int(round(W * (1 - k(t)))), 0))
    elif kind == "slide_up":
        top = cur.with_position(lambda t: (0, int(round(H * (1 - k(t))))))
    elif kind == "wipe":
        def mask_frame(t):
            m = np.zeros((H, W), dtype=float)
            m[:, : int(round(W * min(1.0, t / d)))] = 1.0
            return m

        top = cur.with_mask(VideoClip(mask_frame, is_mask=True, duration=dur))
    elif kind == "zoom":
        def z(t):
            return 0.7 + 0.3 * k(t)

        top = (
            cur.resized(z)
            .with_position(lambda t: (int(round(W * (1 - z(t)) / 2)), int(round(H * (1 - z(t)) / 2))))
            .with_effects([vfx.CrossFadeIn(d)])
        )
    else:
        print(f"[warn] unknown transition {kind!r}, using a hard cut", file=sys.stderr)
        return cur
    return CompositeVideoClip([tail, top], size=(W, H)).with_duration(dur)


def main(argv=None):
    ap = argparse.ArgumentParser(description="Render a whitebox-video-storyboard layout.json with MoviePy >= 2")
    ap.add_argument("layout", nargs="?", default="layout.json")
    ap.add_argument("-o", "--out", default="storyboard.mp4")
    ap.add_argument("--scale", type=float, default=1.0, help="resolution multiplier, e.g. 0.5")
    ap.add_argument("--fps", type=int, default=None)
    ap.add_argument("--font", default=None, help="CJK font file (.otf/.ttf/.ttc)")
    ap.add_argument("--scenes", default=None, help="comma-separated 1-based scene numbers, e.g. 1,3")
    ap.add_argument("--preview", action="store_true", help="fast preview: scale 1/3, 12 fps, ultrafast preset")
    ap.add_argument("--frames-dir", default=None, help="also save the middle frame of every scene as PNG")
    ap.add_argument("--no-video", action="store_true", help="skip writing the video (use with --frames-dir)")
    ap.add_argument(
        "--transition-frames", action="store_true", help="with --frames-dir: also save the middle of every scene transition"
    )
    ap.add_argument("--no-transitions", action="store_true", help="ignore scene transitions (hard cuts)")
    args = ap.parse_args(argv)

    layout_path = Path(args.layout).resolve()
    layout = json.loads(layout_path.read_text(encoding="utf-8"))
    scale = args.scale
    fps = args.fps or layout.get("fps") or layout.get("canvas", {}).get("fps") or 30
    if args.preview:
        scale = min(scale, 1 / 3)
        fps = args.fps or 12
    font = resolve_font(args.font, layout, layout_path.parent)
    print(f"[info] font: {font}")
    ctx = Ctx(layout, layout_path.parent, scale, font)

    scenes = layout["scenes"]
    if args.scenes:
        keep = {int(s) for s in args.scenes.split(",") if s.strip()}
        scenes = [s for i, s in enumerate(scenes, 1) if i in keep]
    raw = [build_scene(ctx, s) for s in scenes]
    scene_clips = []
    for i, (s, c) in enumerate(zip(scenes, raw)):
        tr = None if args.no_transitions else s.get("transition")
        scene_clips.append(apply_transition(ctx, raw[i - 1] if i > 0 else None, c, tr, fps))
    n_tr = sum(1 for a, b in zip(raw, scene_clips) if a is not b)
    video = concatenate_videoclips(scene_clips, method="chain")
    print(f"[info] {len(scene_clips)} scenes ({n_tr} transitions), {video.duration:.2f}s, {ctx.W}x{ctx.H} @ {fps}fps")

    if args.frames_dir:
        out = Path(args.frames_dir)
        out.mkdir(parents=True, exist_ok=True)
        t0 = 0.0
        for i, (s, c) in enumerate(zip(scenes, scene_clips)):
            t = t0 + min(c.duration - 0.05, max(0.0, c.duration * 0.75))
            p = out / f"{s.get('id', 'scene')}_frame.png"
            video.save_frame(str(p), t=t)
            print(f"[info] frame {p} @ {t:.2f}s")
            tr = s.get("transition") or {}
            if args.transition_frames and c is not raw[i] and tr.get("duration"):
                tt = t0 + min(float(tr["duration"]), c.duration * 0.9) / 2
                p = out / f"{s.get('id', 'scene')}_transition_{tr.get('type')}.png"
                video.save_frame(str(p), t=tt)
                print(f"[info] transition frame {p} @ {tt:.2f}s")
            t0 += c.duration
    if not args.no_video:
        video.write_videofile(
            args.out,
            fps=fps,
            codec="libx264",
            audio=False,
            preset="ultrafast" if args.preview else "medium",
            threads=os.cpu_count() or 2,
            logger="bar",
        )
        print(f"[done] {args.out}")


if __name__ == "__main__":
    main()
