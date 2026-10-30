// Diagnóstico de saúde (vega#165): os testes do vegad (interface
// org.lyraos.Vega1.Diagnostics) e o teste de GPU, que roda no próprio Vega.
//
// Abrir a página nunca pede senha: o último resultado de cada teste vem dos
// sinais StageFinished, guardados aqui. O relatório completo (que inclui
// trechos do journal e exige logs.read-admin) só é buscado quando o usuário
// pede para vê-lo ou exportá-lo.

import { call, on } from "../api.js";
import { runSoftware } from "../tasks.js";
import { buildHelpText, cut } from "./help-text.js";
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
  { id: "security", icon: "shield", name: N_("Segurança"), desc: N_("Firewall, portas expostas, correções de segurança, SSH, criptografia e contas") },
  { id: "boot", icon: "power", name: N_("Tempo de boot"), desc: N_("Quanto o computador leva para ligar e o que atrasa") },
  { id: "network", icon: "wifi", name: N_("Rede e internet"), desc: N_("Cabo ou Wi-Fi, endereço, roteador, DNS e acesso à internet") },
];
const NAMES = Object.fromEntries(TESTS.map((test) => [test.id, test.name]));

// Ordem de execução do vegad: leituras rápidas primeiro.
const DAEMON_ORDER = ["packages", "disk-health", "logs", "platform", "security", "boot", "network", "fs-scrub", "cpu-stress", "memory-stress", "disk-bench"];
const MODES = {
  quick: { tests: ["packages", "disk-health", "logs", "platform", "security", "boot", "network", "cpu-stress", "memory-stress"], gpu: false, duration: 60 },
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
const REMEDY_ORDER = ["reinstall-packages", "verify-dependencies", "restart-units", "enable-ntp", "enable-firewall", "close-port", "apply-security-patches", "reconnect", "renew-dhcp", "temporary-dns", "disable-wait-online", "disable-service"];

function remedyText(remedy, args = []) {
  if (!args.length) {
    // A lista de testes só conhece os ids (Remedies), sem os argumentos.
    switch (remedy) {
      case "reinstall-packages": return t("Reinstalar os pacotes alterados");
      case "restart-units": return t("Reiniciar os serviços que falharam");
      case "close-port": return t("Fechar as portas expostas no firewall");
    }
  }
  switch (remedy) {
    case "reinstall-packages": return t("Reinstalar os pacotes: {packages}", { packages: args.join(", ") });
    case "verify-dependencies": return t("Instalar as dependências que faltam");
    case "restart-units": return t("Reiniciar os serviços: {units}", { units: args.join(", ") });
    case "enable-ntp": return t("Ligar a sincronização de hora pela rede");
    case "enable-firewall": return t("Ligar o firewall");
    case "close-port": return t("Fechar no firewall: {ports}", { ports: args.map((arg) => arg.replace("|service:", ": serviço ").replace("|port:", ": porta ")).join(", ") });
    case "apply-security-patches": return t("Instalar as correções de segurança");
    case "disable-wait-online": return t("Não esperar pela rede para terminar o boot");
    case "disable-service": return args.length ? t("Desabilitar no boot: {units}", { units: args.join(", ") }) : t("Desabilitar os serviços lentos que você habilitou");
    case "reconnect": return t("Reconectar a rede ({device})", { device: args.join(", ") || "Wi-Fi" });
    case "renew-dhcp": return t("Pedir um novo endereço IP ({device})", { device: args.join(", ") || "rede" });
    case "temporary-dns": return t("Usar DNS público só nesta conexão ({device})", { device: args.join(", ") || "rede" });
    default: return remedy;
  }
}

const REPAIR_STAGES = {
  "repair:reinstall-packages": N_("Reinstalando pacotes"),
  "repair:verify-dependencies": N_("Corrigindo dependências"),
  "repair:restart-units": N_("Reiniciando serviços"),
  "repair:enable-ntp": N_("Ligando a sincronização de hora"),
  "repair:enable-firewall": N_("Ligando o firewall"),
  "repair:close-port": N_("Fechando portas"),
  "repair:apply-security-patches": N_("Instalando correções de segurança"),
  "repair:disable-wait-online": N_("Desligando a espera pela rede no boot"),
  "repair:disable-service": N_("Desabilitando serviços"),
  "repair:reconnect": N_("Reconectando a rede"),
  "repair:renew-dhcp": N_("Pedindo um novo endereço"),
  "repair:temporary-dns": N_("Usando DNS público"),
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
  "platform.firmware-updates": { text: N_("Há atualizações de firmware. O Vega instala pelo fwupd e avisa se for preciso reiniciar.") },
  "backup.stale": { text: N_("Faça um backup agora, com o disco de backup conectado."), go: ["backup"], label: N_("Abrir Backup") },
  "backup.none": { text: N_("Configure um backup para não perder seus arquivos se o disco falhar."), go: ["backup"], label: N_("Abrir Backup") },
  "storage.low-space": { text: N_("Pouco espaço livre faz atualizações e snapshots falharem. Veja o que dá para apagar com segurança."), go: ["storage", "cleanup"], label: N_("Abrir Liberar espaço") },
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
  "boot.slow": { text: N_("Veja abaixo quais serviços atrasam o boot. Desabilitar o que você não usa acelera a inicialização.") },
  "boot.wait-online": { text: N_("Algum programa precisa da rede logo no boot, por isso a espera fica. Se não usa esse programa, desabilite-o em Serviços."), go: ["services"], label: N_("Abrir Serviços") },
  "boot.missing-device": { text: N_("O /etc/fstab ainda monta um disco que foi removido. Tire a linha dele do fstab ou marque-a com nofail."), go: ["storage"], label: N_("Abrir Armazenamento") },
  "boot.slow-service": { text: N_("Este serviço faz parte do sistema. Se o atraso continuar, veja o log dele."), go: ["logs"], label: N_("Abrir Logs") },
  "network.no-link": { text: N_("Confira o cabo ou ligue o Wi-Fi e escolha uma rede."), go: ["network"], label: N_("Abrir Rede") },
  "network.wifi-weak": { text: N_("Aproxime-se do roteador ou tire obstáculos do caminho.") },
  "network.gateway-unreachable": { text: N_("Desligue o roteador da tomada, espere 30 segundos e ligue de novo.") },
  "network.no-internet": { text: N_("O roteador responde, mas a internet não chega até ele. Verifique o modem ou fale com o provedor.") },
  "network.captive-portal": { text: N_("Abra a página de login da rede em Rede → Minha internet não funciona."), go: ["network"], label: N_("Abrir Rede") },
  "security.ssh-root-password": { text: N_("Desative o login do root por senha no SSH (PermitRootLogin) ou desligue o servidor SSH se não usa."), go: ["services"], label: N_("Abrir Serviços") },
  "security.ssh-password": { text: N_("Prefira entrar no SSH com chave em vez de senha (PasswordAuthentication no)."), go: ["services"], label: N_("Abrir Serviços") },
  "security.exposed-port": { text: N_("Feche a porta se não usa esse serviço a partir de outros computadores."), go: ["network", "firewall"], label: N_("Abrir Firewall") },
  "security.empty-password": { text: N_("Defina uma senha para essas contas, ou remova-as."), go: ["users"], label: N_("Abrir Usuários") },
  "security.sudo-open": { text: N_("Só administradores (grupo wheel) devem usar sudo. Revise as contas."), go: ["users"], label: N_("Abrir Usuários") },
  "security.disk-unencrypted": { text: N_("A criptografia do disco se escolhe na instalação; para ativá-la, faça backup e reinstale com a opção de criptografar.") },
  "security.selinux": { text: N_("O SELinux reforça o isolamento dos serviços. A política do Lyra para ele ainda está em definição.") },
  "repair.restart-needed": { text: N_("Reinicie quando puder para concluir a atualização.") },
  "repair.failed": RETRY, "repair.unit-failed": { text: N_("O serviço continua falhando. Veja o log dele para entender o motivo."), go: ["logs"], label: N_("Abrir Logs") },
};

// Informações que também merecem uma orientação.
const INFO_GUIDE = new Set(["platform.firmware-updates", "backup.none", "security.disk-unencrypted", "security.selinux", "repair.restart-needed"]);

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
  const guide = finding.severity !== "info" || INFO_GUIDE.has(finding.code) ? GUIDE[finding.code] : null;
  const index = (stage.findings ?? []).indexOf(finding);
  // Com diagnostics-actions-v1, o conserto vale só para este achado.
  const fix = finding.remedy && actions.repairFinding && index >= 0
    ? button(t("Corrigir"), { kind: "primary", size: "sm", icon: "zap", onClick: () => actions.repairFinding(stage.id, index, finding) })
    : finding.remedy && actions.repair
      ? button(t("Resolver"), { kind: "primary", size: "sm", icon: "zap", onClick: () => actions.repair([{ id: finding.remedy, args: finding.remedyArgs ?? [] }]) })
      : null;
  const seen = stage.id === "logs" && finding.severity !== "info" && actions.markSeen
    ? button(t("Já vi"), { size: "sm", icon: "check", title: t("Os registros até agora deixam de contar"), onClick: () => actions.markSeen(stage) })
    : null;
  const firmware = finding.code === "platform.firmware-updates" && actions.updateFirmware
    ? button(t("Atualizar"), { kind: "primary", size: "sm", icon: "download", onClick: () => actions.updateFirmware() })
    : null;
  const shortcut = firmware || (!fix && guide?.go && actions.go && button(t(guide.label), { size: "sm", icon: "chevron-right", onClick: () => actions.go(...guide.go) }));
  return h("div", { class: "diag-finding" },
    row({
      icon: finding.severity === "info" ? "info" : "alert",
      tint: tone,
      title: finding.text,
      subtitle: stageName(stage.id),
      wrap: true,
      suffix: (fix || shortcut || seen) ? h("span", { class: "row-flex" }, seen, fix || shortcut) : null,
    }),
    finding.remedy && h("p", { class: "diag-guide small" }, icon("zap", 14), remedyText(finding.remedy, finding.remedyArgs)),
    !finding.remedy && guide && h("p", { class: "diag-guide small" }, icon("info", 14), t(guide.text)),
    raw && h("details", { class: "diag-raw" }, h("summary", {}, t("Dados brutos")), h("pre", { class: "mono selectable" }, raw), h("div", { class: "row-flex" }, copy)));
}

