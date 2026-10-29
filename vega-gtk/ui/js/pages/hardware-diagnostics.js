// Diagnóstico de saúde (vega#165): os testes do vegad (interface
// org.lyraos.Vega1.Diagnostics) e o teste de GPU, que roda no próprio Vega.
//
// Abrir a página nunca pede senha: o último resultado de cada teste vem dos
// sinais StageFinished, guardados aqui. O relatório completo (que inclui
// trechos do journal e exige logs.read-admin) só é buscado quando o usuário
// pede para vê-lo ou exportá-lo.

import { call, on } from "../api.js";
import { N_, t, locale } from "../i18n.js";
import { hasCapability } from "../store.js";
import { h, icon, clear, button, badge, segmented, row, list, banner, meter, toast, dateTime, duration, confirm, sheet } from "../ui.js";

const TESTS = [
  { id: "packages", icon: "package", name: N_("Integridade de pacotes"), desc: N_("Arquivos alterados e dependências quebradas") },
  { id: "disk-health", icon: "disk", name: N_("Saúde do disco"), desc: N_("SMART e contadores de erro do btrfs") },
  { id: "fs-scrub", icon: "shield-check", name: N_("Sistema de arquivos"), desc: N_("Confere as somas de verificação do btrfs, sem alterar nada") },
  { id: "cpu-stress", icon: "cpu", name: N_("Processador sob carga"), desc: N_("Erros de cálculo, temperatura e redução de velocidade") },
  { id: "memory-stress", icon: "layers", name: N_("Memória sob carga"), desc: N_("Procura dados corrompidos na RAM") },
  { id: "disk-bench", icon: "gauge", name: N_("Desempenho do disco"), desc: N_("Leitura e escrita; grava cerca de 5 GiB") },
  { id: "gpu", icon: "monitor", name: N_("GPUs"), desc: N_("Drivers, renderização e pontuação de cada GPU") },
  { id: "logs", icon: "logs", name: N_("Logs e falhas"), desc: N_("Erros por boot, falhas de programas e serviços com problema") },
  { id: "platform", icon: "activity", name: N_("Estado da plataforma"), desc: N_("Temperaturas, bateria, firmware, relógio e Secure Boot") },
];
const NAMES = Object.fromEntries(TESTS.map((test) => [test.id, test.name]));

// Ordem de execução do vegad: leituras rápidas primeiro.
const DAEMON_ORDER = ["packages", "disk-health", "logs", "platform", "fs-scrub", "cpu-stress", "memory-stress", "disk-bench"];
const MODES = {
  quick: { tests: ["packages", "disk-health", "logs", "platform", "cpu-stress", "memory-stress"], gpu: false, duration: 60 },
  full: { tests: DAEMON_ORDER, gpu: true, duration: 240 },
};
const LOAD_TESTS = new Set(["cpu-stress", "memory-stress", "disk-bench"]);

const VERDICTS = {
  ok: { label: N_("Saudável"), color: "green", icon: "check-circle" },
  attention: { label: N_("Atenção"), color: "yellow", icon: "alert" },
  fail: { label: N_("Falha"), color: "red", icon: "alert-circle" },
  unavailable: { label: N_("Indisponível"), color: "", icon: "info" },
  cancelled: { label: N_("Cancelado"), color: "", icon: "x" },
  aborted: { label: N_("Interrompido"), color: "yellow", icon: "alert" },
};
const verdictBadge = (verdict) => {
  const v = VERDICTS[verdict] ?? VERDICTS.unavailable;
  return badge(t(v.label), v.color, v.icon);
};

function reasonText(reason) {
  if (!reason) return "";
  if (reason.startsWith("missing-tool:")) return t("Instale o pacote {package}.", { package: reason.slice("missing-tool:".length) });
  if (reason === "not-btrfs") return t("A raiz do sistema não usa btrfs.");
  if (reason === "not-implemented") return t("Atualize o vegad para usar este teste.");
  return t("Indisponível neste sistema");
}

