#!/usr/bin/env python3
"""Checa a interface web do vega-gtk sem navegador.

- sintaxe de cada módulo (node --check);
- todo `import { x } from "./y.js"` aponta para um arquivo e um export que existem;
- todo arquivo CSS e script citado no index.html existe;
- todas as páginas do vega-gtk continuam registradas (ids usados em start_page).
"""
import pathlib
import re
import subprocess
import sys

UI = pathlib.Path(__file__).resolve().parents[2] / "vega-gtk" / "ui"
PAGE_IDS = {
    "dashboard", "software", "backup", "virtualization", "assistant", "hardware", "datetime",
    "screen", "monitor", "storage", "network", "desktop", "services", "users", "logs",
}

errors = []
modules = sorted(UI.glob("js/**/*.js"))
exports = {}
for path in modules:
    text = path.read_text(encoding="utf-8")
    names = set(re.findall(r"export (?:async )?(?:function\*?|const|let|class) (\w+)", text))
    for group in re.findall(r"export \{([^}]*)\}", text):
        names |= {item.split(" as ")[-1].strip() for item in group.split(",") if item.strip()}
    if "export default" in text:
        names.add("default")
    exports[path.resolve()] = names
    if path.name != "boot.js":
        result = subprocess.run(["node", "--input-type=module", "--check"], input=text, text=True, capture_output=True)
        if result.returncode:
            errors.append(f"{path.relative_to(UI)}: {result.stderr.strip().splitlines()[0]}")

for path in modules:
    text = path.read_text(encoding="utf-8")
    for names, source in re.findall(r'import \{([^}]*)\} from "([^"]+)"', text):
        target = (path.parent / source).resolve()
        if target not in exports:
            errors.append(f"{path.relative_to(UI)}: arquivo inexistente {source}")
            continue
        for name in (n.split(" as ")[0].strip() for n in names.split(",") if n.strip()):
            if name not in exports[target]:
                errors.append(f"{path.relative_to(UI)}: {source} não exporta {name}")
    for source in re.findall(r'import \w+ from "([^"]+)"', text):
        target = (path.parent / source).resolve()
        if "default" not in exports.get(target, set()):
            errors.append(f"{path.relative_to(UI)}: {source} sem export default")

index = (UI / "index.html").read_text(encoding="utf-8")
for asset in re.findall(r'(?:href|src)="([^"]+\.(?:css|js))"', index):
    if not (UI / asset).is_file():
        errors.append(f"index.html: {asset} não existe")

registered = set()
for path in UI.glob("js/pages/*.js"):
    registered.update(re.findall(r'^\s*id: "([a-z]+)",', path.read_text(encoding="utf-8"), re.M))
missing = PAGE_IDS - registered
if missing:
    errors.append(f"páginas sem registro: {sorted(missing)}")

if errors:
    print("\n".join(errors))
    sys.exit(1)
print(f"{len(modules)} módulos ok")
