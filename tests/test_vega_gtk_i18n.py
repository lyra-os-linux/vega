import ast
import pathlib
import re
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
PO_DIR = ROOT / "vega-gtk" / "po"
LOCALES = ("en-US", "pt-BR", "es-ES", "zh-CN")
PLACEHOLDER = re.compile(r"\{[A-Za-z_][A-Za-z0-9_]*\}")


def messages(path):
    result = {}
    lines = path.read_text(encoding="utf-8").splitlines()
    index = 0
    while index < len(lines):
        if not lines[index].startswith("msgid "):
            index += 1
            continue
        key = ast.literal_eval(lines[index][6:])
        index += 1
        while index < len(lines) and lines[index].startswith('"'):
            key += ast.literal_eval(lines[index])
            index += 1
        while index < len(lines) and not lines[index].startswith("msgstr "):
            index += 1
        if index >= len(lines):
            break
        value = ast.literal_eval(lines[index][7:])
        index += 1
        while index < len(lines) and lines[index].startswith('"'):
            value += ast.literal_eval(lines[index])
            index += 1
        if key:
            result[key] = value
    return result


class VegaGtkI18nTests(unittest.TestCase):
    def test_catalogs_have_identical_complete_keys_and_placeholders(self):
        catalogs = {locale: messages(PO_DIR / f"{locale}.po") for locale in LOCALES}
        expected = set(catalogs["en-US"])
        self.assertGreaterEqual(len(expected), 729)
        for locale, catalog in catalogs.items():
            self.assertEqual(set(catalog), expected, locale)
            self.assertTrue(all(catalog.values()), locale)
            for key, value in catalog.items():
                self.assertEqual(
                    sorted(PLACEHOLDER.findall(value)),
                    sorted(PLACEHOLDER.findall(key)),
                    f"{locale}: {key}",
                )

    def test_technical_names_are_preserved(self):
        protected = ("vegad", "systemd", "Flatpak", "NetworkManager", "PKGBUILD", "SIGTERM")
        for locale in LOCALES:
            catalog = messages(PO_DIR / f"{locale}.po")
            for key, value in catalog.items():
                for token in protected:
                    if token in key:
                        self.assertIn(token, value, f"{locale}: {key}")

    def test_rpm_specs_install_all_catalogs_and_fallback_domains(self):
        for relative in ("packaging/opensuse/vega.spec", "packaging/obs/vega-gtk.spec"):
            spec = (ROOT / relative).read_text(encoding="utf-8")
            for locale in ("en_US", "pt_BR", "es_ES", "zh_CN"):
                self.assertIn(locale, spec)
            self.assertIn("vega-gtk-fallback.mo", spec)


if __name__ == "__main__":
    unittest.main()
