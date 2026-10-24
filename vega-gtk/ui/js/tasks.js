// Transações longas do vegad (software e backup) e outras tarefas com
// progresso. Ficam visíveis no botão de tarefas do cabeçalho em qualquer
// página; as páginas também acompanham as suas.

import { call, on } from "./api.js";
import { t } from "./i18n.js";
import { h, icon, clear, meter, popover, toast } from "./ui.js";

const CONSOLE_LINES = 10000;
const CONSOLE_BYTES = 4 * 1024 * 1024;

const tasks = new Map(); // chave: "software:12", "backup:3", "local:1"
const listeners = new Set();
const waiters = new Map();
let localId = 0;

export function onTasks(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

const emit = () => listeners.forEach((fn) => fn(allTasks()));
export const allTasks = () => [...tasks.values()];
export const runningTasks = () => allTasks().filter((task) => task.state === "running");

function create(key, title, kind) {
  const task = { key, title, kind, state: "running", percent: null, message: t("Iniciando…"), console: [], consoleBytes: 0, packages: {}, started: Date.now() };
  tasks.set(key, task);
  emit();
  return task;
}

function appendConsole(task, line) {
  task.console.push(line);
  task.consoleBytes += line.length;
  while (task.console.length > CONSOLE_LINES || task.consoleBytes > CONSOLE_BYTES) {
    task.consoleBytes -= task.console.shift().length;
  }
}

function finish(task, success, message) {
  task.state = success ? "done" : "failed";
  task.message = message || (success ? t("Concluído") : t("Falhou"));
  task.percent = 100;
  appendConsole(task, success ? t("concluído") : t("falhou"));
  emit();
  waiters.get(task.key)?.({ success, message: task.message, task });
  waiters.delete(task.key);
}

on("software-transaction", (event) => {
  const task = tasks.get(`software:${event.id}`);
  if (!task) return;
  switch (event.type) {
    case "progress":
      task.percent = event.percent;
      if (event.message) task.message = event.message;
      break;
    case "package":
      task.packages[event.package] = { phase: event.phase, percent: event.percent };
      appendConsole(task, `[zypper] ${event.package} ${event.phase} ${event.percent}%`);
      break;
    case "console":
      // As fontes "event" e "stdout" repetem o que já aparece no progresso.
      if (event.source !== "event" && event.source !== "stdout") appendConsole(task, `[${event.source}] ${event.line}`);
      break;
    case "key-pending":
      task.keyPending = event;
      break;
    case "finished":
      return finish(task, event.success, event.message);
  }
  emit();
});

on("backup-transaction", (event) => {
  const task = tasks.get(`backup:${event.id}`);
  if (!task) return;
  if (event.type === "progress") {
    task.percent = event.percent;
    if (event.message) task.message = event.message;
    emit();
  } else if (event.type === "finished") {
    finish(task, event.success, event.message);
  }
});

/**
 * Inicia uma transação de software e devolve { id, task, done }, onde
 * `done` resolve com { success, message, task } no fim.
 */
export async function runSoftware(action, title) {
  const id = await call("software_run", { action });
  return track(`software:${id}`, title, "software");
}

export async function runBackup(action, title) {
  const id = await call("backup_run", { action });
  return track(`backup:${id}`, title, "backup");
}

function track(key, title, kind) {
  const task = tasks.get(key) ?? create(key, title, kind);
  task.title = title;
  const done = new Promise((resolve) => {
    if (task.state !== "running") resolve({ success: task.state === "done", message: task.message, task });
    else waiters.set(key, resolve);
  });
  return { task, done };
}

/** Tarefa local (sem transação do vegad), com progresso manual. */
export function localTask(title) {
  const task = create(`local:${++localId}`, title, "local");
  return {
    task,
    progress(percent, message) {
      task.percent = percent;
      if (message) task.message = message;
      emit();
    },
    finish: (success, message) => finish(task, success, message),
  };
}

/** Painel de transação reutilizável (progresso, console e resultado). */
export function transactionPanel(task, { onClose } = {}) {
  const root = h("div", { class: "card job-card" });
  let showConsole = false;
  const paint = () => {
    const running = task.state === "running";
    const consoleEl = showConsole && h("div", { class: "job-log mono" }, task.console.slice(-400).map((line) => h("div", {}, line)));
    clear(
      root,
      h("div", { class: "job-head" },
        h("span", { class: `lead ${task.state === "failed" ? "red" : task.state === "done" ? "green" : ""}`.trim() },
          running ? h("span", { class: "spinner" }) : icon(task.state === "done" ? "check-circle" : "alert-circle", 20)),
        h("div", { class: "spacer" }, h("strong", {}, task.title), h("div", { class: "small muted selectable" }, task.message)),
        running && task.percent != null && h("span", { class: "percent" }, `${task.percent}%`),
        !running && onClose && h("button", { class: "icon-btn", "aria-label": t("Fechar"), onclick: onClose }, icon("x", 16))),
      running && h("div", { class: task.percent == null ? "meter indeterminate" : "meter" }, h("i", { style: { width: `${task.percent ?? 30}%` } })),
      h("button", { class: "btn flat sm", onclick: () => { showConsole = !showConsole; paint(); } }, icon(showConsole ? "chevron-down" : "chevron-right", 14), t("Console")),
      consoleEl,
    );
    if (consoleEl) consoleEl.scrollTop = consoleEl.scrollHeight;
  };
  paint();
  const off = onTasks(() => (document.body.contains(root) || task.state === "running" ? paint() : off()));
  return root;
}

// ---------- Botão e popover do cabeçalho ----------

export function mountTasksButton(button) {
  const paint = () => {
    const active = runningTasks();
    clear(button, active.length ? h("span", { class: "spinner" }) : icon("tasks", 17));
    button.title = active.length ? t("{count} tarefa(s) em andamento", { count: active.length }) : t("Tarefas");
    if (!active.length && tasks.size) button.append(h("i", { class: "badge-dot" }));
  };
  onTasks(paint);
  paint();

  button.addEventListener("click", () => {
    const body = h("div");
    const render = () => {
      const items = allTasks().sort((a, b) => b.started - a.started);
      clear(
        body,
        h("div", { class: "popover-head" }, t("Tarefas")),
        items.length === 0
          ? h("div", { class: "empty", style: { padding: "18px 16px 26px" } }, icon("tasks", 28, "ic faint"), h("p", { class: "small" }, t("Nenhuma tarefa nesta sessão. Atualizações, backups e instalações aparecem aqui.")))
          : items.map((task) =>
              h("div", { class: "task" },
                h("div", { class: "task-top" },
                  task.state === "running" ? h("span", { class: "spinner" }) : icon(task.state === "done" ? "check-circle" : "alert-circle", 16, "ic"),
                  task.title,
                  h("span", { class: "pct" }, task.state === "running" ? (task.percent != null ? `${task.percent}%` : "") : task.state === "done" ? t("Concluído") : t("Falhou"))),
                task.state === "running" && task.percent != null ? meter(task.percent / 100) : null,
                h("small", {}, task.message))),
      );
    };
    render();
    const pop = popover(button, body);
    const off = onTasks(() => (document.body.contains(pop) ? render() : off()));
  });
}

/** Toast ao terminar qualquer tarefa. */
onTasks((list) => {
  for (const task of list) {
    if (task.state !== "running" && !task.announced) {
      task.announced = true;
      toast(task.state === "done" ? t("{title}: concluído", { title: task.title }) : t("{title}: falhou", { title: task.title }), { ic: task.state === "done" ? "check-circle" : "alert-circle" });
    }
  }
});
