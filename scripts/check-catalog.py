#!/usr/bin/env python3
"""Confere os catálogos do Software (vega-gtk/ui/js/pages/software/*-catalog.js).

Sem opções, só a validação offline, usada na CI de PR:
- todo app tem name, origin, id e text, e a origem é conhecida;
- ids únicos por seção e seções únicas por catálogo;
- id do Flathub no formato de id de app (com.empresa.App);
- apps do Windows e AppImages existem em windows_apps.rs e appimage_apps.rs;
- `tasks` (seção ou cartão) entre as de TASKS (catalog-search.js) e `tags`
  como lista de textos;
- textos e tags com tradução em en-US e es-ES (po/*.po).

Com --remote, também confere na rede (workflow diário catalog.yml):
- `origin: "flathub"`: o app existe em flathub.org/api/v2/appstream/<id>;
- `origin: "official"`: o pacote existe nos metadados dos repositórios da
  imagem (Leap 16.1 oss e non-oss, Packman Essentials, lyra, vega e fina) ou,
  com `repo.url`, no repositório do fabricante; com `repo.package`, confere o
  pacote do Lyra que registra o repositório;
- `origin: "npm"`: o pacote existe no registro do npm;
- receitas do Windows (RECIPES): a URL responde e o SHA-256 bate com o fixado;
- AppImages: a última release tem o AppImage e o arquivo de checksums;
  nos fixados, a tag tem o arquivo e o SHA-256 confere com o do GitHub.
Os verbos do winetricks não entram: o próprio winetricks confere o checksum.

Saída: 0 tudo certo, 1 itens quebrados, 2 verificação incompleta (falha de
rede ou serviço fora do ar, sem concluir se o item quebrou).
"""
import argparse
import ast
import concurrent.futures
import gzip
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
GTK = ROOT / "vega-gtk"
CATALOGS = sorted((GTK / "ui" / "js" / "pages" / "software").glob("*-catalog.js"))
ORIGINS = {"official", "flathub", "windows", "npm", "appimage"}
FLATHUB_ID = re.compile(r"^[A-Za-z_][\w-]*(\.[A-Za-z_][\w-]*){2,}$")
REPOS = [
    "https://download.opensuse.org/distribution/leap/16.1/repo/oss/",
    "https://download.opensuse.org/distribution/leap/16.1/repo/non-oss/",
    "https://ftp.gwdg.de/pub/linux/misc/packman/suse/openSUSE_Leap_16.1/Essentials/",
    "https://download.opensuse.org/repositories/home:/rodrigosbrito:/lyra/openSUSE_Leap_16.1/",
    "https://download.opensuse.org/repositories/home:/rodrigosbrito:/vega/openSUSE_Leap_16.1/",
    "https://download.opensuse.org/repositories/home:/rodrigosbrito:/fina/openSUSE_Leap_16.1/",
]
USER_AGENT = "lyra-vega-check-catalog (+https://github.com/lyra-os-linux/vega)"

LOADER = """
import { pathToFileURL } from "node:url";
const out = {};
for (const file of process.argv.slice(1)) {
  const module = await import(pathToFileURL(file));
  for (const [name, value] of Object.entries(module)) out[`${file.split("/").pop()}:${name}`] = value;
}
console.log(JSON.stringify(out));
"""


class Unverified(Exception):
    """A verificação não terminou (rede, servidor fora do ar)."""


def load_catalogs():
    """Lê os catálogos pelo node e devolve [(catálogo, seção, app)]."""
    result = subprocess.run(
        ["node", "--input-type=module", "-e", LOADER, *map(str, CATALOGS)],
        text=True, capture_output=True, check=True,
    )
    entries, errors = [], []
    for export, items in json.loads(result.stdout).items():
        sections, seen_sections = [], set()
        loose = [item for item in items if "apps" not in item]
        if loose:
            sections.append({"id": "", "apps": loose})
        for section in (item for item in items if "apps" in item):
            if section.get("id") in seen_sections:
                errors.append(f"{export}: seção repetida {section.get('id')!r}")
            seen_sections.add(section.get("id"))
            if not section.get("title"):
                errors.append(f"{export}: seção {section.get('id')!r} sem title")
            sections.append(section)
        for section in sections:
            seen = set()
            for app in section["apps"]:
                where = f"{export}/{section['id'] or '-'}"
                key = (app.get("origin"), app.get("id"))
                if key in seen:
                    errors.append(f"{where}: {app.get('name')} repetido ({key[0]}:{key[1]})")
                seen.add(key)
                entries.append((where, section, app))
    return entries, errors


