import { call } from "../api.js";
import { N_, t } from "../i18n.js";
import { h, icon, clear, empty, skeletonRows, normalize, toast } from "../ui.js";

// Mesmos valores enviados ao vegad pelo vega-gtk (ui/logs.rs).
const PRIORITIES = [["", N_("Todas as prioridades")], ["err", N_("Erro ou mais grave")], ["warning", N_("Aviso ou mais grave")], ["info", N_("Informação ou mais grave")], ["debug", N_("Tudo, incluindo debug")]];
const SINCE = [["-15min", N_("Últimos 15 min")], ["-1hour", N_("Última hora")], ["-24hour", N_("Últimas 24h")], ["-7day", N_("Últimos 7 dias")], ["", N_("Sem limite de período")]];
const LIMITS = [[100, N_("100 linhas")], [250, N_("250 linhas")], [500, N_("500 linhas")], [1000, N_("1.000 linhas")]];

function select(options, value, label) {
  const el = h("select", { class: "input", "aria-label": label }, options.map(([v, text]) => h("option", { value: String(v), selected: String(v) === String(value) }, t(text))));
  return el;
}

// Destaca a busca na linha sem montar HTML a partir do texto do journal.
function highlight(line, q) {
  if (!q) return line;
  const lower = line.toLowerCase();
  const parts = [];
  let index = 0;
  let at;
  while ((at = lower.indexOf(q, index)) !== -1) {
    parts.push(line.slice(index, at), h("mark", {}, line.slice(at, at + q.length)));
    index = at + q.length;
  }
  parts.push(line.slice(index));
  return parts;
}

// Sem prioridade estruturada nas linhas, só sinaliza as que citam erro/aviso.
function severity(line) {
  const n = normalize(line);
  if (/\b(error|erro|failed|falha|fatal|critical|panic)\b/.test(n)) return "err";
  if (/\b(warning|warn|aviso)\b/.test(n)) return "warn";
  return "";
}

export default {
  id: "logs",
  title: N_("Log do Sistema"),
  icon: "logs",
  group: "system",
  keywords: ["journal", "journalctl", "erros", "eventos", "registro", "log"],

  render(ctx) {
    const unit = h("select", { class: "input", "aria-label": t("Todas as unidades"), disabled: true }, h("option", { value: "" }, t("Todas as unidades")));
    const priority = select(PRIORITIES, "", t("Todas as prioridades"));
    const since = select(SINCE, "-1hour", t("Última hora"));
    const limit = select(LIMITS, 500, t("500 linhas"));
    const search = h("input", { class: "input", placeholder: t("Buscar texto no log…"), "aria-label": t("Buscar texto no log…") });
    const queryButton = h("button", { class: "btn primary" }, icon("search", 15), t("Buscar"));
    const status = h("span", { class: "small faint selectable" }, t("Carregando unidades do journal…"));
    const output = h("div", { class: "table-wrap" }, skeletonRows(6));
    let lines = [];
    let lastQuery = "";
    let running = false;

    const paint = () => {
      if (!lines.length) {
        clear(output, empty("logs", t("Nenhuma entrada encontrada para os filtros selecionados.")));
        return;
      }
      const q = lastQuery.toLowerCase();
      clear(output, h("div", { class: "log-view mono selectable", role: "log" },
        lines.map((line) => h("div", { class: `log-line ${severity(line)}`.trim() }, highlight(line, q)))));
    };

    async function query() {
      if (running) return;
      running = true;
      queryButton.disabled = true;
      clear(queryButton, h("span", { class: "spinner" }), t("Buscando…"));
      status.textContent = t("Consultando o journal…");
      lastQuery = search.value.trim();
      try {
        lines = await call("logs_query", { unit: unit.value, priority: priority.value, since: since.value, search: lastQuery, limit: Number(limit.value) });
        status.textContent = t("{count} linha(s)", { count: lines.length });
        paint();
      } catch (error) {
        status.textContent = error.message;
        clear(output, empty("alert-circle", t("Log do Sistema"), error.message));
      } finally {
        running = false;
        queryButton.disabled = false;
        clear(queryButton, icon("search", 15), t("Buscar"));
      }
    }

    // O journal administrativo pede polkit: nada é consultado antes de a
    // página ser aberta (o vega-gtk só carregava ao exibir o módulo).
    (async () => {
      try {
        const units = await call("logs_units");
        for (const name of units) unit.append(h("option", { value: name, selected: name === ctx.sub }, name));
        if (ctx.sub && !units.includes(ctx.sub)) unit.append(h("option", { value: ctx.sub, selected: true }, ctx.sub));
        unit.disabled = false;
        if (ctx.sub) ctx.setCrumb(ctx.sub);
      } catch (error) {
        status.textContent = error.message;
        clear(output, empty("alert-circle", t("Log do Sistema"), error.message));
        return;
      }
      await query();
    })();

    queryButton.addEventListener("click", query);
    search.addEventListener("keydown", (e) => { if (e.key === "Enter") query(); });
    for (const el of [unit, priority, since, limit]) el.addEventListener("change", query);

    const copy = h("button", { class: "icon-btn", "aria-label": t("Copiar"), title: t("Copiar"), onclick: () => {
      navigator.clipboard?.writeText(lines.join("\n"));
      toast(t("Copiado"));
    } }, icon("copy", 17));

    return {
      el: h("div", {},
        h("div", { class: "page-hero" }, h("div", {}, h("h2", {}, t("Log do Sistema")), h("p", {}, t("Consulta somente leitura do journal pelo vegad")))),
        h("div", { class: "toolbar logs-toolbar" },
          h("label", { class: "input-icon logs-search" }, icon("search", 15), search),
          queryButton),
        h("div", { class: "toolbar logs-filters" }, unit, priority, since, limit),
        h("div", { class: "logs-status" }, status),
        output),
      actions: [copy],
    };
  },
};
