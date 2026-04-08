from __future__ import annotations

import json
from pathlib import Path


ROOT = Path("/Users/peak/coding/tools/agent-undo/paper/eval")
RESULTS = ROOT / "results.json"
RESULTS_REPO = ROOT / "results_repo_snapshot.json"
OUT_SVG = ROOT / "microeval.svg"


WIDTH = 1100
HEIGHT = 430
PANEL_W = 450
PANEL_H = 250
PANEL_Y = 110
LEFT_X = 70
RIGHT_X = 580


def rect(x: float, y: float, w: float, h: float, fill: str, rx: int = 8) -> str:
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="{fill}" />'


def text(x: float, y: float, value: str, size: int = 20, fill: str = "#f5f5f5", weight: str = "500", anchor: str = "start") -> str:
    return (
        f'<text x="{x}" y="{y}" fill="{fill}" font-size="{size}" font-family="Inter, Arial, sans-serif" '
        f'font-weight="{weight}" text-anchor="{anchor}">{value}</text>'
    )


def line(x1: float, y1: float, x2: float, y2: float, stroke: str = "#2f2f2f", width: int = 1) -> str:
    return f'<line x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}" stroke="{stroke}" stroke-width="{width}" />'


def make_panel(x: int, title: str) -> list[str]:
    return [
        rect(x, PANEL_Y, PANEL_W, PANEL_H, "#111111", rx=18),
        text(x + 20, PANEL_Y + 34, title, size=22, weight="700"),
    ]


def render_grouped_bars(
    x: int,
    title: str,
    groups: list[str],
    series_names: list[str],
    series_values: list[list[float]],
    max_value: float,
    formatter,
    y_label: str,
) -> list[str]:
    parts: list[str] = []
    parts += make_panel(x, title)
    base_y = PANEL_Y + PANEL_H - 42
    usable_h = PANEL_H - 90
    gap = 26
    bar_w = 34
    group_gap = 48
    start_x = x + 34
    colors = ["#f59e0b", "#fbbf24"]

    for gi, group in enumerate(groups):
        group_start = start_x + gi * (len(series_names) * bar_w + gap + group_gap)
        center = group_start + (len(series_names) * bar_w + gap) / 2 - gap / 2
        for si, name in enumerate(series_names):
            value = series_values[si][gi]
            bx = group_start + si * (bar_w + gap)
            bh = 0 if max_value == 0 else usable_h * (value / max_value)
            by = base_y - bh
            parts.append(rect(bx, by, bar_w, bh, colors[si], rx=8))
            parts.append(text(bx + bar_w / 2, by - 10, formatter(value), size=14, weight="700", anchor="middle"))
        parts.append(text(center, base_y + 26, group, size=16, fill="#d4d4d4", anchor="middle"))

    for frac in [0.25, 0.5, 0.75, 1.0]:
        gy = base_y - usable_h * frac
        parts.append(line(x + 18, gy, x + PANEL_W - 18, gy, stroke="#2c2c2c", width=1))

    # legend
    lx = x + 20
    ly = PANEL_Y + 58
    parts.append(rect(lx, ly, 14, 14, colors[0], rx=3))
    parts.append(text(lx + 22, ly + 12, series_names[0], size=14, fill="#d4d4d4"))
    parts.append(rect(lx + 120, ly, 14, 14, colors[1], rx=3))
    parts.append(text(lx + 142, ly + 12, series_names[1], size=14, fill="#d4d4d4"))
    parts.append(text(x + PANEL_W - 20, PANEL_Y + 34, y_label, size=14, fill="#bfbfbf", anchor="end"))

    return parts


def main() -> None:
    with RESULTS.open() as f:
        data = json.load(f)
    with RESULTS_REPO.open() as f:
        repo = json.load(f)

    latency_labels = ["init", "detect", "oops"]
    latency_series = [
        [data["init_ms"], data["write_detection_ms"], data["oops_ms"]],
        [repo["init_ms"], repo["write_detection_ms"], repo["oops_ms"]],
    ]

    storage_labels = ["timeline db", "objects"]
    storage_series = [
        [data["timeline_db_bytes"] / 1024 / 1024, data["object_store_bytes"] / 1024 / 1024],
        [repo["timeline_db_bytes"] / 1024 / 1024, repo["object_store_bytes"] / 1024 / 1024],
    ]

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}" viewBox="0 0 {WIDTH} {HEIGHT}" fill="none">',
        rect(0, 0, WIDTH, HEIGHT, "#0a0a0a", rx=0),
        text(70, 56, "agent-undo micro-evaluation", size=30, weight="800"),
        text(70, 82, "Synthetic repository and repo-snapshot measurements from the current release build", size=17, fill="#bfbfbf"),
    ]

    parts += render_grouped_bars(
        LEFT_X,
        "Latency (ms)",
        latency_labels,
        ["synthetic", "repo snapshot"],
        latency_series,
        max(max(s) for s in latency_series) * 1.18,
        lambda v: f"{v:.1f}",
        "milliseconds",
    )

    parts += render_grouped_bars(
        RIGHT_X,
        "Store growth (MiB)",
        storage_labels,
        ["synthetic", "repo snapshot"],
        storage_series,
        max(max(s) for s in storage_series) * 1.35,
        lambda v: f"{v:.2f}",
        "MiB",
    )

    parts.append(text(70, HEIGHT - 24, "Release binary: 4.85 MiB  ·  synthetic: 87 events  ·  repo snapshot: 94 files / 99 events", size=15, fill="#d4d4d4"))
    parts.append("</svg>")

    OUT_SVG.write_text("\n".join(parts))


if __name__ == "__main__":
    main()