// ---------- O que fazer com cada achado ----------

// Consertos que o vegad aplica (diagnostics-v2), na ordem do contrato.
const REMEDY_ORDER = ["reinstall-packages", "verify-dependencies", "restart-units", "enable-ntp"];

function remedyText(remedy, args = []) {
  switch (remedy) {
    case "reinstall-packages": return t("Reinstalar os pacotes: {packages}", { packages: args.join(", ") });
    case "verify-dependencies": return t("Instalar as dependências que faltam");
    case "restart-units": return t("Reiniciar os serviços: {units}", { units: args.join(", ") });
    case "enable-ntp": return t("Ligar a sincronização de hora pela rede");
    default: return remedy;
  }
}

const REPAIR_STAGES = {
  "repair:reinstall-packages": N_("Reinstalando pacotes"),
  "repair:verify-dependencies": N_("Corrigindo dependências"),
  "repair:restart-units": N_("Reiniciando serviços"),
  "repair:enable-ntp": N_("Ligando a sincronização de hora"),
};
const stageName = (id) => t(NAMES[id] ?? REPAIR_STAGES[id] ?? id);

const BACKUP_NOW = { text: N_("Faça backup dos seus arquivos agora e planeje a troca do disco. O Vega não tenta consertar disco com defeito."), go: ["backup"], label: N_("Abrir Backup") };
const COOLING = { text: N_("Limpe as saídas de ar, use o computador sobre uma superfície rígida e evite deixá-lo no colo durante tarefas pesadas.") };
const RETRY = { text: N_("Tente de novo. Se voltar a acontecer, exporte o relatório e abra uma issue.") };

// Orientação para achados sem conserto automático, por código.
const GUIDE = {
  "smart.failed": BACKUP_NOW, "smart.critical-warning": BACKUP_NOW, "smart.spare-low": BACKUP_NOW,
  "smart.media-errors": BACKUP_NOW, "smart.sectors": BACKUP_NOW, "smart.prefail": BACKUP_NOW,
  "smart.self-test-errors": BACKUP_NOW, "smart.error-log": BACKUP_NOW, "btrfs.device-errors": BACKUP_NOW,
  "scrub.errors": BACKUP_NOW,
  "smart.worn": { text: N_("O disco está chegando ao fim da vida útil. Mantenha backups em dia e planeje a troca."), go: ["backup"], label: N_("Abrir Backup") },
  "smart.hot": COOLING, "platform.hot-idle": COOLING, "cpu.slow-under-load": COOLING, "thermal.abort": COOLING,
  "platform.battery-worn": { text: N_("A bateria guarda bem menos carga do que quando era nova. Considere trocá-la.") },
  "platform.firmware-updates": { text: N_("Há atualizações de firmware. Instale-as pela central de software do GNOME e reinicie quando pedir.") },
  "platform.ntp": { text: N_("A sincronização está ligada, mas sem resposta. Confira a conexão com a internet."), go: ["datetime"], label: N_("Abrir Data e hora") },
  "platform.secure-boot-off": { text: N_("Se quiser essa proteção, ative o Secure Boot nas configurações de firmware (UEFI) do computador.") },
  "platform.kernel-tainted": { text: N_("O kernel registrou uma falha. Reinicie o computador; se voltar a acontecer, veja os logs."), go: ["logs"], label: N_("Abrir Logs") },
  "logs.coredumps": { text: N_("Os dados brutos mostram quais programas falharam. Manter o sistema atualizado costuma resolver."), go: ["software", "updates"], label: N_("Ver atualizações") },
  "gpu.rtd3-awake": { text: N_("Algum programa está usando a NVIDIA. Feche-o para economizar bateria."), go: ["monitor"], label: N_("Abrir Monitor") },
  "gpu.bench-failed": { text: N_("Confira o estado do driver da GPU."), go: ["hardware", "nvidia"], label: N_("Ver NVIDIA") },
  "power.abort": { text: N_("Conecte o carregador e execute o teste de novo.") },
  "memory.insufficient": { text: N_("Feche alguns programas para liberar memória e execute o teste de novo.") },
  "bench.insufficient-space": { text: N_("Libere ao menos 10 GiB no disco e execute o teste de novo."), go: ["storage"], label: N_("Abrir Armazenamento") },
  "stage.timeout": RETRY, "stage.tool-failed": RETRY, "bench.failed": RETRY, "stress.incomplete": RETRY, "scrub.incomplete": RETRY,
  "repair.failed": RETRY, "repair.unit-failed": { text: N_("O serviço continua falhando. Veja o log dele para entender o motivo."), go: ["logs"], label: N_("Abrir Logs") },
};

