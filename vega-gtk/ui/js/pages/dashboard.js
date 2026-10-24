import { call } from "../api.js";
import { health, onHealth, refresh } from "../health.js";
import { N_, t } from "../i18n.js";
import { runningTasks, onTasks } from "../tasks.js";
import { h, icon, clear, button, sparkline, meter, bytes, rate, num, section } from "../ui.js";

// Métricas ao vivo só enquanto o Painel está aberto.
const POLL_MS = 3000;

function stat(ic, title) {
  const value = h("div", { class: "stat-value" }, "—");
  const foot = h("div", { class: "stat-foot" }, " ");
  const card = h("div", { class: "card stat" }, h("div", { class: "stat-head" }, icon(ic, 16), title), value);
  return { card, value, foot };
}

/** Cartão de resumo no estilo dos cartões do vega-gtk, com destino opcional. */
function summaryCard(ic, title, target, navigate) {
  const value = h("div", { class: "small muted selectable", style: { "margin-top": "2px" } }, t("Carregando…"));
  const lead = h("span", { class: "lead" }, icon(ic, 18));
  const body = [lead, h("div", { class: "body" }, h("div", { class: "title" }, title), value), target && icon("chevron-right", 16, "ic chev")];
  const card = target
    ? h("button", { class: "list-row", onclick: () => navigate(...target) }, body)
    : h("div", { class: "list-row" }, body);
  return { card, value, lead };
}

function tone(lead, kind) {
  lead.className = `lead ${kind}`.trim();
}