def rust_ids(path, pattern):
    return set(re.findall(pattern, path.read_text(encoding="utf-8")))


def recipes():
    """Receitas do Windows: {id: (url, sha256)}."""
    text = (GTK / "src" / "backend" / "windows_apps.rs").read_text(encoding="utf-8")
    found = re.findall(r'id: "([^"]+)",\s*url: "([^"]+)",\s*sha256: "([0-9a-f]{64})"', text)
    return {rid: (url, sha) for rid, url, sha in found}


def verbs():
    text = (GTK / "src" / "backend" / "windows_apps.rs").read_text(encoding="utf-8")
    block = re.search(r"pub const VERBS: &\[&str\] = &\[(.*?)\];", text, re.S).group(1)
    return set(re.findall(r'"([^"]+)"', block))


def appimages():
    """AppImages: {id: (dono/repositório, prefixo, arquivo de checksums)}."""
    text = (GTK / "src" / "backend" / "appimage_apps.rs").read_text(encoding="utf-8")
    found = re.findall(r'id: "([^"]+)",\s*name: "[^"]*",\s*repo: "([^"]+)",\s*source: Source::Latest \{\s*prefix: "([^"]+)",\s*sums: "([^"]+)"', text, re.S)
    apps = {aid: (repo, prefix, sums) for aid, repo, prefix, sums in found}
    # Versão fixada: (dono/repositório, tag, arquivo, SHA-256).
    pinned = re.findall(r'id: "([^"]+)",\s*name: "[^"]*",\s*repo: "([^"]+)",\s*source: Source::Pinned \{\s*tag: "([^"]+)",\s*file: "([^"]+)",\s*sha256: "([^"]+)"', text, re.S)
    apps.update({aid: (repo, tag, file, sha) for aid, repo, tag, file, sha in pinned})
    return apps


def po_translations(path):
    """msgid → msgstr de um catálogo po."""
    entries, current, field = {}, {}, None
    for line in path.read_text(encoding="utf-8").splitlines() + [""]:
        match = re.match(r"(msgid|msgstr) (\".*\")$", line)
        if match:
            field = match.group(1)
            current[field] = ast.literal_eval(match.group(2))
        elif line.startswith('"') and field:
            current[field] += ast.literal_eval(line)
        else:
            if "msgid" in current:
                entries[current["msgid"]] = current.get("msgstr", "")
            current, field = {}, None
    return entries


def task_ids():
    text = (GTK / "ui" / "js" / "pages" / "software" / "catalog-search.js").read_text(encoding="utf-8")
    return set(re.findall(r'\{ id: "([a-z]+)", label: N_\(', text))