/** Consertos propostos pelo relatório: id → argumentos somados. */
function reportRemedies(report) {
  const found = new Map();
  for (const stage of report.stages ?? []) {
    for (const finding of stage.findings ?? []) {
      if (!finding.remedy) continue;
      const args = found.get(finding.remedy) ?? [];
      for (const arg of finding.remedyArgs ?? []) if (!args.includes(arg)) args.push(arg);
      found.set(finding.remedy, args);
    }
  }
  return REMEDY_ORDER.filter((id) => found.has(id)).map((id) => ({ id, args: found.get(id) }));
}

// ---------- Últimos resultados (local, sem senha) ----------

const STORE_KEY = "vega.diagnostics";

function loadLast() {
  try {
    return JSON.parse(localStorage.getItem(STORE_KEY)) ?? { stages: {}, job: 0, gpu: null };
  } catch {
    return { stages: {}, job: 0, gpu: null };
  }
}

function saveLast(last) {
  localStorage.setItem(STORE_KEY, JSON.stringify(last));
}

// ---------- Relatório ----------

/** Junta o relatório do vegad com a etapa de GPU do Vega. */
function mergeReport(report, gpu) {
  const stages = [...(report?.stages ?? [])];
  if (gpu && !stages.some((stage) => stage.id === "gpu")) stages.push(gpu);
  return { ...report, stages };
}

function groups(report) {
  const healthy = [];
  const attention = [];
  const info = [];
  for (const stage of report.stages) {
    if (stage.verdict === "ok") healthy.push(stage);
    if (stage.verdict === "unavailable") info.push({ stage, finding: { severity: "info", code: "stage.unavailable", text: reasonText(stage.metrics?.reason) || stage.summary } });
    for (const finding of stage.findings ?? []) {
      (finding.severity === "info" ? info : attention).push({ stage, finding });
    }
  }
  attention.sort((a, b) => (a.finding.severity === "fail" ? -1 : 0) - (b.finding.severity === "fail" ? -1 : 0));
  return { healthy, attention, info };
}

function findingRow({ stage, finding }, actions = {}) {
  const raw = finding.raw?.trim();
  const tone = finding.severity === "fail" ? "red" : finding.severity === "attention" ? "yellow" : "";
  const copy = raw && button(t("Copiar"), {
    icon: "copy", size: "sm",
    onClick: async () => {
      await navigator.clipboard?.writeText(raw);
      toast(t("Copiado"));
    },
  });
  const guide = finding.severity !== "info" || finding.code === "platform.firmware-updates" ? GUIDE[finding.code] : null;
  const fix = finding.remedy && actions.repair
    ? button(t("Resolver"), { kind: "primary", size: "sm", icon: "zap", onClick: () => actions.repair([{ id: finding.remedy, args: finding.remedyArgs ?? [] }]) })
    : null;
  const shortcut = !fix && guide?.go && actions.go && button(t(guide.label), { size: "sm", icon: "chevron-right", onClick: () => actions.go(...guide.go) });
  return h("div", { class: "diag-finding" },
    row({
      icon: finding.severity === "info" ? "info" : "alert",
      tint: tone,
      title: finding.text,
      subtitle: stageName(stage.id),
      wrap: true,
      suffix: fix || shortcut || null,
    }),
    finding.remedy && h("p", { class: "diag-guide small" }, icon("zap", 14), remedyText(finding.remedy, finding.remedyArgs)),
    !finding.remedy && guide && h("p", { class: "diag-guide small" }, icon("info", 14), t(guide.text)),
    raw && h("details", { class: "diag-raw" }, h("summary", {}, t("Dados brutos")), h("pre", { class: "mono selectable" }, raw), h("div", { class: "row-flex" }, copy)));
}