export default {
  id: "dashboard",
  title: N_("Painel"),
  icon: "grid",
  group: "main",
  keywords: ["inicio", "resumo", "saude", "status", "dashboard"],

  render(ctx) {
    const heroHost = h("div");

    const cpu = stat("cpu", t("Processador"));
    const cpuSpark = sparkline();
    cpu.card.append(cpuSpark, cpu.foot);
    const mem = stat("layers", t("Memória"));
    const memSpark = sparkline("spark alt");
    mem.card.append(memSpark, mem.foot);
    const net = stat("wifi", t("Rede"));
    const netSpark = sparkline();
    net.card.append(netSpark, net.foot);
    const disk = stat("disk", t("Disco (/)"));
    const diskMeter = h("div", { style: { "margin-top": "auto" } }, meter(0, { lg: true }));
    disk.card.append(diskMeter, disk.foot);
    disk.foot.style.marginTop = "0";

    const series = { cpu: [], mem: [], rx: [] };
    let previous = null;
    const push = (list, value) => { list.push(value); if (list.length > 60) list.shift(); };
    const sample = async () => {
      if (!ctx.visible()) return;
      try {
        const m = await call("monitor_metrics");
        push(series.cpu, m.cpu_percent);
        push(series.mem, (100 * m.mem_used) / Math.max(1, m.mem_total));
        cpu.value.replaceChildren(num(m.cpu_percent, 0), h("small", {}, "%"));
        cpu.foot.textContent = t("{count} núcleos", { count: m.cpu_per_core.length });
        cpuSpark.update(series.cpu, 100);
        mem.value.replaceChildren(bytes(m.mem_used), h("small", {}, t("de {total}", { total: bytes(m.mem_total, 0) })));
        mem.foot.textContent = m.swap_total ? t("Swap: {used} de {total}", { used: bytes(m.swap_used), total: bytes(m.swap_total) }) : t("Sem swap configurado");
        memSpark.update(series.mem, 100);
        if (previous) {
          const seconds = Math.max(0.2, (performance.now() - previous.at) / 1000);
          const rx = Math.max(0, (m.net_rx_bytes - previous.rx) / seconds);
          const tx = Math.max(0, (m.net_tx_bytes - previous.tx) / seconds);
          push(series.rx, rx);
          net.value.replaceChildren(rate(rx).replace("/s", ""), h("small", {}, "/s ↓"));
          net.foot.textContent = t("Envio {rate}", { rate: rate(tx) });
          netSpark.update(series.rx, Math.max(...series.rx, 50_000));
        } else {
          net.value.textContent = t("Calculando…");
        }
        previous = { at: performance.now(), rx: m.net_rx_bytes, tx: m.net_tx_bytes };
      } catch (error) {
        cpu.foot.textContent = error.message;
      }
    };
    sample();
    const timer = setInterval(sample, POLL_MS);
    ctx.cleanup(() => clearInterval(timer));

    // Resumo: os sete cartões do vega-gtk.
    const backend = summaryCard("server", t("Backend"));
    const system = summaryCard("monitor", t("Sistema"));
    const updates = summaryCard("download", t("Atualizações"), ["software", "updates"], ctx.navigate);
    const backup = summaryCard("archive", t("Backup"), ["backup"], ctx.navigate);
    const snapshots = summaryCard("history", t("Pontos de Restauração"), ["backup", "snapshots"], ctx.navigate);
    const services = summaryCard("cog", t("Serviços"), ["services"], ctx.navigate);

    const paintHero = () => {
      const updateTask = runningTasks().find((task) => task.kind === "software");
      const pending = health.updates?.length ?? 0;
      const struggling = health.services?.length ?? 0;
      let card;
      if (updateTask) {
        card = h("div", { class: "hero" },
          h("div", { class: "hero-ic" }, h("span", { class: "spinner lg" })),
          h("div", { class: "hero-body" }, h("div", { class: "eyebrow" }, t("Em andamento")), h("h2", {}, updateTask.title), h("p", {}, updateTask.message)),
          button(t("Acompanhar"), { kind: "pill", onClick: () => ctx.navigate("software") }));
      } else if (!health.loaded) {
        card = h("div", { class: "hero" },
          h("div", { class: "hero-ic" }, h("span", { class: "spinner lg" })),
          h("div", { class: "hero-body" }, h("div", { class: "eyebrow" }, t("Visão geral do sistema")), h("h2", {}, t("Verificando…"))));
      } else if (health.errors.status) {
        card = h("div", { class: "hero attention" },
          h("div", { class: "hero-ic" }, icon("alert", 30)),
          h("div", { class: "hero-body" }, h("div", { class: "eyebrow" }, t("Backend indisponível")), h("h2", {}, t("O vegad não respondeu")), h("p", { class: "selectable" }, health.errors.status)),
          button(t("Tentar novamente"), { kind: "pill", icon: "refresh", onClick: () => refresh() }));
      } else if (pending || struggling) {
        card = h("div", { class: `hero ${struggling ? "attention" : ""}`.trim() },
          h("div", { class: "hero-ic" }, icon(struggling ? "alert" : "download", 30)),
          h("div", { class: "hero-body" },
            h("div", { class: "eyebrow" }, t("Visão geral do sistema")),
            h("h2", {}, pending ? t("{count} pacote(s) pendente(s)", { count: pending }) : t("{count} serviço(s) requer(em) atenção", { count: struggling })),
            h("p", {}, [pending && struggling ? t("{count} serviço(s) requer(em) atenção", { count: struggling }) : "", pending ? t("Um ponto de restauração é criado antes de atualizar.") : ""].filter(Boolean).join(" · "))),
          h("div", { class: "row-flex" },
            struggling > 0 && button(t("Ver serviços"), { kind: "pill ghost", onClick: () => ctx.navigate("services") }),
            pending > 0 && button(t("Ver atualizações"), { kind: "pill", icon: "download", onClick: () => ctx.navigate("software", "updates") })));
      } else {
        card = h("div", { class: "hero" },
          h("div", { class: "hero-ic" }, icon("shield-check", 30)),
          h("div", { class: "hero-body" }, h("div", { class: "eyebrow" }, t("Visão geral do sistema")), h("h2", {}, t("Tudo em dia")), h("p", {}, t("Nenhum serviço com problema"))),
          button(t("Atualizar"), { kind: "pill ghost", icon: "refresh", onClick: () => refresh() }));
      }
      clear(heroHost, card);
    };

    const paint = () => {
      paintHero();
      const error = (key) => health.errors[key];
      const s = health.status;
      backend.value.textContent = error("status") ?? (s ? t("vegad {version} conectado • {distro}", { version: s.version, distro: s.distro }) : t("Carregando…"));
      tone(backend.lead, error("status") ? "red" : s ? "green" : "");
      system.value.textContent = error("status") ? t("Backend indisponível") : s ? t("{distro} • interface nativa ativa", { distro: s.distro }) : t("Carregando…");

      if (error("updates")) updates.value.textContent = error("updates");
      else if (health.updates == null) updates.value.textContent = t("Verificando atualizações…");
      else updates.value.textContent = health.updates.length ? t("{count} pacote(s) pendente(s)", { count: health.updates.length }) : t("Tudo em dia");
      tone(updates.lead, health.updates?.length ? "tint" : health.updates ? "green" : "");

      backup.value.textContent = error("backups") ?? (health.backups == null ? t("Carregando…") : health.backups.length ? t("{count} destino(s) configurado(s)", { count: health.backups.length }) : t("Não configurado"));
      tone(backup.lead, health.backups?.length ? "green" : "");

      const sn = health.snapshots;
      snapshots.value.textContent = error("snapshots") ?? (sn == null ? t("Carregando…") : !sn.available ? t("Não suportado neste sistema") : sn.count ? t("{count} snapshot(s)", { count: sn.count }) : t("Nenhum snapshot"));
      tone(snapshots.lead, sn?.count ? "tint" : "");

      services.value.textContent = error("services") ?? (health.services == null ? t("Carregando…") : health.services.length ? t("{count} serviço(s) habilitado(s), mas parado(s)", { count: health.services.length }) : t("Nenhum serviço com problema"));
      tone(services.lead, health.services?.length ? "red" : health.services ? "green" : "");

      const d = health.disk;
      if (error("disk")) {
        disk.value.textContent = "—";
        disk.foot.textContent = error("disk");
      } else if (d) {
        disk.value.replaceChildren(`${d.percent}%`, h("small", {}, t("em uso")));
        disk.foot.textContent = t("{percent}% • {used} de {total} usados", d);
        clear(diskMeter, meter(d.percent / 100, { lg: true }));
      }
    };
    ctx.cleanup(onHealth(paint));
    ctx.cleanup(onTasks(paintHero));
    paint();
    // Abrir o Painel atualiza os dados, como clicar em "Painel" no vega-gtk.
    refresh();

    const diskCard = h("button", { class: "card interactive stat", onclick: () => ctx.navigate("hardware") }, [...disk.card.childNodes]);

    const el = h("div", {},
      heroHost,
      section(t("Agora"), {}, h("div", { class: "grid c4" }, cpu.card, mem.card, net.card, diskCard)),
      section(t("Resumo"), {}, h("div", { class: "grid c2" },
        h("div", { class: "list" }, updates.card, backup.card, snapshots.card),
        h("div", { class: "list" }, backend.card, system.card, services.card))),
    );
    return { el, actions: [h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: () => refresh() }, icon("refresh", 17))] };
  },
};
