#!/usr/bin/env python3
"""Lista textos traduzíveis do vega-gtk ausentes dos catálogos po/.

Fontes: gettext("…") no Rust e t("…") / N_("…") na interface web. Saída em
JSON (lista de msgids) para revisão e tradução manual dos três catálogos.
"""
import ast
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
GTK = ROOT / "vega-gtk"
STRING = r'"((?:[^"\\]|\\.)*)"'
PATTERNS = {
    ".rs": re.compile(r"gettext\(\s*" + STRING),
    ".js": re.compile(r"\b(?:t|N_)\(\s*" + STRING),
}


def catalog(path):
    keys = set()
    lines = path.read_text(encoding="utf-8").splitlines()
    i = 0
    while i < len(lines):
        if lines[i].startswith("msgid "):
            key = ast.literal_eval(lines[i][6:])
            i += 1
            while i < len(lines) and lines[i].startswith('"'):
                key += ast.literal_eval(lines[i])
                i += 1
            keys.add(key)
        else:
            i += 1
    return keys


def used():
    found = {}
    for path in sorted(list((GTK / "src").rglob("*.rs")) + list((GTK / "ui").rglob("*.js"))):
        if "tests" in path.name:
            continue
        for match in PATTERNS[path.suffix].finditer(path.read_text(encoding="utf-8")):
            text = json.loads('"' + match.group(1) + '"')
            found.setdefault(text, str(path.relative_to(ROOT)))
    return found


if __name__ == "__main__":
    known = catalog(GTK / "po" / "pt-BR.po")
    missing = {k: v for k, v in used().items() if k not in known}
    json.dump(missing, sys.stdout, ensure_ascii=False, indent=1)
    print(f"\n{len(missing)} ausentes", file=sys.stderr)