function reportView(report, actions = {}) {
  const { healthy, attention, info } = groups(report);
  const remedies = reportRemedies(report);
  const block = (title, ic, tone, items, render) => items.length > 0 && h("div", { class: "diag-group" },
    h("h4", {}, icon(ic, 16, `ic ${tone}`), title, h("span", { class: "n" }, items.length)),
    list(items.map(render)));
  const render = (entry) => findingRow(entry, actions);
  return h("div", { class: "diag-report" },
    h("p", { class: "small muted" }, t("Resultado: {result} · {date}", {
      result: t(VERDICTS[report.result]?.label ?? N_("Em andamento")),
      date: report.finished ? dateTime(report.finished) : dateTime(report.started),
    })),
    remedies.length > 0 && actions.repair && banner("info", "zap",
      t("O Vega pode corrigir {count} problema(s) com segurança", { count: remedies.length }),
      t("Antes ele cria um snapshot e, depois, roda de novo os testes afetados."),
      button(t("Resolver tudo o que for seguro"), { kind: "primary", icon: "zap", onClick: () => actions.repair(remedies) })),
    block(t("Atenção"), "alert", "yellow", attention, render),
    block(t("Saudável"), "check-circle", "green", healthy, (stage) => row({ icon: "check", tint: "green", title: stageName(stage.id), subtitle: stage.summary, wrap: true })),
    block(t("Informações e ruído conhecido"), "info", "", info, render));
}

function markdown(report) {
  const lines = [`# ${t("Diagnóstico de saúde do Lyra OS")}`, ""];
  const host = report.host ?? {};
  lines.push(`- ${t("Resultado")}: ${t(VERDICTS[report.result]?.label ?? N_("Em andamento"))}`);
  lines.push(`- ${t("Início")}: ${report.started ?? ""}`);
  for (const key of ["product", "bios", "os", "kernel", "vegad", "hostname"]) if (host[key]) lines.push(`- ${key}: ${host[key]}`);
  for (const stage of report.stages) {
    lines.push("", `## ${stageName(stage.id)}: ${t(VERDICTS[stage.verdict]?.label ?? stage.verdict)}`, "", stage.summary || "");
    for (const finding of stage.findings ?? []) {
      lines.push("", `- **${finding.severity}** \`${finding.code}\`: ${finding.text}`);
      if (finding.raw?.trim()) lines.push("", "```", finding.raw.trim(), "```");
    }
    if (stage.metrics && Object.keys(stage.metrics).length) lines.push("", "```json", JSON.stringify(stage.metrics, null, 2), "```");
  }
  return lines.join("\n") + "\n";
}

// ---------- Cartão ----------

