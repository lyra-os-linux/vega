#!/usr/bin/env python3
"""Baixa os logos dos aplicativos populares (Vega → Software → Populares e Desenvolvedores).

Lê os ids `logo:` de popular-catalog.js e developer-catalog.js, consulta
a API de appstream do Flathub e grava o ícone de 128 px em
vega-gtk/ui/img/popular/<id>.png. Os arquivos ficam no repositório: o Vega não
acessa a rede para mostrar a vitrine.

Uso: scripts/update-popular-logos.py [--force]
"""
import json
import pathlib
import re
import subprocess
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
CATALOGS = [ROOT / "vega-gtk/ui/js/pages/software" / f"{name}-catalog.js" for name in ("popular", "developer", "network")]
OUT = ROOT / "vega-gtk/ui/img/popular"
API = "https://flathub.org/api/v2/appstream/{}"
LIMIT = 64 * 1024
# Apps fora do Flathub: logo baixado direto do projeto (SVG vira PNG de
# 128 px com o rsvg-convert).
DIRECT = {
    "postgres-draco": "https://dracodb.com.br/assets/draco-logo.png",
    "linuxtoys": "https://raw.githubusercontent.com/psygreg/linuxtoys/7.3.4/src/linuxtoys.svg",
}


def rasterize(svg):
    return subprocess.run(
        ["rsvg-convert", "--width=128", "--height=128", "--keep-aspect-ratio", "--format=png"],
        input=svg, capture_output=True, check=True,
    ).stdout


def fetch(url):
    request = urllib.request.Request(url, headers={"User-Agent": "lyra-vega-popular-logos"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read()


def icon_url(app_id):
    data = json.loads(fetch(API.format(app_id)))
    icons = [i for i in data.get("icons") or [] if not i.get("scale") and i.get("width") == 128]
    return icons[0]["url"] if icons else data.get("icon")


def main():
    force = "--force" in sys.argv[1:]
    ids = sorted(set(re.findall(r'logo:\s*"([^"]+)"', "".join(c.read_text(encoding="utf-8") for c in CATALOGS))))
    OUT.mkdir(parents=True, exist_ok=True)
    failed = []
    for app_id in ids:
        target = OUT / f"{app_id}.png"
        if target.exists() and not force:
            continue
        try:
            url = DIRECT.get(app_id) or icon_url(app_id)
            if not url:
                raise RuntimeError("sem ícone no appstream")
            data = fetch(url)
            if url.endswith(".svg"):
                data = rasterize(data)
            if not data.startswith(b"\x89PNG") or len(data) > LIMIT:
                raise RuntimeError(f"ícone inválido ({len(data)} bytes)")
            target.write_bytes(data)
            print(f"ok   {app_id}")
        except Exception as error:  # noqa: BLE001 - relatório por app
            failed.append(app_id)
            print(f"erro {app_id}: {error}", file=sys.stderr)
    stale = {p.stem for p in OUT.glob("*.png")} - set(ids)
    for name in sorted(stale):
        (OUT / f"{name}.png").unlink()
        print(f"rm   {name}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
