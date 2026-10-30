// "Minha internet não funciona" (add-network-troubleshooter): roda só o teste
// `network` do vegad, que não pede senha (org.lyraos.vega.network.diagnose),
// e mostra passo a passo onde a conexão quebrou. Consertar usa o Resolver do
// diagnóstico (RepairFindings), que pede a senha de administrador.
import { call, on } from "../api.js";
import { N_, t } from "../i18n.js";
import { hasCapability } from "../store.js";
import { h, icon, clear, button, banner, meter, sheet, toast } from "../ui.js";

const STEPS = [
  { id: "link", icon: "wifi", name: N_("Cabo ou Wi-Fi") },
  { id: "address", icon: "globe", name: N_("Endereço IP") },
  { id: "gateway", icon: "server", name: N_("Roteador") },
  { id: "dns", icon: "search", name: N_("DNS (nomes dos sites)") },
  { id: "internet", icon: "external", name: N_("Internet") },
];

const STATUS = {
  ok: { icon: "check-circle", tone: "green", label: N_("Funcionando") },
  warn: { icon: "alert", tone: "yellow", label: N_("Atenção") },
  fail: { icon: "alert-circle", tone: "red", label: N_("Aqui está o problema") },
  skipped: { icon: "minus", tone: "", label: N_("Não testado") },
};

// O que dizer e oferecer para cada achado.
const GUIDE = {
  "network.no-link": N_("Confira se o cabo está bem encaixado ou se o Wi-Fi está ligado e conectado a uma rede conhecida."),
  "network.wifi-weak": N_("O sinal está fraco. Aproxime-se do roteador ou tire obstáculos do caminho."),
  "network.no-address": N_("O roteador não entregou um endereço. Pedir de novo costuma resolver; se não, reinicie o roteador."),
  "network.gateway-unreachable": N_("O computador está conectado, mas o roteador não responde. Desligue o roteador da tomada, espere 30 segundos e ligue de novo."),
  "network.dns-failed": N_("A conexão funciona, mas os nomes dos sites não são traduzidos. Um DNS público resolve até a próxima conexão."),
  "network.no-internet": N_("O roteador responde, mas a internet não chega até ele. Verifique o modem ou fale com o provedor."),
  "network.captive-portal": N_("Esta rede exige login numa página antes de liberar a internet."),
  "network.vpn": N_("Se a internet só falha com a VPN ligada, o problema está nela."),
};

const REMEDIES = {
  reconnect: N_("Reconectar"),
  "renew-dhcp": N_("Pedir um novo endereço"),
  "temporary-dns": N_("Usar DNS público nesta conexão"),
};

/** Botão para o topo da página Rede; null se o vegad não tem o teste. */
export function troubleshootButton() {
  if (!hasCapability("diagnostics-v4")) return null;
  return button(t("Minha internet não funciona"), { icon: "wifi", kind: "primary", onClick: openTroubleshooter });
}