export function diagnosticsCard(ctx) {
  const root = h("div", { class: "card diag-card", id: "sec-diagnostics" });
  let tests = [];
  let last = loadLast();
  let mode = "quick";
  let running = null; // { job, tests, gpu, stage, percent, message, sample, error }
  let error = null;

  const unlisten = [
    on("diagnostics-event", (event) => {
      if (!running || event.job !== running.job) return;
      switch (event.type) {
        case "progress":
          running.stage = event.stage;
          running.percent = event.percent;
          if (event.message) running.message = event.message;
          break;
        case "sample":
          running.sample = event;
          break;
        case "stage":
          last.stages[event.stage] = { verdict: event.verdict, summary: event.summary, at: Date.now() };
          saveLast(last);
          (running.done ??= new Set()).add(event.stage);
          break;
        case "finished":
          last.job = event.job;
          saveLast(last);
          afterDaemon(event);
          return;
      }
      paint();
    }),
    on("diagnostics-gpu", (event) => {
      if (!running) return;
      running.stage = "gpu";
      running.percent = event.percent;
      running.message = event.message;
      paint();
    }),
  ];
  ctx.cleanup?.(() => unlisten.forEach((fn) => fn()));

  async function load() {
    if (!hasCapability("diagnostics-v1")) {
      clear(root, banner("info", "info", t("Diagnóstico"), t("Atualize o vegad para usar o diagnóstico de saúde.")));
      return;
    }
    try {
      tests = await call("diagnostics_tests");
      const current = await call("diagnostics_current");
      if (current.job_id) {
        // Um diagnóstico começado antes desta janela abrir.
        running = { job: current.job_id, tests: current.tests, gpu: false, stage: current.stage, percent: current.percent, message: "" };
        await call("diagnostics_follow", { job: current.job_id });
      }
    } catch (e) {
      error = e.message;
    }
    paint();
  }

  const available = (id) => id === "gpu" || tests.find((test) => test.id === id)?.available;
  const estimate = (id) => (id === "gpu" ? 240 : tests.find((test) => test.id === id)?.estimated_seconds ?? 0);

  async function start(daemonTests, { gpu, durationSeconds }) {
    error = null;
    const wanted = daemonTests.filter((id) => tests.find((test) => test.id === id));
    running = { job: 0, tests: [...wanted, ...(gpu ? ["gpu"] : [])], gpu, stage: wanted[0] ?? "gpu", percent: 0, message: t("Iniciando…") };
    paint();
    if (wanted.length === 0) return runGpu(null);
    try {
      running.job = await call("diagnostics_start", { tests: wanted, options: { durationSeconds } });
      paint();
    } catch (e) {
      running = null;
      error = e.message;
      paint(wanted.some((id) => LOAD_TESTS.has(id)) ? { retryWithout: () => start(wanted.filter((id) => !LOAD_TESTS.has(id)), { gpu: false, durationSeconds }) } : {});
    }
  }

  function afterDaemon(event) {
    if (running?.gpu) return runGpu(event);
    if (running?.repair) {
      running = null;
      paint();
      toast(event.result === "ok" ? t("Problemas resolvidos") : t("Correção concluída; veja o relatório"), { action: t("Ver relatório"), onAction: openReport });
      return;
    }
    finish(event.result);
  }

  async function runGpu() {
    running.stage = "gpu";
    running.percent = 0;
    paint();
    try {
      const stage = await call("gpu_bench");
      last.stages.gpu = { verdict: stage.verdict, summary: stage.summary, at: Date.now() };
      (running.done ??= new Set()).add("gpu");
      last.gpu = { ...stage, job: last.job };
      saveLast(last);
    } catch (e) {
      error = e.message;
    }
    finish();
  }

  function finish() {
    running = null;
    if (!ctx.visible()) return;
    paint();
    toast(t("Diagnóstico concluído"));
  }

  async function cancel() {
    if (!running) return;
    running.message = t("Cancelando…");
    paint();
    try {
      if (running.stage === "gpu") await call("gpu_bench_cancel");
      else if (running.job) await call("diagnostics_cancel", { job: running.job });
    } catch (e) {
      error = e.message;
    }
  }

  async function fetchReport(redact) {
    const raw = await call("diagnostics_report", { job: last.job || 0, redact });
    const report = JSON.parse(raw);
    return mergeReport(report, last.gpu?.job === report.jobId ? last.gpu : null);
  }

  async function openReport() {
    try {
      const report = await fetchReport(false);
      const includeSensitive = h("input", { type: "checkbox" });
      const exportAs = async (format) => {
        try {
          const data = includeSensitive.checked ? report : await fetchReport(true);
          const stamp = (data.started ?? "").slice(0, 10) || "diagnostico";
          const ok = format === "json"
            ? await call("diagnostics_export", { fileName: `vega-diagnostico-${stamp}.json`, content: JSON.stringify(data, null, 2) })
            : await call("diagnostics_export", { fileName: `vega-diagnostico-${stamp}.md`, content: markdown(data) });
          if (ok) toast(t("Relatório salvo"));
        } catch (e) {
          toast(e.message, { ic: "alert-circle" });
        }
      };
      let close = null;
      const actions = {
        go: (page, sub) => { close?.(); ctx.navigate?.(page, sub); },
        repair: hasCapability("diagnostics-v2") && !running ? async (remedies) => {
          if (await repair(report.jobId, remedies)) close?.();
        } : null,
      };
      close = sheet(t("Relatório do diagnóstico"), h("div", {},
        reportView(report, actions),
        h("div", { class: "card pad diag-export" },
          h("label", { class: "row-flex small" }, includeSensitive, t("Incluir número de série, endereço MAC e nome do computador")),
          h("div", { class: "row-flex" },
            button(t("Exportar JSON"), { icon: "download", onClick: () => exportAs("json") }),
            button(t("Exportar texto (Markdown)"), { icon: "download", onClick: () => exportAs("md") })))));
    } catch (e) {
      toast(e.message, { ic: "alert-circle" });
    }
  }

  /** Confirma e pede ao vegad os consertos; devolve true se começou. */
  async function repair(job, remedies) {
    const ok = await confirm({
      title: t("Resolver os problemas encontrados?"),
      text: t("O Vega vai fazer o seguinte. A senha de administrador será pedida."),
      ic: "zap",
      facts: [
        ...remedies.map((remedy) => ({ icon: "check", text: remedyText(remedy.id, remedy.args) })),
        { icon: "history", text: t("Antes, um snapshot permite desfazer tudo.") },
        { icon: "refresh", text: t("Depois, os testes afetados rodam de novo para confirmar.") },
      ],
      confirmLabel: t("Resolver"),
      required: true,
    });
    if (!ok) return false;
    error = null;
    try {
      const id = await call("diagnostics_repair", { job, remedies: remedies.map((remedy) => remedy.id) });
      running = { job: id, tests: [], gpu: false, repair: true, stage: `repair:${remedies[0].id}`, percent: 0, message: t("Iniciando…") };
      paint();
      return true;
    } catch (e) {
      toast(e.message, { ic: "alert-circle" });
      return false;
    }
  }

  async function runOne(id) {
    if (LOAD_TESTS.has(id) || id === "disk-bench") {
      const ok = await confirm({
        title: t("Executar {test}?", { test: t(NAMES[id]) }),
        text: id === "disk-bench"
          ? t("O teste grava cerca de 5 GiB no disco e apaga tudo no fim. Mantenha o computador na tomada.")
          : t("O teste deixa o computador lento e quente por alguns minutos. Mantenha o computador na tomada; ele para sozinho se esquentar demais."),
        ic: "zap",
        confirmLabel: t("Executar"),
        required: false,
      });
      if (!ok) return;
    }
    if (id === "gpu") return start([], { gpu: true });
    return start([id], { gpu: false, durationSeconds: 240 });
  }

  async function runAll() {
    const chosen = MODES[mode];
    const ok = await confirm({
      title: mode === "quick" ? t("Executar o diagnóstico rápido?") : t("Executar o diagnóstico completo?"),
      text: mode === "quick"
        ? t("Cerca de 3 minutos: pacotes, disco, logs, plataforma e 1 minuto de carga no processador e na memória.")
        : t("Cerca de 15 minutos: todos os testes, com 4 minutos de carga no processador e na memória, verificação do sistema de arquivos e desempenho do disco (grava cerca de 5 GiB)."),
      ic: "activity",
      confirmLabel: t("Executar"),
      required: false,
    });
    if (!ok) return;
    start(chosen.tests, { gpu: chosen.gpu, durationSeconds: chosen.duration });
  }

  function telemetry(sample) {
    if (!sample) return null;
    const parts = [];
    if (sample.pkg_temp_c != null) parts.push(t("CPU {temp} °C", { temp: sample.pkg_temp_c }));
    if (sample.avg_mhz != null) parts.push(t("{ghz} GHz", { ghz: (sample.avg_mhz / 1000).toLocaleString(locale, { maximumFractionDigits: 1 }) }));
    if (sample.nvme_temp_c != null) parts.push(t("SSD {temp} °C", { temp: sample.nvme_temp_c }));
    if (sample.throttle_events) parts.push(t("{count} reduções de velocidade", { count: sample.throttle_events }));
    return parts.length ? h("div", { class: "diag-telemetry small mono" }, icon("activity", 14), parts.join(" · ")) : null;
  }

  function testRow(test) {
    const info = tests.find((item) => item.id === test.id);
    const isAvailable = available(test.id);
    const result = last.stages[test.id];
    // Um item que já terminou neste job mostra o resultado, não "Na fila".
    const finished = running?.done?.has(test.id);
    const active = running?.stage === test.id && !finished;
    const queued = running && running.tests.includes(test.id) && !active && !finished;
    const subtitle = !isAvailable && info ? reasonText(info.reason) : result?.summary ? `${t(test.desc)} · ${result.summary}` : t(test.desc);
    let suffix;
    if (active) {
      suffix = h("span", { class: "row-flex" }, h("span", { class: "small muted" }, `${running.percent ?? 0}%`), h("span", { class: "spinner" }));
    } else if (queued) {
      suffix = badge(t("Na fila"), "");
    } else {
      suffix = h("span", { class: "row-flex" },
        result && verdictBadge(result.verdict),
        h("span", { class: "small faint" }, duration(estimate(test.id))),
        button(t("Executar"), { size: "sm", icon: "play", disabled: !isAvailable || Boolean(running), onClick: () => runOne(test.id) }));
    }
    return row({ icon: test.icon, tint: active ? "tint" : "", title: t(test.name), subtitle, wrap: true, suffix });
  }

  function paint({ retryWithout } = {}) {
    const header = h("div", { class: "diag-head" },
      h("div", { class: "spacer" },
        h("strong", {}, t("Diagnóstico de saúde")),
        h("div", { class: "small muted" }, last.job
          ? t("Última execução: {date}", { date: dateTime(Math.max(...Object.values(last.stages).map((stage) => stage.at ?? 0))) })
          : t("Testa disco, memória, processador, GPU e sistema sem alterar nada."))),
      !running && segmented([{ id: "quick", label: t("Rápido") }, { id: "full", label: t("Completo") }], mode, (id) => { mode = id; }),
      !running && button(t("Executar todos"), { kind: "primary", icon: "play", onClick: runAll }),
      !running && last.job > 0 && button(t("Ver relatório"), { icon: "logs", onClick: openReport }));
    const progress = running && h("div", { class: "diag-running" },
      h("div", { class: "row-flex" },
        h("span", { class: "spacer small" }, `${stageName(running.stage)} · ${running.message || ""}`),
        button(t("Cancelar"), { size: "sm", icon: "stop", onClick: cancel })),
      meter((running.percent ?? 0) / 100, { warnAt: 2, critAt: 2 }),
      telemetry(running.sample));
    clear(root,
      header,
      error && banner("danger", "alert-circle", t("O diagnóstico não foi concluído"), error, retryWithout && button(t("Executar só os testes sem carga"), { size: "sm", onClick: retryWithout })),
      progress,
      list(TESTS.map(testRow)));
  }

  load();
  return root;
}

export const diagnosticsSection = { id: "diagnostics", label: N_("Diagnóstico"), icon: "activity", keywords: ["diagnostico", "saude", "teste", "smart", "memoria", "stress", "disco", "temperatura"] };
