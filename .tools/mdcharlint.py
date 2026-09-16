#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""mdcharlint.py：Markdown 技术文档禁用字符检查（G005）。

用法：uv run --script .tools/mdcharlint.py [文件或目录 ...]（缺省扫全仓 docs 与根目录 md）
豁免三层（ohmypwsh 总台落地经验，2026-09-02）：
  1. 豁免区（围栏代码块、行内代码、链接目标、裸 URL）行内掩掉；
  2. SKIP_DIRS 历史归档与第三方目录整体跳过（diary/proven 不回改惯例、
     node_modules 为上游材料）；
  3. 封闭豁免清单 .tools/md-char-allow.txt（若在）：只减不增；D10 清零后已删。
规则：四类硬禁令（破折号、箭头、emoji、非法全角）；中文正文标点白名单放行。
源自《中英文 Markdown 技术文档写作规范》v1.0（2026-09-02 转档，G005）。
"""
import re
import sys
from pathlib import Path

RULES = [
    ("DASH", re.compile("[–—―−ー]")),
    ("ARROW", re.compile("[←-⇿➔➜➡⬅-⬇]")),
    ("EMOJI", re.compile(
        "[\U0001F000-\U0001FAFF☀-➿⬀-⬏"
        "‼⁉ℹ︀-️‍]")),
    ("FULLWIDTH", re.compile("[！-｠　‘-”]")),
]
CJK_OK = set("，。：；？！、（）《》「」『』·")
INLINE_CODE = re.compile(r"`[^`]*`")
LINK_TARGET = re.compile(r"\]\([^)]*\)")
BARE_URL = re.compile(r"https?://\S+")
SKIP_DIRS = ("docs/diary/", "docs/proven/", "docs/web/share-src/node_modules/", "target/", "docs/aidoc/")
ALLOW_FILE = Path(__file__).parent / "md-char-allow.txt"


def load_allow():
    if not ALLOW_FILE.is_file():
        return set()
    out = set()
    for line in ALLOW_FILE.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            out.add(line.replace("\\", "/"))
    return out


def mask(line):
    line = INLINE_CODE.sub(lambda m: "`" + " " * (len(m.group()) - 2) + "`", line)
    line = LINK_TARGET.sub("]( )", line)
    return BARE_URL.sub(" ", line)


def scan(text):
    in_fence = False
    for no, raw in enumerate(text.splitlines(), 1):
        if raw.lstrip().startswith(("```", "$$")):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        for col, ch in enumerate(mask(raw), 1):
            for name, pattern in RULES:
                if pattern.match(ch) and not (name == "FULLWIDTH" and ch in CJK_OK):
                    yield no, col, name, ch
                    break


def iter_targets(args):
    if args:
        for a in args:
            p = Path(a)
            if p.is_dir():
                yield from sorted(p.rglob("*.md"))
            else:
                yield p
        return
    root = Path(__file__).parent.parent
    yield from sorted(root.glob("*.md"))
    yield from sorted((root / "docs").rglob("*.md"))


def main():
    allow = load_allow()
    hits = 0
    for path in iter_targets(sys.argv[1:]):
        try:
            rel = path.relative_to(Path.cwd()).as_posix() if path.is_absolute() else path.as_posix()
        except ValueError:
            rel = path.as_posix()
        if any(rel.startswith(s) for s in SKIP_DIRS):
            continue
        if rel in allow:
            continue
        for no, col, name, ch in scan(path.read_text(encoding="utf-8")):
            print(f"{rel}:{no}:{col}: {name} U+{ord(ch):04X} {ch!r}")
            hits += 1
    tail = f"（封闭清单豁免 {len(allow)} 文件）" if allow else ""
    print(f"违规 {hits} 处{tail}" if hits else f"通过：未发现违规字符{tail}")
    return 1 if hits else 0


if __name__ == "__main__":
    sys.exit(main())
