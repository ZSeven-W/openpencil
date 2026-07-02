#!/usr/bin/env python3
"""Per-node render-parity diff: Pencil baseline render vs OpenPencil Rust render.

Both sides export one PNG per node id (filename stem = node id):
  baseline/  <- Pencil export_nodes(png@2x)
  ours/      <- openpencil-desktop --render-shots (same .pen converted via pen2op)

Because pen2op preserves node ids 1:1, the same <id>.png exists on both sides,
so parity is measured per node. Metric is perceptual (the agreed bar — both
stacks are Skia, so AA/font-hinting/taffy-vs-flexbox guarantee sub-pixel and
position deltas; byte-exact is the wrong target):

  - bbox parity : ratio of rendered pixel dims (ours vs baseline). Divergence
                  here = LAYOUT gap (taffy != Pencil JS flexbox).
  - SSIM        : windowed structural similarity on luma, after resizing both
                  to the baseline's dims (isolates PAINT content from size).
  - dE76        : mean CIE76 colour difference in LAB (isolates fill/variable
                  resolution — the "white circle" class).

Verdict per node:
  MISSING : node rendered on one side only (structural)
  LAYOUT  : bbox dims differ > --bbox-tol (paint may still match once resized)
  PAINT   : bbox ok but SSIM < --ssim-min or dE76 > --de-max
  PASS    : within all tolerances

Usage:
  diff_nodes.py <baseline_dir> <ours_dir> [--out report] [--ssim-min 0.90]
                [--de-max 5.0] [--bbox-tol 0.03]
"""
import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy.ndimage import uniform_filter


def load_rgb(path):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.float64)


def to_luma(rgb):
    return rgb @ np.array([0.299, 0.587, 0.114])


def ssim(a, b, win=7):
    """Windowed SSIM on two equal-shape luma arrays (0..255)."""
    C1 = (0.01 * 255) ** 2
    C2 = (0.03 * 255) ** 2
    mu_a = uniform_filter(a, win)
    mu_b = uniform_filter(b, win)
    mu_a2, mu_b2, mu_ab = mu_a * mu_a, mu_b * mu_b, mu_a * mu_b
    var_a = uniform_filter(a * a, win) - mu_a2
    var_b = uniform_filter(b * b, win) - mu_b2
    cov = uniform_filter(a * b, win) - mu_ab
    num = (2 * mu_ab + C1) * (2 * cov + C2)
    den = (mu_a2 + mu_b2 + C1) * (var_a + var_b + C2)
    return float(np.clip(num / den, 0, 1).mean())


def srgb_to_lab(rgb):
    """rgb in 0..255 -> CIE L*a*b* (D65)."""
    c = rgb / 255.0
    lin = np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)
    m = np.array([
        [0.4124, 0.3576, 0.1805],
        [0.2126, 0.7152, 0.0722],
        [0.0193, 0.1192, 0.9505],
    ])
    xyz = lin @ m.T
    white = np.array([0.95047, 1.0, 1.08883])
    xyz = xyz / white
    d = 6 / 29
    f = np.where(xyz > d ** 3, np.cbrt(xyz), xyz / (3 * d * d) + 4 / 29)
    L = 116 * f[..., 1] - 16
    a = 500 * (f[..., 0] - f[..., 1])
    bb = 200 * (f[..., 1] - f[..., 2])
    return np.stack([L, a, bb], axis=-1)


def mean_de76(a_rgb, b_rgb):
    la, lb = srgb_to_lab(a_rgb), srgb_to_lab(b_rgb)
    return float(np.sqrt(((la - lb) ** 2).sum(-1)).mean())


def resize_to(arr, hw):
    h, w = hw
    img = Image.fromarray(arr.astype(np.uint8)).resize((w, h), Image.BILINEAR)
    return np.asarray(img, dtype=np.float64)