function openTroubleshooter() {
  const body = h("div", { class: "net-check" });
  let job = 0;
  let stage = null; // etapa `network` do relatório
  let reportJob = 0;
  let progress = { percent: 0, message: t("Iniciando…") };
  let running = true;
  // Eventos de um job ainda sem id: o teste leva menos de um segundo.
  const early = [];

  const off = on("diagnostics-event", (event) => {
    if (!job) {
      early.push(event);
      return;
    }
    handle(event);
  });

  // Fechar o painel desliga o ouvinte.
  const close = sheet(t("Minha internet não funciona"), body, { wide: false, onClose: () => off() });

  function handle(event) {
    if (event.job !== job) return;
    if (event.type === "progress") {
      progress = { percent: event.percent, message: event.message || progress.message };
      paint();
    } else if (event.type === "finished") {
      running = false;
      loadReport(event.job);
    }
  }

  async function begin(promise, message = t("Iniciando…")) {
    running = true;
    progress = { percent: 0, message };
    paint();
    try {
      job = await promise;
      early.splice(0).forEach(handle);
    } catch (e) {
      running = false;
      clear(body, banner("danger", "alert-circle", t("Não foi possível verificar a rede"), e.message));
    }
  }

  async function loadReport(id) {
    try {
      const report = JSON.parse(await call("diagnostics_report", { job: id, redact: false }));
      stage = (report.stages ?? []).filter((s) => s.id === "network").pop() ?? null;
      reportJob = report.jobId;
    } catch (e) {
      stage = null;
      toast(e.message, { ic: "alert-circle" });
    }
    paint();
  }

  async function fix(index, finding) {
    if (!hasCapability("diagnostics-actions-v1")) return;
    job = 0;
    await begin(call("diagnostics_repair_findings", { job: reportJob, findings: [["network", index]] }), t(REMEDIES[finding.remedy] ?? finding.remedy));
  }

  function stepRow(step, status) {
    const s = STATUS[status?.status] ?? STATUS.skipped;
    return h("div", { class: `net-step ${status?.status ?? "skipped"}` },
      h("span", { class: `lead ${s.tone}` }, icon(step.icon, 18)),
      h("div", { class: "body" },
        h("div", { class: "title" }, t(step.name)),
        status?.detail && h("div", { class: "small muted mono selectable" }, status.detail)),
      h("span", { class: `small ${s.tone}` }, icon(s.icon, 15), " ", t(s.label)));
  }

  function findingBlock(finding, index) {
    const guide = GUIDE[finding.code];
    const actions = [];
    if (finding.remedy && REMEDIES[finding.remedy] && hasCapability("diagnostics-actions-v1")) {
      actions.push(button(t(REMEDIES[finding.remedy]), { kind: "primary", icon: "zap", onClick: () => fix(index, finding) }));
    }
    if (finding.code === "network.captive-portal" && stage?.metrics?.portalUrl) {
      actions.push(button(t("Abrir página de login"), { kind: "primary", icon: "external", onClick: () => call("open_portal", { uri: stage.metrics.portalUrl }).catch((e) => toast(e.message, { ic: "alert-circle" })) }));
    }
    const tone = finding.severity === "fail" ? "danger" : finding.severity === "attention" ? "warning" : "info";
    return banner(tone, finding.severity === "info" ? "info" : "alert", finding.text, guide ? t(guide) : "",
      actions.length ? h("div", { class: "row-flex" }, actions) : null);
  }

  function paint() {
    const steps = new Map((stage?.metrics?.steps ?? []).map((s) => [s.id, s]));
    if (running) {
      clear(body,
        h("p", { class: "muted" }, progress.message),
        meter((progress.percent ?? 0) / 100, { warnAt: 2, critAt: 2 }),
        h("div", { class: "net-steps" }, STEPS.map((step) => stepRow(step, null))));
      return;
    }
    if (!stage) {
      clear(body, banner("danger", "alert-circle", t("Não foi possível verificar a rede"), ""));
      return;
    }
    const ok = stage.verdict === "ok";
    clear(body,
      ok
        ? banner("info", "check-circle", t("A conexão está funcionando"), t("Cabo ou Wi-Fi, endereço, roteador, DNS e internet responderam. Se um site específico não abre, o problema pode ser nele."))
        : null,
      h("div", { class: "net-steps" }, STEPS.map((step) => stepRow(step, steps.get(step.id)))),
      (stage.findings ?? []).map((finding, index) => findingBlock(finding, index)),
      h("div", { class: "row-flex net-check-actions" },
        h("span", { class: "spacer" }),
        button(t("Verificar de novo"), { icon: "refresh", onClick: start }),
        button(t("Fechar"), { onClick: () => close() })));
  }

  function start() {
    job = 0;
    stage = null;
    begin(call("diagnostics_start", { tests: ["network"], options: {} }));
  }

  start();
}
