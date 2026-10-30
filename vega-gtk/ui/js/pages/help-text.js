// Texto do "Pedir ajuda" (add-help-report), montado a partir do relatório
// **redigido** do diagnóstico. Sem dependências da interface, para ser
// testado com node (tests/test_help_text.py).

/** Limite do texto: a URL de issue e de discussão fica abaixo de ~8 KB. */
export const HELP_LIMIT = 6000;

const ELLIPSIS = "…";

/** Corta `text` em `max` caracteres, terminando com "…" quando cortou. */
export function cut(text, max) {
  if (text.length <= max) return text;
  if (max <= 1) return ELLIPSIS;
  return text.slice(0, max - 1).trimEnd() + ELLIPSIS;
}

/**
 * Monta o Markdown do pedido de ajuda. `opts`:
 * - `description`: o que o usuário escreveu;
 * - `vega`: versão do Vega;
 * - `t(text, vars)`: tradução; `stageName(id)` e `verdictLabel(verdict)`.
 * Só entram os achados de atenção e falha. Se passar de `limit`, os dados
 * brutos são encurtados por igual; nenhum achado sai do texto.
 */
export function buildHelpText(report, opts, limit = HELP_LIMIT) {
  const { t, stageName, verdictLabel } = opts;
  const host = report.host ?? {};
  const head = [];
  head.push(`## ${t("O que está acontecendo")}`, "", (opts.description ?? "").trim() || t("(sem descrição)"), "");
  head.push(`## ${t("Sistema")}`, "");
  const facts = [
    [t("Computador"), host.product],
    [t("Sistema"), host.os],
    ["Kernel", host.kernel],
    ["vegad", host.vegad],
    ["Vega", opts.vega],
    [t("Diagnóstico"), report.started],
  ];
  for (const [label, value] of facts) if (value) head.push(`- ${label}: ${value}`);
  head.push("", `## ${t("Testes")}`, "");
  for (const stage of report.stages ?? []) {
    const summary = stage.summary ? ` — ${stage.summary}` : "";
    head.push(`- ${stageName(stage.id)}: ${verdictLabel(stage.verdict)}${summary}`);
  }

  const findings = [];
  for (const stage of report.stages ?? []) {
    for (const finding of stage.findings ?? []) {
      if (finding.severity !== "attention" && finding.severity !== "fail") continue;
      findings.push({
        title: `- **${stageName(stage.id)}:** ${finding.text} (\`${finding.code}\`)`,
        raw: (finding.raw ?? "").trim(),
      });
    }
  }

  const compose = (rawMax) => {
    const lines = [...head];
    if (findings.length) {
      lines.push("", `## ${t("Achados")}`, "");
      for (const finding of findings) {
        lines.push(finding.title);
        if (finding.raw && rawMax > 0) lines.push("", "```", cut(finding.raw, rawMax), "```", "");
      }
    }
    return lines.join("\n").trimEnd() + "\n";
  };

  let text = compose(Infinity);
  if (text.length <= limit) return text;
  // Encurta os dados brutos por igual até caber; sem espaço, tira só eles.
  const withoutRaw = compose(0).length;
  const withRaw = findings.filter((finding) => finding.raw).length;
  // Cada trecho custa a cerca do bloco de código (~10 caracteres).
  let share = withRaw ? Math.floor((limit - withoutRaw) / withRaw) - 12 : 0;
  while (share > 0) {
    text = compose(share);
    if (text.length <= limit) return text;
    share -= Math.max(1, Math.ceil((text.length - limit) / withRaw));
  }
  return cut(compose(0), limit);
}
