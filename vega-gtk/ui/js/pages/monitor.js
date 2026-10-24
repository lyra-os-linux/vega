import { call } from "../api.js";
import { N_, t } from "../i18n.js";
import { h, icon, clear, sparkline, segmented, empty, confirm, fail, toast, bytes, num, normalize, skeletonRows } from "../ui.js";

const POLL_MS = 2000;
const HISTORY = 60;

const fmtBytes = (n) => bytes(n, 1);
const percent = (used, total) => (total ? (100 * used) / total : 0);

function push(series, value) {
  series.push(Math.max(0, value));
  if (series.length > HISTORY) series.shift();
}

/**
 * Pai antes dos filhos; um processo cujo pai não está na lista vira raiz e
 * ciclos aparecem no fim sem indentação (build_process_tree do vega-gtk).
 */
export function processTree(processes) {
  const present = new Set(processes.map((p) => p.pid));
  const children = new Map();
  const childIndex = new Set();
  processes.forEach((p, index) => {
    if (p.ppid !== p.pid && present.has(p.ppid)) {
      if (!children.has(p.ppid)) children.set(p.ppid, []);
      children.get(p.ppid).push(index);
      childIndex.add(index);
    }
  });
  const order = [];
  const visited = new Array(processes.length).fill(false);
  const stack = processes.map((_, i) => i).filter((i) => !childIndex.has(i)).reverse().map((i) => [i, 0]);
  while (stack.length) {
    const [index, depth] = stack.pop();
    if (visited[index]) continue;
    visited[index] = true;
    order.push([index, depth]);
    for (const kid of [...(children.get(processes[index].pid) ?? [])].reverse()) stack.push([kid, depth + 1]);
  }
  visited.forEach((seen, index) => { if (!seen) order.push([index, 0]); });
  return order;
}

function chartCard(title, cls = "") {
  const value = h("strong", { class: "mon-value" }, "—");
  const detail = h("span", { class: "small muted mon-detail" }, " ");
  const chart = sparkline(`chart ${cls}`.trim(), { width: 400, height: 110, gridLines: 3 });
  const extra = h("div");
  const card = h("div", { class: "card pad mon-card" },
    h("div", { class: "mon-head" }, h("span", { class: "mon-title" }, title), h("span", { class: "spacer" }), detail),
    value, chart, extra);
  return { card, value, detail, chart, extra };
}

/** Grade de mini-gráficos (núcleos da CPU ou GPUs). */
function miniGrid() {
  const root = h("div", { class: "mini-grid" });
  const cells = [];
  return {
    root,
    update(values, label) {
      if (cells.length !== values.length) {
        cells.length = 0;
        clear(root);
        values.forEach(() => {
          const text = h("span", { class: "mini-label" });
          const spark = sparkline("spark mini", { width: 56, height: 24 });
          const series = [];
          root.append(h("div", { class: "mini-cell" }, text, spark));
          cells.push({ text, spark, series });
        });
      }
      values.forEach((value, index) => {
        const cell = cells[index];
        cell.text.textContent = label(index, value);
        cell.text.title = cell.text.textContent;
        if (value >= 0) {
          push(cell.series, value);
          cell.spark.update(cell.series, 100);
        }
      });
    },
  };
}

