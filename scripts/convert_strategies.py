#!/usr/bin/env python3
"""Convert Flowseal zapret-discord-youtube strategy .bat files into
plain-text winws argument templates used by zapret-gui.

Input : _ref/zapret-discord-youtube/general*.bat
Output: strategies/<slug>.txt  (raw args with %BIN%/%LISTS%/%GameFilter*% placeholders)
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REF = ROOT / "_ref" / "zapret-discord-youtube"
OUT = ROOT / "strategies"

SLUG_MAP = {
    "general": "general",
    "general (ALT)": "alt",
    "general (ALT2)": "alt2",
    "general (ALT3)": "alt3",
    "general (ALT4)": "alt4",
    "general (ALT5)": "alt5",
    "general (ALT6)": "alt6",
    "general (ALT7)": "alt7",
    "general (ALT8)": "alt8",
    "general (ALT9)": "alt9",
    "general (ALT10)": "alt10",
    "general (ALT11)": "alt11",
    "general (ALT12)": "alt12",
    "general (ALT13)": "alt13",
    "general (EXP)": "exp",
    "general (SIMPLE FAKE)": "simple-fake",
    "general (SIMPLE FAKE ALT)": "simple-fake-alt",
    "general (SIMPLE FAKE ALT2)": "simple-fake-alt2",
    "general (FAKE TLS AUTO)": "fake-tls-auto",
    "general (FAKE TLS AUTO ALT)": "fake-tls-auto-alt",
    "general (FAKE TLS AUTO ALT2)": "fake-tls-auto-alt2",
    "general (FAKE TLS AUTO ALT3)": "fake-tls-auto-alt3",
}

ORDER = [
    "general", "alt", "alt2", "alt3", "alt4", "alt5", "alt6", "alt7",
    "alt8", "alt9", "alt10", "alt11", "alt12", "alt13",
    "simple-fake", "simple-fake-alt", "simple-fake-alt2",
    "fake-tls-auto", "fake-tls-auto-alt", "fake-tls-auto-alt2", "fake-tls-auto-alt3",
    "exp",
]


def extract_args(bat_path: Path) -> str:
    text = bat_path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()

    capture = False
    parts: list[str] = []
    for raw in lines:
        line = raw.rstrip()
        if not capture:
            if "winws.exe" in line and line.lstrip().lower().startswith("start"):
                # start "title" /min "C:\...\winws.exe" ARGS...
                m = re.search(
                    r'winws\.exe"\s+(.*)$', line, flags=re.IGNORECASE
                )
                if m:
                    rest = m.group(1).strip()
                    if rest.endswith("^"):
                        rest = rest[:-1].rstrip()
                    if rest:
                        parts.append(rest)
                capture = True
            continue
        # continuation lines: strip trailing ^
        stripped = line.rstrip()
        if stripped.endswith("^"):
            stripped = stripped[:-1].rstrip()
        if stripped:
            parts.append(stripped)

    joined = " ".join(parts)
    # cmd-escape artifacts: ^! -> ! (winws wants a raw exclamation)
    joined = joined.replace("^!", "!")
    # normalize whitespace
    joined = re.sub(r"\s+", " ", joined).strip()
    return joined


def main() -> int:
    if not REF.is_dir():
        print(f"reference repo not found: {REF}", file=sys.stderr)
        return 1

    OUT.mkdir(parents=True, exist_ok=True)

    written = []
    for bat in sorted(REF.glob("general*.bat")):
        stem = bat.stem  # e.g. "general (ALT10)"
        slug = SLUG_MAP.get(stem)
        if slug is None:
            print(f"skip unknown strategy file: {bat.name}")
            continue
        args = extract_args(bat)
        if not args:
            print(f"empty args for {bat.name}, skipping")
            continue
        out_path = OUT / f"{slug}.txt"
        header = (
            f"# strategy: {stem}\n"
            f"# source: Flowseal/zapret-discord-youtube {bat.name}\n"
            f"# placeholders: %BIN% %LISTS% %GameFilterTCP% %GameFilterUDP%\n"
        )
        out_path.write_text(header + args + "\n", encoding="utf-8")
        written.append(slug)
        print(f"ok: {bat.name} -> strategies/{slug}.txt ({len(args)} chars)")

    # stable index file for UI ordering
    index = [s for s in ORDER if s in written]
    # append any extras not in ORDER (sorted)
    extras = sorted(set(written) - set(ORDER))
    index.extend(extras)
    (OUT / "index.txt").write_text("\n".join(index) + "\n", encoding="utf-8")
    print(f"\n{len(written)} strategies converted, index written")
    return 0


if __name__ == "__main__":
    sys.exit(main())