def check_offline(entries):
    errors = []
    tasks = task_ids()
    windows_ids = verbs() | set(recipes())
    appimage_ids = set(appimages())
    translations = {lang: po_translations(GTK / "po" / f"{lang}.po") for lang in ("en-US", "es-ES")}
    texts = set()
    for where, section, app in entries:
        label = f"{where}: {app.get('name') or app.get('id')}"
        for field in ("name", "origin", "id", "text"):
            if not app.get(field):
                errors.append(f"{label}: sem {field}")
        origin, app_id = app.get("origin"), app.get("id", "")
        if origin not in ORIGINS:
            errors.append(f"{label}: origem desconhecida {origin!r}")
        if origin == "flathub" and not FLATHUB_ID.match(app_id):
            errors.append(f"{label}: id do Flathub inválido {app_id!r}")
        if origin == "windows" and app_id not in windows_ids:
            errors.append(f"{label}: {app_id!r} não está em VERBS nem em RECIPES (windows_apps.rs)")
        if origin == "appimage" and app_id not in appimage_ids:
            errors.append(f"{label}: {app_id!r} não está em APPS (appimage_apps.rs)")
        repo = app.get("repo")
        if repo is not None and not repo.get("name"):
            errors.append(f"{label}: repo sem name")
        elif repo is not None and "package" in repo:
            if not re.fullmatch(r"[0-9A-F]{40}", repo.get("fingerprint", "")):
                errors.append(f"{label}: repo.fingerprint precisa de 40 dígitos hexadecimais")
        elif repo is not None and not str(repo.get("url", "")).startswith("https://"):
            errors.append(f"{label}: repo precisa de url https ou de package")
        for task in set(app.get("tasks", [])) | set(section.get("tasks", [])):
            if task not in tasks:
                errors.append(f"{label}: tarefa desconhecida {task!r} (TASKS em catalog-search.js)")
        tags = app.get("tags", [])
        if not isinstance(tags, list) or not all(isinstance(tag, str) and tag for tag in tags):
            errors.append(f"{label}: tags precisa ser uma lista de textos")
            tags = []
        texts.update(filter(None, (app.get("text"), section.get("title"), *tags)))
    for lang, known in translations.items():
        for text in sorted(texts):
            if not known.get(text):
                errors.append(f"po/{lang}.po: sem tradução para {text!r}")
    return errors


def fetch(url, accept_404=False):
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    token = os.environ.get("GITHUB_TOKEN")
    if token and url.startswith("https://api.github.com/"):
        request.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(request, timeout=120) as response:
            return response.read()
    except urllib.error.HTTPError as error:
        if accept_404 and error.code in (404, 410):
            return None
        raise Unverified(f"{url}: HTTP {error.code}") from error
    except (urllib.error.URLError, TimeoutError, OSError) as error:
        raise Unverified(f"{url}: {error}") from error


def repo_packages(base):
    """Nomes dos pacotes de um repositório rpm-md, pelos metadados."""
    base = base if base.endswith("/") else base + "/"
    repomd = fetch(base + "repodata/repomd.xml").decode()
    href = re.search(r'<data type="primary">.*?<location href="([^"]+)"', repomd, re.S)
    if not href:
        raise Unverified(f"{base}: repomd.xml sem primary")
    data = fetch(base + href.group(1))
    if href.group(1).endswith(".zst"):
        data = subprocess.run(["zstd", "-dc"], input=data, capture_output=True, check=True).stdout
    elif href.group(1).endswith(".gz"):
        data = gzip.decompress(data)
    return set(re.findall(rb"<name>([^<]+)</name>", data))


def check_flathub(app_id):
    body = fetch(f"https://flathub.org/api/v2/appstream/{urllib.parse.quote(app_id)}", accept_404=True)
    if body is None or json.loads(body or b"null") is None:
        return "não existe no Flathub"
    return None


def check_npm(name):
    if fetch(f"https://registry.npmjs.org/{urllib.parse.quote(name, safe='@')}", accept_404=True) is None:
        return "não existe no registro do npm"
    return None


def check_recipe(url, sha256):
    body = fetch(url, accept_404=True)
    if body is None:
        return f"o download sumiu (HTTP 404): {url}"
    if body.lstrip(b"\xef\xbb\xbf \r\n\t")[:1] == b"<":
        # Alguns fabricantes respondem 200 com a página de download quando a
        # versão fixada sai do ar (o Mp3tag faz isso).
        return f"o download devolve uma página HTML, não o instalador (versão retirada?): {url}"
    actual = hashlib.sha256(body).hexdigest()
    if actual != sha256:
        return f"SHA-256 mudou: fixado {sha256}, baixado {actual} ({url})"
    return None


def check_appimage(repo, prefix, sums):
    release = json.loads(fetch(f"https://api.github.com/repos/{repo}/releases/latest"))
    names = {asset["name"] for asset in release.get("assets", [])}
    if sums not in names:
        return f"a release {release.get('tag_name')} de {repo} não tem {sums}"
    if not any(name.startswith(prefix) and name.endswith(".AppImage") for name in names):
        return f"a release {release.get('tag_name')} de {repo} não tem {prefix}*.AppImage"
    return None


