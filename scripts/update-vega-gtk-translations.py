#!/usr/bin/env python3
"""Extract vega-gtk messages and refresh the three supported PO catalogs.

Translations fetched by this maintainer tool are drafts. Runtime/builds never
contact the network. Placeholders are protected and validated before writing.
"""

from __future__ import annotations

import ast
import json
import pathlib
import re
import subprocess
import sys
import time
import urllib.parse
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
GTK = ROOT / "vega-gtk"
POT = GTK / "po" / "vega-gtk.pot"
LANGUAGES = {"en-US": "en", "pt-BR": None, "es-ES": "es"}
PLACEHOLDER = re.compile(r"\{[A-Za-z_][A-Za-z0-9_]*\}")


def quoted(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def parse_messages(pot: str) -> list[tuple[list[str], str]]:
    messages: list[tuple[list[str], str]] = []
    comments: list[str] = []
    lines = pot.splitlines()
    index = 0
    while index < len(lines):
        line = lines[index]
        if line.startswith("#"):
            comments.append(line)
            index += 1
            continue
        if not line.startswith("msgid "):
            comments = []
            index += 1
            continue
        value = ast.literal_eval(line[6:])
        index += 1
        while index < len(lines) and lines[index].startswith('"'):
            value += ast.literal_eval(lines[index])
            index += 1
        if value:
            messages.append((comments, value))
        comments = []
    return messages


def protect(value: str) -> tuple[str, list[str]]:
    placeholders: list[str] = []

    def replace(match: re.Match[str]) -> str:
        placeholders.append(match.group(0))
        return f"\ue100{len(placeholders) - 1}\ue101"

    return PLACEHOLDER.sub(replace, value), placeholders


def request_translation(value: str, target: str) -> str:
    query = urllib.parse.urlencode(
        {"client": "gtx", "sl": "pt", "tl": target, "dt": "t", "q": value}
    )
    request = urllib.request.Request(
        "https://translate.googleapis.com/translate_a/single?" + query,
        headers={"User-Agent": "Vega translation maintainer/1"},
    )
    for attempt in range(5):
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                data = json.load(response)
            return "".join(part[0] for part in data[0])
        except Exception:
            if attempt == 4:
                raise
            time.sleep(1 + attempt)
    raise AssertionError("unreachable")


def translate_batch(values: list[str], target: str) -> list[str]:
    separator = "\ue000"
    protected: list[str] = []
    placeholders_by_value: list[list[str]] = []
    for value in values:
        text, placeholders = protect(value)
        protected.append(text)
        placeholders_by_value.append(placeholders)
    result = request_translation(("\n" + separator + "\n").join(protected), target)
    parts = re.split(r"\s*\ue000\s*", result)
    if len(parts) != len(values):
        raise ValueError(f"batch separator mismatch: wanted {len(values)}, got {len(parts)}")
    for index, placeholders in enumerate(placeholders_by_value):
        leading = re.match(r"^\s*", values[index]).group(0)
        trailing = re.search(r"\s*$", values[index]).group(0)
        parts[index] = leading + parts[index].strip() + trailing
        for number, placeholder in enumerate(placeholders):
            token = f"\ue100{number}\ue101"
            parts[index] = parts[index].replace(token, placeholder)
        if sorted(PLACEHOLDER.findall(parts[index])) != sorted(placeholders):
            raise ValueError(f"placeholder mismatch in {values[index]!r}: {parts[index]!r}")
    return parts


def header(locale: str) -> str:
    plural = "nplurals=2; plural=(n != 1);"
    return "\n".join(
        [
            "# Vega GTK translation catalog.",
            "# Machine-assisted draft: native-speaker review required.",
            "#",
            'msgid ""',
            'msgstr ""',
            '"Project-Id-Version: vega-gtk 4.0.3\\n"',
            '"Report-Msgid-Bugs-To: https://github.com/lyra-os-linux/vega/issues\\n"',
            f'"Language: {locale}\\n"',
            '"PO-Revision-Date: 2026-08-12 00:00+0000\\n"',
            '"Last-Translator: Vega contributors\\n"',
            '"Language-Team: Lyra OS\\n"',
            '"MIME-Version: 1.0\\n"',
            '"Content-Type: text/plain; charset=UTF-8\\n"',
            '"Content-Transfer-Encoding: 8bit\\n"',
            f'"Plural-Forms: {plural}\\n"',
            "",
        ]
    )


def main() -> int:
    subprocess.run(
        [
            "xtr",
            "vega-gtk/src/main.rs",
            "-o",
            str(POT),
            "--package-name",
            "vega-gtk",
            "--package-version",
            "4.0.3",
            "--msgid-bugs-address",
            "https://github.com/lyra-os-linux/vega/issues",
        ],
        check=True,
        cwd=ROOT,
    )
    messages = parse_messages(POT.read_text(encoding="utf-8"))
    selected = set(sys.argv[1:]) or set(LANGUAGES)
    unknown = selected.difference(LANGUAGES)
    if unknown:
        raise SystemExit(f"unsupported catalog(s): {', '.join(sorted(unknown))}")
    for locale, target in LANGUAGES.items():
        if locale not in selected:
            continue
        output = [header(locale)]
        translated_messages: list[str] = []
        if target is None:
            translated_messages = [message for _, message in messages]
        else:
            for start in range(0, len(messages), 20):
                batch = [message for _, message in messages[start : start + 20]]
                translated_messages.extend(translate_batch(batch, target))
                print(f"{locale}: {min(start + 20, len(messages))}/{len(messages)}", flush=True)
        for (comments, message), translated in zip(messages, translated_messages, strict=True):
            output.extend(comments)
            output.append("msgid " + quoted(message))
            output.append("msgstr " + quoted(translated))
            output.append("")
        path = GTK / "po" / f"{locale}.po"
        path.write_text("\n".join(output), encoding="utf-8")
        subprocess.run(["msgfmt", "--check", "-o", "/dev/null", str(path)], check=True)
        print(f"wrote {path.relative_to(ROOT)} ({len(messages)} messages)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