def diff_node(base_path, ours_path, args):
    base = load_rgb(base_path)
    ours = load_rgb(ours_path)
    bh, bw = base.shape[:2]
    oh, ow = ours.shape[:2]
    dim_ratio_w = ow / bw if bw else 0.0
    dim_ratio_h = oh / bh if bh else 0.0
    bbox_off = max(abs(dim_ratio_w - 1), abs(dim_ratio_h - 1))
    bbox_ok = bbox_off <= args.bbox_tol

    # Normalize ours onto baseline dims to isolate paint from size.
    ours_n = resize_to(ours, (bh, bw)) if (oh, ow) != (bh, bw) else ours
    # Pad luma to >= window so uniform_filter is meaningful.
    s = ssim(to_luma(base), to_luma(ours_n)) if min(bh, bw) >= 3 else 1.0
    de = mean_de76(base, ours_n)

    # Color/fill fidelity INDEPENDENT of position. A heavy 24x24 downsample
    # averages out text + sub-pixel element-position drift, leaving only the
    # large fills / gradients / backgrounds. Low here ⇒ the colours are right,
    # so any SSIM / full-res dE loss is internal position drift = LAYOUT
    # (taffy vs Pencil flexbox), NOT a paint bug (fill/variable/shader). High
    # here ⇒ a genuine fill divergence = PAINT. This separates the goal's two
    # 归因 buckets, which a bbox-only split conflates (a spatially-varying
    # gradient at a drifted position reads as huge full-res dE while the fill
    # itself is pixel-correct — verified visually on XUa6B's Programs cards).
    color_de = mean_de76(resize_to(base, (24, 24)), resize_to(ours_n, (24, 24)))

    if not bbox_ok:
        verdict = "LAYOUT"
    elif color_de > args.de_max:
        verdict = "PAINT"
    elif s >= args.ssim_min and de <= args.de_max:
        verdict = "PASS"
    else:
        # Fills match (low downsampled dE); the SSIM / edge-dE loss is
        # element-position drift between the two flex engines = LAYOUT.
        verdict = "LAYOUT"
    return {
        "verdict": verdict,
        "ssim": round(s, 4),
        "de76": round(de, 2),
        "color_de": round(color_de, 2),
        "bbox_off": round(bbox_off, 4),
        "base_dims": [bw, bh],
        "ours_dims": [ow, oh],
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("baseline")
    ap.add_argument("ours")
    ap.add_argument("--out", default="report")
    ap.add_argument("--ssim-min", type=float, default=0.90)
    ap.add_argument("--de-max", type=float, default=5.0)
    ap.add_argument("--bbox-tol", type=float, default=0.03)
    args = ap.parse_args()

    base_dir, ours_dir = Path(args.baseline), Path(args.ours)
    base_ids = {p.stem: p for p in base_dir.glob("*.png")}
    ours_ids = {p.stem: p for p in ours_dir.glob("*.png")}
    all_ids = sorted(set(base_ids) | set(ours_ids))

    rows = []
    for nid in all_ids:
        if nid not in base_ids or nid not in ours_ids:
            rows.append({"id": nid, "verdict": "MISSING",
                         "side": "ours" if nid in ours_ids else "baseline"})
            continue
        r = diff_node(base_ids[nid], ours_ids[nid], args)
        r["id"] = nid
        rows.append(r)

    counts = {}
    for r in rows:
        counts[r["verdict"]] = counts.get(r["verdict"], 0) + 1
    total = len(rows)
    passed = counts.get("PASS", 0)
    summary = {
        "total_nodes": total,
        "pass": passed,
        "pass_rate": round(passed / total, 3) if total else 0.0,
        "counts": counts,
        "thresholds": {"ssim_min": args.ssim_min, "de_max": args.de_max,
                       "bbox_tol": args.bbox_tol},
    }

    Path(f"{args.out}.json").write_text(
        json.dumps({"summary": summary, "nodes": rows}, indent=2))

    # Markdown: worst first (LAYOUT/PAINT/MISSING before PASS), then by ssim.
    order = {"MISSING": 0, "LAYOUT": 1, "PAINT": 2, "PASS": 3}
    rows_sorted = sorted(rows, key=lambda r: (order.get(r["verdict"], 9),
                                              r.get("ssim", 1.0)))
    lines = [f"# Render parity: `{base_dir.name}` (Pencil) vs ours\n",
             f"- nodes: **{total}** · PASS **{passed}** "
             f"({summary['pass_rate']*100:.1f}%) · {counts}\n",
             f"- thresholds: SSIM≥{args.ssim_min} dE76≤{args.de_max} "
             f"bbox±{args.bbox_tol*100:.0f}%\n",
             "\n| node | verdict | SSIM | dE76 | colorDE | bbox_off | base | ours |",
             "|---|---|---|---|---|---|---|---|"]
    for r in rows_sorted:
        if r["verdict"] == "MISSING":
            lines.append(f"| `{r['id']}` | MISSING ({r['side']}) | — | — | — | — | — | — |")
        else:
            bd = "×".join(map(str, r["base_dims"]))
            od = "×".join(map(str, r["ours_dims"]))
            lines.append(f"| `{r['id']}` | {r['verdict']} | {r['ssim']} | "
                         f"{r['de76']} | {r.get('color_de', '—')} | {r['bbox_off']} | {bd} | {od} |")
    Path(f"{args.out}.md").write_text("\n".join(lines) + "\n")

    print(json.dumps(summary, indent=2))
    print(f"\nwrote {args.out}.json + {args.out}.md")


if __name__ == "__main__":
    sys.exit(main())