def check_pinned_appimage(repo, tag, file, sha256):
    release = json.loads(fetch(f"https://api.github.com/repos/{repo}/releases/tags/{tag}"))
    asset = next((a for a in release.get("assets", []) if a["name"] == file), None)
    if asset is None:
        return f"a release {tag} de {repo} não tem {file}"
    if asset.get("digest") and asset["digest"] != f"sha256:{sha256}":
        return f"{file}: o SHA-256 fixado não confere com o publicado no GitHub"
    return None


def check_remote(entries):
    """Devolve (quebrados, não verificados)."""
    broken, unverified, jobs = [], [], []
    packages, vendor = set(), {}
    for base in REPOS:
        try:
            packages |= repo_packages(base)
        except Unverified as error:
            unverified.append(f"repositório {error}")
    official_ok = not unverified
    for where, _section, app in entries:
        label = f"{where}: {app['name']} ({app['origin']}:{app['id']})"
        origin = app["origin"]
        if origin == "flathub":
            jobs.append((label, check_flathub, (app["id"],)))
        elif origin == "npm":
            jobs.append((label, check_npm, (app["id"],)))
        elif origin == "official" and app.get("repo", {}).get("package"):
            # O pacote do Lyra registra o repositório do fabricante; o URL
            # fica no .repo, então aqui só o pacote do Lyra é conferido.
            package = app["repo"]["package"]
            if official_ok and package.encode() not in packages:
                broken.append(f"{label}: pacote do repositório {package} não existe nos repositórios do Lyra")
        elif origin == "official" and app.get("repo"):
            url = app["repo"]["url"]
            if url not in vendor:
                try:
                    vendor[url] = repo_packages(url)
                except Unverified as error:
                    vendor[url] = None
                    unverified.append(f"repositório {error}")
            if vendor[url] is not None and app["id"].encode() not in vendor[url]:
                broken.append(f"{label}: pacote não existe em {url}")
        elif origin == "official" and official_ok and app["id"].encode() not in packages:
            broken.append(f"{label}: pacote não existe nos repositórios do Leap 16.1 e do Lyra")
    for rid, (url, sha256) in sorted(recipes().items()):
        jobs.append((f"receita do Windows {rid}", check_recipe, (url, sha256)))
    for aid, spec in sorted(appimages().items()):
        check = check_appimage if len(spec) == 3 else check_pinned_appimage
        jobs.append((f"AppImage {aid}", check, spec))

    def run(job):
        label, function, args = job
        try:
            problem = function(*args)
            return (label, problem, None)
        except Unverified as error:
            return (label, None, str(error))

    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        for label, problem, error in pool.map(run, jobs):
            if problem:
                broken.append(f"{label}: {problem}")
            if error:
                unverified.append(f"{label}: {error}")
    return broken, unverified


def markdown(errors, broken, unverified):
    lines = []
    for title, items in (
        ("Erros nos catálogos", errors),
        ("Itens quebrados", broken),
        ("Não verificados (rede ou serviço fora do ar)", unverified),
    ):
        if items:
            lines += [f"### {title}", ""] + [f"- {item}" for item in items] + [""]
    return "\n".join(lines) or "Todos os itens do catálogo conferidos.\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--remote", action="store_true", help="também confere na rede")
    parser.add_argument("--markdown", type=pathlib.Path, help="grava o relatório em Markdown")
    args = parser.parse_args()

    entries, errors = load_catalogs()
    errors += check_offline(entries)
    broken, unverified = check_remote(entries) if args.remote and not errors else ([], [])
    report = markdown(errors, broken, unverified)
    if args.markdown:
        args.markdown.write_text(report, encoding="utf-8")
    print(report, end="")
    print(f"{len(entries)} apps em {len(CATALOGS)} catálogos", file=sys.stderr)
    if errors or broken:
        return 1
    return 2 if unverified else 0


if __name__ == "__main__":
    sys.exit(main())