const BOOT_PHASES = [
  ["firmware", N_("Firmware")],
  ["loader", N_("Carregador")],
  ["kernel", N_("Kernel")],
  ["initrd", N_("initrd")],
  ["userspace", N_("Sistema")],
];

/** Barra das fases do último boot e os serviços mais lentos. */
function bootView(stage) {
  const phases = stage.metrics?.phases;
  if (!phases) return null;
  const present = BOOT_PHASES.filter(([id]) => phases[id] > 0);
  const total = present.reduce((sum, [id]) => sum + phases[id], 0) || 1;
  const secs = (value) => t("{seconds} s", { seconds: value.toLocaleString(locale, { maximumFractionDigits: 1 }) });
  const slowest = stage.metrics.slowest ?? [];
  return h("div", { class: "card pad diag-boot" },
    h("h4", {}, icon("power", 16), t("Último boot"), h("span", { class: "n" }, secs(stage.metrics.totalSeconds ?? total))),
    h("div", { class: "diag-boot-bar" }, present.map(([id], i) =>
      h("span", { class: `seg s${i}`, style: { flex: `${phases[id]} 0 0` }, title: `${t(BOOT_PHASES.find((p) => p[0] === id)[1])}: ${secs(phases[id])}` }))),
    h("div", { class: "diag-boot-legend small" }, present.map(([id, label], i) =>
      h("span", {}, h("i", { class: `dot s${i}` }), `${t(label)} ${secs(phases[id])}`))),
    slowest.length > 0 && h("div", { class: "diag-boot-slow" },
      h("div", { class: "small muted" }, t("O que mais demorou antes do login")),
      slowest.map((unit) => h("div", { class: "row-flex small" }, h("span", { class: "mono spacer selectable" }, unit.unit), h("span", { class: "muted" }, secs(unit.seconds))))));
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
    bootView((report.stages ?? []).filter((stage) => stage.id === "boot").pop() ?? {}),
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

// ---------- Pedir ajuda ----------

const HELP_REPO = "https://github.com/lyra-os-linux/lyraos-desktop";
// Acima disso o GitHub recusa a URL; o texto inteiro vai para a área de
// transferência e a página recebe uma versão encurtada.
const URL_LIMIT = 7800;

function helpUrl(kind, title, body) {
  const base = kind === "issue"
    ? `${HELP_REPO}/issues/new?template=help.md&title=`
    : `${HELP_REPO}/discussions/new?category=q-a&title=`;
  const note = `\n\n${t("(Texto encurtado; o completo está na área de transferência.)")}`;
  let text = body;
  let url = `${base}${encodeURIComponent(title)}&body=${encodeURIComponent(text)}`;
  while (url.length > URL_LIMIT && text.length > 200) {
    text = cut(body, Math.floor(text.length * 0.85)) + note;
    url = `${base}${encodeURIComponent(title)}&body=${encodeURIComponent(text)}`;
  }
  return url;
}

/** Painel do "Pedir ajuda": sempre com o relatório redigido, editável. */
async function openHelp(report) {
  let vega = "";
  try {
    vega = (await call("app_info")).version;
  } catch {
    // Sem a versão do Vega, o texto segue com as outras.
  }
  const opts = { t, vega, stageName: (id) => stageName(id), verdictLabel: (verdict) => t(VERDICTS[verdict]?.label ?? verdict) };
  const description = h("textarea", { class: "input help-description", rows: 4, placeholder: t("Ex.: desde ontem o Wi-Fi cai a cada poucos minutos") });
  const preview = h("textarea", { class: "input mono help-preview", rows: 14, spellcheck: "false" });
  const counter = h("span", { class: "small muted" });
  let edited = false;
  const refresh = () => {
    if (!edited) preview.value = buildHelpText(report, { ...opts, description: description.value });
    counter.textContent = t("{count} caracteres", { count: preview.value.length });
  };
  description.addEventListener("input", refresh);
  preview.addEventListener("input", () => {
    edited = true;
    counter.textContent = t("{count} caracteres", { count: preview.value.length });
  });
  refresh();
  const title = () => {
    const first = description.value.trim().split("\n")[0];
    return cut(first || t("Ajuda com o diagnóstico do Lyra OS"), 80);
  };
  const copy = async () => {
    await navigator.clipboard?.writeText(preview.value);
    toast(t("Copiado"));
  };
  const open = async (kind) => {
    // O texto completo também vai para a área de transferência.
    await navigator.clipboard?.writeText(preview.value).catch(() => {});
    try {
      await call("open_uri", { uri: helpUrl(kind, title(), preview.value) });
    } catch (e) {
      toast(e.message, { ic: "alert-circle" });
    }
  };
  sheet(t("Pedir ajuda"), h("div", { class: "help-panel" },
    h("label", { class: "small muted" }, t("O que está acontecendo?")),
    description,
    h("div", { class: "row-flex" }, h("label", { class: "small muted spacer" }, t("O que será compartilhado (pode editar)")), counter),
    preview,
    h("p", { class: "small muted" }, icon("lock", 14), " ", t("Número de série, endereços MAC, nome da rede, nome do computador e das contas já foram trocados por [redacted]. Nada sai daqui sem você clicar.")),
    h("div", { class: "row-flex help-actions" },
      button(t("Copiar"), { icon: "copy", onClick: copy }),
      h("span", { class: "spacer" }),
      button(t("Abrir uma issue no GitHub"), { icon: "external", onClick: () => open("issue") }),
      button(t("Abrir no fórum"), { kind: "primary", icon: "external", onClick: () => open("forum") }))));
}

// ---------- Cartão ----------

export function diagnosticsCard(ctx) {
  const root = h("div", { class: "card diag-card", id: "sec-diagnostics" });
  let tests = [];
  let last = loadLast();
  let mode = "quick";
  let running = null; // { job, tests, gpu, stage, percent, message, sample, error }
  let error = null;
  // Achados com conserto por teste (diagnostics-actions-v1): id → [{ job, finding_index, remedy }].
  let remedies = new Map();
  // "Já vi" num relatório antigo: roda os logs, marca e roda de novo.
  let pendingSeen = false;
  const actionsOn = () => hasCapability("diagnostics-actions-v1");
  const jobOf = (id) => last.stages[id]?.job || last.job;
  const needsFix = (id) => ["attention", "fail"].includes(last.stages[id]?.verdict);

  async function loadRemedies() {
    if (!actionsOn()) return;
    const jobs = new Set(Object.keys(last.stages).filter(needsFix).map(jobOf).filter(Boolean));
    const found = new Map();
    for (const job of jobs) {
      try {
        for (const item of await call("diagnostics_remedies", { job })) {
          if (jobOf(item.stage) !== job) continue;
          if (!found.has(item.stage)) found.set(item.stage, []);
          found.get(item.stage).push({ job, finding_index: item.finding_index, remedy: item.remedy });
        }
      } catch {
        // Relatório apagado ou vegad antigo: o item fica com "Ver o que fazer".
      }
    }
    remedies = found;
    if (!running) paint();
  }

  /** Corrigir de um teste inteiro, a partir da lista. */
  async function fixStage(id) {
    const items = remedies.get(id) ?? [];
    if (!items.length) return;
    const ids = [...new Set(items.map((item) => item.remedy))];
    await repair(items[0].job, ids.map((remedy) => ({ id: remedy, args: [] })), items.map((item) => [id, item.finding_index]));
  }

  /** "Já vi" nos logs: os registros até agora deixam de contar. */
  async function markSeen(job) {
    const ok = await confirm({
      title: t("Marcar os logs como vistos?"),
      text: t("Os erros, falhas de programas e serviços com falha registrados até agora deixam de contar. O que acontecer depois volta a aparecer."),
      ic: "check",
      confirmLabel: t("Já vi"),
      required: false,
    });
    if (!ok) return false;
    try {
      await call("diagnostics_mark_logs_seen", { job });
    } catch (e) {
      // Só o relatório de um vegad anterior, sem o ponto de corte, segue em
      // frente (lê os logs agora e marca depois). Senha errada ou cancelada
      // para aqui, com o erro na tela.
      if (!/etapa logs/.test(e.message)) {
        toast(e.message, { ic: "alert-circle" });
        return false;
      }
      pendingSeen = true;
    }
    start(["logs"], { gpu: false });
    return true;
  }

  // Eventos que chegam antes de o Vega saber o id do job: um teste rápido
  // (logs com "Já vi") pode terminar antes de a chamada responder.
  const early = [];
  function adopt(job) {
    const mine = early.splice(0).filter((event) => event.job === job);
    for (const event of mine) if (running?.job === job) handle(event);
  }

  function handle(event) {
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
        last.stages[event.stage] = { verdict: event.verdict, summary: event.summary, at: Date.now(), job: event.job };
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
  }

  const unlisten = [
    on("diagnostics-event", (event) => {
      if (!running || event.job !== running.job) {
        early.push(event);
        if (early.length > 200) early.shift();
        return;
      }
      handle(event);
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
    loadRemedies();
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
      adopt(running.job);
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
      toast(event.result === "ok" ? t("Problemas resolvidos") : t("Correção concluída; veja o relatório"), { action: t("Ver relatório"), onAction: () => openReport() });
      loadRemedies();
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
    if (pendingSeen && last.stages.logs?.job) {
      pendingSeen = false;
      call("diagnostics_mark_logs_seen", { job: last.stages.logs.job })
        .then(() => start(["logs"], { gpu: false }))
        .catch((e) => toast(e.message, { ic: "alert-circle" }));
      return;
    }
    loadRemedies();
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

  async function fetchReport(redact, job = last.job) {
    const raw = await call("diagnostics_report", { job: job || 0, redact });
    const report = JSON.parse(raw);
    return mergeReport(report, last.gpu?.job === report.jobId ? last.gpu : null);
  }

  async function openReport(job = last.job) {
    try {
      const report = await fetchReport(false, job);
      const includeSensitive = h("input", { type: "checkbox" });
      const exportAs = async (format) => {
        try {
          const data = includeSensitive.checked ? report : await fetchReport(true, job);
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
        repairFinding: actionsOn() && !running ? async (stageId, index, finding) => {
          if (await repair(report.jobId, [{ id: finding.remedy, args: finding.remedyArgs ?? [] }], [[stageId, index]])) close?.();
        } : null,
        updateFirmware: async () => {
          close?.();
          const { done } = await runSoftware({ kind: "update-firmware" }, t("Atualizar firmware"));
          const result = await done;
          toast(result.message || (result.success ? t("Firmware atualizado.") : t("A atualização de firmware falhou.")), { ic: result.success ? "check-circle" : "alert-circle" });
        },
        markSeen: actionsOn() && !running ? async () => {
          if (await markSeen(report.jobId)) close?.();
        } : null,
      };
      close = sheet(t("Relatório do diagnóstico"), h("div", {},
        reportView(report, actions),
        h("div", { class: "card pad diag-export" },
          h("label", { class: "row-flex small" }, includeSensitive, t("Incluir número de série, endereço MAC e nome do computador")),
          h("div", { class: "row-flex" },
            button(t("Pedir ajuda"), { kind: "primary", icon: "users", onClick: async () => {
              try {
                const redacted = await fetchReport(true, job);
                close?.();
                openHelp(redacted);
              } catch (e) {
                toast(e.message, { ic: "alert-circle" });
              }
            } }),
            button(t("Exportar JSON"), { icon: "download", onClick: () => exportAs("json") }),
            button(t("Exportar texto (Markdown)"), { icon: "download", onClick: () => exportAs("md") })))));
    } catch (e) {
      toast(e.message, { ic: "alert-circle" });
    }
  }

  /** Confirma e pede ao vegad os consertos; devolve true se começou. */
  async function repair(job, remedies, findings = null) {
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
      const id = findings
        ? await call("diagnostics_repair_findings", { job, findings })
        : await call("diagnostics_repair", { job, remedies: remedies.map((remedy) => remedy.id) });
      running = { job: id, tests: [], gpu: false, repair: true, stage: `repair:${remedies[0].id}`, percent: 0, message: t("Iniciando…") };
      paint();
      adopt(id);
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
      const broken = actionsOn() && needsFix(test.id) && test.id !== "gpu";
      const fixes = broken && remedies.get(test.id);
      suffix = h("span", { class: "row-flex" },
        result && verdictBadge(result.verdict),
        broken && test.id === "logs" && button(t("Já vi"), { size: "sm", icon: "check", disabled: Boolean(running), onClick: () => markSeen(jobOf("logs")) }),
        broken && (fixes?.length
          ? button(t("Corrigir"), { size: "sm", kind: "primary", icon: "zap", disabled: Boolean(running), onClick: () => fixStage(test.id) })
          : button(t("Ver o que fazer"), { size: "sm", icon: "info", disabled: Boolean(running), onClick: () => openReport(jobOf(test.id)) })),
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
      !running && last.job > 0 && button(t("Ver relatório"), { icon: "logs", onClick: () => openReport() }));
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
      // Testes que este vegad não tem (o `network` antes do diagnostics-v4) ficam de fora.
      list(TESTS.filter((test) => test.id === "gpu" || !tests.length || tests.some((item) => item.id === test.id)).map(testRow)));
  }

  load();
  return root;
}

export const diagnosticsSection = { id: "diagnostics", label: N_("Diagnóstico"), icon: "activity", keywords: ["diagnostico", "saude", "teste", "smart", "memoria", "stress", "disco", "temperatura"] };