export default {
  id: "monitor",
  title: N_("Monitor do Sistema"),
  icon: "activity",
  group: "system",
  keywords: ["processos", "cpu", "memoria", "gpu", "desempenho", "encerrar", "top", "uso"],
  sections: [{ id: "processes", label: N_("Processos"), icon: "tasks", keywords: ["matar", "kill", "pid"] }],

  render(ctx) {
    let tab = ctx.sub === "processes" ? "processes" : "resources";
    const status = h("span", { class: "small faint selectable" }, t("Carregando métricas…"));
    const processStatus = h("span", { class: "small faint selectable" }, t("Carregando processos…"));

    // ---------- Recursos ----------
    const cpu = chartCard(t("CPU"));
    const cores = miniGrid();
    cpu.extra.append(cores.root);
    const gpu = chartCard(t("GPU"), "alt");
    const gpus = miniGrid();
    gpu.extra.append(gpus.root);
    const memory = chartCard(t("Memória"), "alt");
    const swap = chartCard(t("Swap"));
    const disk = chartCard(t("Disco"));
    const network = chartCard(t("Rede"), "alt");
    const series = { cpu: [], gpu: [], mem: [], swap: [], disk: [], net: [] };
    let previous = null;

    function showMetrics(m) {
      push(series.cpu, m.cpu_percent);
      cpu.value.textContent = t("{percent}%", { percent: num(m.cpu_percent, 1) });
      cpu.detail.textContent = t("{count} núcleos", { count: m.cpu_per_core.length });
      cpu.chart.update(series.cpu, 100);
      cores.update(m.cpu_per_core, (i, v) => t("Núcleo {n}: {percent}%", { n: i, percent: num(v, 0) }));

      if (m.gpu_percent >= 0) {
        push(series.gpu, m.gpu_percent);
        gpu.value.textContent = t("{percent}%", { percent: num(m.gpu_percent, 1) });
        gpu.chart.update(series.gpu, 100);
      } else {
        gpu.value.textContent = t("Uso indisponível");
      }
      gpu.card.classList.toggle("mon-unavailable", m.gpu_percent < 0 && !m.gpu_per_device.some((v) => v >= 0));
      gpus.update(m.gpu_per_device, (i, v) => (v >= 0 ? t("GPU {n}: {percent}%", { n: i, percent: num(v, 0) }) : t("GPU {n}: indisponível", { n: i })));

      push(series.mem, percent(m.mem_used, m.mem_total));
      memory.value.textContent = t("{used} de {total}", { used: fmtBytes(m.mem_used), total: fmtBytes(m.mem_total) });
      memory.detail.textContent = `${num(percent(m.mem_used, m.mem_total), 0)}%`;
      memory.chart.update(series.mem, 100);

      if (m.swap_total === 0) {
        swap.value.textContent = t("Sem swap configurado");
        swap.detail.textContent = "";
      } else {
        push(series.swap, percent(m.swap_used, m.swap_total));
        swap.value.textContent = t("{used} de {total}", { used: fmtBytes(m.swap_used), total: fmtBytes(m.swap_total) });
        swap.detail.textContent = `${num(percent(m.swap_used, m.swap_total), 0)}%`;
        swap.chart.update(series.swap, 100);
      }

      const now = performance.now();
      if (previous) {
        const seconds = Math.max(0.001, (now - previous.at) / 1000);
        const perSec = (current, before) => Math.max(0, current - before) / seconds;
        const read = perSec(m.disk_read_bytes, previous.m.disk_read_bytes);
        const write = perSec(m.disk_write_bytes, previous.m.disk_write_bytes);
        const rx = perSec(m.net_rx_bytes, previous.m.net_rx_bytes);
        const tx = perSec(m.net_tx_bytes, previous.m.net_tx_bytes);
        push(series.disk, read + write);
        push(series.net, rx + tx);
        disk.value.textContent = t("{read}/s leitura • {write}/s escrita", { read: fmtBytes(read), write: fmtBytes(write) });
        network.value.textContent = t("{rx}/s recebido • {tx}/s enviado", { rx: fmtBytes(rx), tx: fmtBytes(tx) });
        // Disco e rede se ajustam ao maior valor da série (mínimo 1).
        disk.chart.update(series.disk, Math.max(1, ...series.disk));
        network.chart.update(series.net, Math.max(1, ...series.net));
      } else {
        disk.value.textContent = t("Calculando…");
        network.value.textContent = t("Calculando…");
      }
      previous = { at: now, m };
    }

    // ---------- Processos ----------
    let processes = null;
    let query = "";
    const processList = h("div", {}, skeletonRows(6));

    async function kill(p) {
      const ok = await confirm({
        title: t("Encerrar processo?"),
        text: t("{name} (PID {pid}) será encerrado com SIGTERM.", { name: p.name, pid: p.pid }),
        ic: "stop",
        tone: "danger",
        confirmLabel: t("Encerrar"),
        confirmKind: "danger",
        required: false,
      });
      if (!ok) return;
      try {
        await call("monitor_kill", { pid: p.pid });
        processStatus.textContent = t("Sinal enviado ao processo.");
        toast(t("Sinal enviado ao processo."));
      } catch (error) {
        processStatus.textContent = error.message;
        fail(error);
      }
    }

    function showProcesses() {
      if (!processes) return;
      processStatus.textContent = t("{count} processo(s)", { count: processes.length });
      const q = normalize(query.trim());
      const order = q
        ? processes.map((p, i) => [i, 0]).filter(([i]) => normalize(`${processes[i].name} ${processes[i].pid} ${processes[i].user}`).includes(q))
        : processTree(processes);
      if (!order.length) {
        clear(processList, empty(q ? "search" : "activity", q ? t("Nenhum resultado encontrado") : t("Nenhum processo listado")));
        return;
      }
      const maxCpu = Math.max(1, ...processes.map((p) => p.cpu_percent));
      clear(processList, h("div", { class: "list proc-list" }, order.slice(0, 400).map(([index, depth]) => {
        const p = processes[index];
        const indent = h("span", { class: "proc-indent" });
        indent.style.width = `${12 + 20 * depth}px`;
        const bar = h("i");
        bar.style.width = `${Math.min(100, (100 * p.cpu_percent) / maxCpu)}%`;
        return h("div", { class: `list-row proc-row ${depth ? "proc-child" : ""}`.trim() },
          indent,
          h("div", { class: "body" },
            h("div", { class: "title" }, p.name),
            h("div", { class: "subtitle mono" }, t("PID {pid} • {user} • {state}", { pid: p.pid, user: p.user, state: p.state }))),
          h("div", { class: "suffix" },
            h("span", { class: "proc-usage" },
              h("span", { class: "meter proc-meter" }, bar),
              h("span", { class: "proc-nums" }, t("{cpu}% • {mem}", { cpu: num(p.cpu_percent, 1), mem: fmtBytes(p.memory) }))),
            h("button", { class: "icon-btn sm", "aria-label": t("Encerrar processo"), title: t("Encerrar processo"), onclick: () => kill(p) }, icon("x", 15))));
      })));
    }

    // ---------- Sondagem (só com a página visível) ----------
    let inFlight = false;
    async function tick() {
      if (inFlight || !ctx.visible()) return;
      inFlight = true;
      try {
        const [metrics, list] = await Promise.allSettled([call("monitor_metrics"), call("monitor_processes")]);
        if (metrics.status === "fulfilled") {
          showMetrics(metrics.value);
          status.textContent = "";
        } else {
          status.textContent = metrics.reason.message;
        }
        if (list.status === "fulfilled") {
          processes = list.value;
          showProcesses();
        } else {
          processStatus.textContent = list.reason.message;
        }
      } finally {
        inFlight = false;
      }
    }
    tick();
    const timer = setInterval(tick, POLL_MS);
    ctx.cleanup(() => clearInterval(timer));

    const resources = h("div", { class: "mon-grid" }, cpu.card, gpu.card, memory.card, swap.card, disk.card, network.card);
    const search = h("input", { class: "input", placeholder: t("Buscar"), "aria-label": t("Buscar") });
    search.addEventListener("input", () => { query = search.value; showProcesses(); });
    const processesView = h("div", {},
      h("div", { class: "toolbar" }, h("label", { class: "input-icon" }, icon("search", 15), search), h("span", { class: "spacer" }), processStatus),
      processList);
    const host = h("div");
    const show = () => clear(host, tab === "resources" ? resources : processesView);
    const tabs = segmented([{ id: "resources", label: t("Recursos"), icon: "gauge" }, { id: "processes", label: t("Processos"), icon: "tasks" }], tab, (id) => { tab = id; show(); });
    show();

    return {
      el: h("div", {},
        h("div", { class: "page-hero" }, h("div", {}, h("h2", {}, t("Monitor do Sistema")), h("p", {}, t("Uso de recursos e processos em execução"))), tabs),
        status,
        host),
    };
  },
};
