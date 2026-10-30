"""Texto do "Pedir ajuda" (vega-gtk/ui/js/pages/help-text.js), testado com node."""
import json
import pathlib
import shutil
import subprocess
import unittest

MODULE = pathlib.Path(__file__).resolve().parents[1] / "vega-gtk" / "ui" / "js" / "pages" / "help-text.js"

RUNNER = """
import { buildHelpText, HELP_LIMIT } from %s;
const input = JSON.parse(process.argv[1]);
const opts = {
  description: input.description,
  vega: "5.8.0",
  t: (text) => text,
  stageName: (id) => id,
  verdictLabel: (verdict) => verdict,
};
const text = buildHelpText(input.report, opts, input.limit ?? HELP_LIMIT);
process.stdout.write(JSON.stringify({ text, limit: HELP_LIMIT }));
"""


def build(report, description="A internet cai toda hora", limit=None):
    script = RUNNER % json.dumps(MODULE.as_uri())
    args = {"report": report, "description": description}
    if limit:
        args["limit"] = limit
    result = subprocess.run(
        ["node", "--input-type=module", "-e", script, json.dumps(args)],
        capture_output=True, text=True, check=True,
    )
    return json.loads(result.stdout)


def report(raw_size=200, findings=3):
    return {
        "started": "2026-10-30T10:00:00-03:00",
        "host": {"product": "Nitro AN515-57", "os": "Lyra OS 16.1", "kernel": "6.12.0", "vegad": "5.6.0", "hostname": "[redacted]"},
        "stages": [
            {"id": "logs", "verdict": "attention", "summary": "3 erros", "findings": [
                {"severity": "attention", "code": f"logs.item{i}", "text": f"Achado {i}", "raw": "x" * raw_size}
                for i in range(findings)
            ] + [{"severity": "info", "code": "logs.noise.boot", "text": "Ruído", "raw": "ruído"}]},
            {"id": "platform", "verdict": "ok", "summary": "tudo certo", "findings": []},
        ],
    }


@unittest.skipUnless(shutil.which("node"), "node ausente")
class HelpTextTest(unittest.TestCase):
    def test_small_report_keeps_everything_but_info(self):
        out = build(report())
        text = out["text"]
        self.assertIn("A internet cai toda hora", text)
        self.assertIn("- Computador: Nitro AN515-57", text)
        self.assertIn("- Vega: 5.8.0", text)
        self.assertIn("- logs: attention — 3 erros", text)
        self.assertIn("- platform: ok — tudo certo", text)
        self.assertEqual(text.count("`logs.item"), 3)
        self.assertNotIn("Ruído", text, "informação não entra")
        self.assertNotIn("…", text)

    def test_big_raw_data_is_cut_but_every_finding_stays(self):
        # 40 achados com 1.000 caracteres de dados brutos: 40.000 no total.
        out = build(report(raw_size=1000, findings=40))
        text = out["text"]
        self.assertLessEqual(len(text), out["limit"])
        self.assertEqual(text.count("`logs.item"), 40)
        self.assertIn("…", text)
        self.assertIn("- platform: ok", text)

    def test_redacted_values_pass_through_untouched(self):
        text = build(report())["text"]
        self.assertNotIn("hostname", text)

    def test_empty_description_is_marked(self):
        text = build(report(), description="  ")["text"]
        self.assertIn("(sem descrição)", text)


if __name__ == "__main__":
    unittest.main()
