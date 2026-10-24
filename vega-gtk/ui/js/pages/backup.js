// Backup: cópias de arquivos com Restic e pontos de restauração do sistema
// (Snapper), ambos pelo vegad. Paridade com BackupPage e SnapshotsPage do
// vega-gtk: mesmas ações, confirmações e textos.

import { call, on, pick } from "../api.js";
import { N_, t } from "../i18n.js";
import { prefs } from "../store.js";
import { runBackup, transactionPanel } from "../tasks.js";
import {
  h, icon, clear, button, iconButton, badge, segmented, section, banner, empty, skeletonRows,
  confirm, sheet, toast, fail, busy, bytes, dayLabel, time, dateTime, meter,
} from "../ui.js";

const FREQUENCIES = [
  ["manual", N_("Manual")],
  ["daily", N_("Diário")],
  ["weekly", N_("Semanal")],
];
const frequencyLabel = (value) => t(FREQUENCIES.find(([id]) => id === value)?.[1] ?? value);
const PATH_LIMIT = 100;

// ---------- Pontos de restauração (Snapper) ----------

function snapshotsTab(ctx) {
  const root = h("div", {}, skeletonRows(4));
  const state = { available: null, points: [], diff: new Map(), busy: false };

  const setBusy = (value) => {
    state.busy = value;
    root.querySelectorAll("[data-action]").forEach((el) => (el.disabled = value));
  };

  async function load() {
    try {
      state.available = await call("snapshots_available");
      state.points = state.available ? (await call("snapshots_list")).filter((s) => s.id !== 0) : [];
    } catch (error) {
      clear(root, empty("alert-circle", t("Pontos de Restauração"), error.message));
      return;
    }
    paint();
  }

  async function create() {
    const input = h("input", { class: "input", placeholder: t("Descrição do ponto de restauração"), "aria-label": t("Descrição do ponto de restauração") });
    input.style.width = "100%";
    input.addEventListener("keydown", (e) => { if (e.key === "Enter") input.closest(".dialog")?.querySelector(".dialog-actions .btn.primary")?.click(); });
    const ok = await confirm({ title: t("Criar ponto de restauração?"), text: t("O snapshot será criado pelo backend disponível no sistema."), ic: "history", extra: input, confirmLabel: t("Criar") });
    if (!ok) return;
    const description = input.value.trim();
    if (!description) return toast(t("Informe uma descrição para o snapshot."), { ic: "alert" });
    setBusy(true);
    try {
      await call("snapshots_create", { description });
      toast(t("Criando ponto de restauração…"));
      await load();
    } catch (error) {
      fail(error);
    } finally {
      setBusy(false);
    }
  }

  async function compare(point, host, trigger) {
    if (state.diff.has(point.id)) {
      state.diff.delete(point.id);
      return paint();
    }
    await busy(trigger, async () => {
      try {
        state.diff.set(point.id, await call("snapshots_diff", { id: point.id }));
      } catch (error) {
        state.diff.set(point.id, error);
      }
    });
    paint();
    host?.scrollIntoView({ block: "nearest" });
  }

  async function rollback(point) {
    // Revisão obrigatória: as diferenças de pacotes aparecem antes da escolha.
    const review = h("div", { class: "backup-review mono selectable" }, t("Carregando revisão obrigatória do rollback…"));
    const reviewing = call("snapshots_diff", { id: point.id }).then(
      (lines) => clear(review, lines.length ? lines.map((line) => h("div", {}, line)) : t("Nenhuma diferença de pacotes foi detectada. Arquivos do sistema ainda podem ser alterados.")),
      (error) => clear(review, error.message),
    );
    const decision = confirm({
      title: t("Aplicar snapshot #{id}?", { id: point.id }),
      text: t("Revise as diferenças. O sistema poderá precisar ser reiniciado após o rollback."),
      ic: "restart",
      tone: "warn",
      extra: review,
      confirmLabel: t("Aplicar rollback"),
      confirmKind: "danger",
      required: true,
    });
    await reviewing;
    if (!(await decision)) return;
    setBusy(true);
    try {
      toast(t("Aplicando ponto de restauração…"), { ic: "history" });
      await call("snapshots_rollback", { id: point.id });
      toast(t("Rollback aplicado. Reinicie o sistema se o backend solicitar."), { ic: "power", timeout: 10000 });
      await load();
    } catch (error) {
      fail(error);
    } finally {
      setBusy(false);
    }
  }

  async function remove(point) {
    const ok = await confirm({
      title: t("Excluir ponto de restauração?"),
      text: t("O snapshot #{id} será excluído permanentemente.", { id: point.id }),
      ic: "trash", tone: "danger", confirmLabel: t("Excluir"), confirmKind: "danger", required: false,
    });
    if (!ok) return;
    setBusy(true);
    try {
      await call("snapshots_delete", { id: point.id });
      state.diff.delete(point.id);
      await load();
    } catch (error) {
      fail(error);
    } finally {
      setBusy(false);
    }
  }

  async function clearAll() {
    let points;
    try {
      points = (await call("snapshots_list")).filter((s) => s.id !== 0);
    } catch (error) {
      return fail(error);
    }
    if (!points.length) return toast(t("Não há pontos de restauração que possam ser excluídos."), { ic: "info" });
    const ok = await confirm({
      title: t("Limpar pontos de restauração criados?"),
      text: t("Os {count} pontos que podem ser removidos serão excluídos permanentemente. O snapshot #0, quando presente, será preservado.", { count: points.length }),
      ic: "trash", tone: "danger", confirmLabel: t("Limpar pontos"), confirmKind: "danger", required: false,
    });
    if (!ok) return;
    setBusy(true);
    const label = h("div", { class: "small muted" }, t("Excluindo 0 de {total} pontos…", { total: points.length }));
    const bar = h("div", { class: "meter" }, h("i", { style: { width: "0%" } }));
    const panel = h("div", { class: "card pad backup-progress" }, h("strong", {}, t("Limpando pontos de restauração…")), label, bar);
    root.prepend(panel);
    const off = on("snapshots-clear-progress", ({ deleted, total }) => {
      label.textContent = t("Excluindo {deleted} de {total} pontos…", { deleted, total });
      bar.firstChild.style.width = `${total ? (100 * deleted) / total : 100}%`;
    });
    try {
      const count = await call("snapshots_clear");
      toast(t("{count} pontos de restauração foram excluídos.", { count }));
    } catch (error) {
      fail(error);
    } finally {
      off();
      setBusy(false);
      state.diff.clear();
      await load();
    }
  }

  async function retention(input) {
    const keep = Math.min(100, Math.max(1, Math.round(Number(input.value) || 10)));
    input.value = String(keep);
    const ok = await confirm({
      title: t("Alterar política de retenção?"),
      text: t("O sistema manterá os {keep} snapshots mais recentes e removerá imediatamente os mais antigos.", { keep }),
      ic: "history", tone: "warn", confirmLabel: t("Aplicar"), confirmKind: "danger", required: false,
    });
    if (!ok) return;
    setBusy(true);
    try {
      await call("snapshots_set_retention", { keep });
      toast(t("Retenção aplicada: mantendo os {keep} snapshots mais recentes.", { keep }));
      await load();
    } catch (error) {
      fail(error);
    } finally {
      setBusy(false);
    }
  }

  function pointCard(point) {
    const date = new Date(point.timestamp * 1000);
    const diff = state.diff.get(point.id);
    const diffHost = diff && h("div", { class: "backup-diff mono selectable" },
      diff instanceof Error ? diff.message
        : diff.length ? diff.map((line) => h("div", {}, line))
        : t("Nenhuma diferença de pacotes em relação ao estado atual."));
    const compareBtn = button(t("Comparar"), { size: "sm", kind: diff ? "" : "flat", icon: "layers" });
    compareBtn.dataset.action = "compare";
    compareBtn.addEventListener("click", () => compare(point, diffHost, compareBtn));
    const apply = button(t("Aplicar"), { size: "sm", kind: "flat", icon: "restart", onClick: () => rollback(point) });
    apply.dataset.action = "rollback";
    const del = iconButton("trash", t("Excluir"), () => remove(point), "sm");
    del.dataset.action = "delete";
    return h("div", { class: "tl-item" },
      h("div", { class: "card tl-card backup-point" },
        h("div", { class: "spacer" },
          h("div", { class: "row-flex" }, h("strong", { class: "selectable" }, point.description || t("Snapshot #{id}", { id: point.id })), point.trigger && badge(point.trigger, "outline")),
          h("div", { class: "row-flex small muted" }, time(date), point.description && h("span", { class: "num" }, `#${point.id}`))),
        h("div", { class: "backup-point-actions" }, compareBtn, apply, del)),
      diffHost);
  }

  function paint() {
    if (state.available === false) {
      return clear(root, empty("history", t("Pontos de Restauração"), t("Snapshots não são suportados neste sistema.")));
    }
    const points = [...state.points].sort((a, b) => b.timestamp - a.timestamp);
    const byDay = new Map();
    for (const point of points) {
      const key = dayLabel(point.timestamp * 1000);
      if (!byDay.has(key)) byDay.set(key, []);
      byDay.get(key).push(point);
    }
    const keep = h("input", { class: "input", type: "number", min: "1", max: "100", step: "1", value: "10", "aria-label": t("Manter os snapshots mais recentes") });
    keep.style.width = "84px";
    const applyKeep = button(t("Aplicar retenção"), { size: "sm", onClick: () => retention(keep) });
    applyKeep.dataset.action = "retention";
    const createBtn = button(t("Criar ponto"), { kind: "primary", size: "sm", icon: "plus", onClick: create });
    createBtn.dataset.action = "create";
    const clearBtn = button(t("Limpar pontos criados"), { kind: "danger-soft", size: "sm", icon: "trash", onClick: clearAll, disabled: !points.length });
    clearBtn.dataset.action = "clear";
    const newest = points[0];

    clear(root,
      h("div", { class: "grid c3" },
        h("div", { class: "card stat" }, h("div", { class: "stat-head" }, icon("history", 16), t("Pontos de Restauração")), h("div", { class: "stat-value" }, String(points.length)),
          h("div", { class: "stat-foot" }, newest ? `#${newest.id} • ${dateTime(newest.timestamp * 1000)}` : t("Nenhum ponto de restauração encontrado"))),
        h("div", { class: "card stat" }, h("div", { class: "stat-head" }, icon("clock", 16), t("Manter os snapshots mais recentes")),
          h("div", { class: "row-flex", style: { "margin-top": "auto" } }, keep, applyKeep)),
        h("div", { class: "card stat" }, h("div", { class: "stat-head" }, icon("shield-check", 16), "Snapper"),
          h("div", { class: "row-flex", style: { "margin-top": "auto" } }, createBtn, clearBtn))),
      section(t("Pontos de Restauração"), { id: "sec-snapshots" },
        points.length
          ? h("div", { class: "timeline" }, [...byDay].map(([day, items]) => [h("div", { class: "tl-day" }, day), items.map(pointCard)]))
          : empty("history", t("Nenhum ponto de restauração encontrado"), t("Selecione um ponto para comparar, aplicar ou excluir"), button(t("Criar ponto"), { kind: "primary", icon: "plus", onClick: create }))),
    );
    if (state.busy) setBusy(true);
  }

  load();
  return { el: root, create, reload: load };
}

// ---------- Backups (Restic) ----------

function backupsTab(ctx) {
  const root = h("div", {}, skeletonRows(3));
  const state = { configs: null, error: null, selected: null, snapshots: null, snapshot: null, paths: null, checked: new Set(), target: "", mode: "separate-folder", alert: null, task: null };

  async function load() {
    try {
      state.configs = await call("backup_configs");
      state.error = null;
    } catch (error) {
      state.configs = [];
      state.error = error.message;
    }
    if (state.selected && !state.configs.some((c) => c.id === state.selected.id)) select(null);
    else paint();
  }

  async function select(config) {
    state.selected = config;
    state.snapshots = config ? null : [];
    state.snapshot = null;
    state.paths = null;
    state.checked.clear();
    paint();
    if (!config) return;
    try {
      state.snapshots = await call("backup_snapshots", { config: config.id });
    } catch (error) {
      state.snapshots = error;
    }
    if (state.selected === config) paint();
  }

  async function selectSnapshot(snapshot) {
    state.snapshot = snapshot;
    state.paths = null;
    state.checked.clear();
    paint();
    try {
      state.paths = await call("backup_snapshot_paths", { config: state.selected.id, snapshot: snapshot.id });
    } catch (error) {
      state.paths = error;
    }
    if (state.snapshot === snapshot) paint();
  }

  function newConfig() {
    const id = h("input", { class: "input", placeholder: t("Identificador, por exemplo documentos") });
    const paths = h("input", { class: "input", placeholder: t("Caminhos separados por vírgula") });
    const destination = h("input", { class: "input", placeholder: t("Diretório ou repositório de destino") });
    const uuid = h("input", { class: "input", placeholder: t("UUID do volume (opcional)") });
    const frequency = h("select", { class: "input", "aria-label": t("Frequência") }, FREQUENCIES.map(([value, label]) => h("option", { value }, t(label))));
    const status = h("p", { class: "small faint" }, t("Os caminhos serão validados pelo vegad antes de serem salvos."));
    const field = (label, input) => h("label", { class: "pref-group" }, h("h4", {}, label), input);
    const createBtn = button(t("Criar"), { kind: "primary" });
    const close = sheet(t("Nova configuração de backup"), [
      field(t("Identificador"), id),
      field(t("Caminhos"), paths),
      field(t("Destino"), destination),
      field(t("UUID do destino"), uuid),
      field(t("Frequência"), frequency),
      status,
      h("div", { class: "row-flex", style: { "justify-content": "flex-end" } }, button(t("Cancelar"), { onClick: () => close() }), createBtn),
    ], { wide: false });
    createBtn.addEventListener("click", () => busy(createBtn, async () => {
      const config = {
        id: id.value.trim(),
        paths: paths.value.split(",").map((p) => p.trim()).filter(Boolean),
        destination: destination.value.trim(),
        destination_uuid: uuid.value.trim(),
        frequency: frequency.value,
      };
      if (!config.id || !config.paths.length || !config.destination) {
        status.textContent = t("Identificador, ao menos um caminho e destino são obrigatórios.");
        status.className = "small backup-error";
        return;
      }
      status.textContent = t("Criando configuração…");
      try {
        await call("backup_create_config", { config });
        close();
        await load();
      } catch (error) {
        status.textContent = error.message;
        status.className = "small backup-error";
      }
    }));
    requestAnimationFrame(() => id.focus());
  }

  async function removeConfig(config) {
    const ok = await confirm({
      title: t("Excluir configuração de backup?"),
      text: t("A configuração {id} será removida. Os dados já armazenados no destino não serão apagados automaticamente.", { id: config.id }),
      ic: "trash", tone: "danger", confirmLabel: t("Excluir"), confirmKind: "danger", required: false,
    });
    if (!ok) return;
    try {
      await call("backup_delete_config", { id: config.id });
      state.selected = null;
      await load();
    } catch (error) {
      fail(error);
    }
  }

  async function runNow(config) {
    const ok = await confirm({
      title: t("Executar backup agora?"),
      text: t("Configuração {id} para {count} caminho(s).", { id: config.id, count: config.paths.length }),
      ic: "archive", confirmLabel: t("Executar"), required: false,
    });
    if (!ok) return;
    try {
      const { task, done } = await runBackup({ kind: "run-now", config: config.id }, `${t("Backup")} ${config.id}`);
      state.task = task;
      paint();
      const result = await done;
      if (prefs.notify_backups) call("notify", { id: "backup-finished", summary: t("Backup concluído"), body: result.message }).catch(() => {});
      if (state.selected?.id === config.id) select(state.selected);
    } catch (error) {
      fail(error);
    }
  }

  async function restore() {
    const target = state.target.trim();
    const paths = [...state.checked];
    if (!target) return toast(t("Informe a pasta de destino antes de restaurar."), { ic: "alert" });
    if (!paths.length) return toast(t("Selecione ao menos um caminho para restaurar."), { ic: "alert" });
    const replacing = state.mode === "replace";
    const ok = await confirm({
      title: replacing ? t("Substituir arquivos existentes?") : t("Restaurar em pasta separada?"),
      text: replacing
        ? t("{count} item(ns) poderão substituir arquivos existentes em {target}. Esta ação pode causar perda de dados.", { count: paths.length, target })
        : t("{count} item(ns) serão restaurados em uma pasta separada dentro de {target}.", { count: paths.length, target }),
      ic: replacing ? "alert" : "folder", tone: replacing ? "danger" : "",
      confirmLabel: t("Restaurar"), confirmKind: replacing ? "danger" : "primary", required: false,
    });
    if (!ok) return;
    try {
      const { task } = await runBackup({ kind: "restore-items", snapshot: state.snapshot.id, target, mode: state.mode, paths }, t("Restauração parcial"));
      state.task = task;
      paint();
    } catch (error) {
      fail(error);
    }
  }

  function configCard(config) {
    const active = state.selected?.id === config.id;
    const subtitle = [config.destination, config.destination_uuid && `UUID ${config.destination_uuid}`, t("{count} caminho(s)", { count: config.paths.length }), frequencyLabel(config.frequency)].filter(Boolean).join(" • ");
    return h("button", { class: `list-row ${active ? "backup-selected" : ""}`.trim(), "aria-pressed": String(active), onclick: () => select(active ? null : config) },
      h("span", { class: `lead ${active ? "tint" : ""}`.trim() }, icon("archive", 18)),
      h("div", { class: "body" }, h("div", { class: "title" }, config.id), h("div", { class: "subtitle mono", title: subtitle }, subtitle)),
      h("div", { class: "suffix" }, badge(frequencyLabel(config.frequency), "outline"), icon("chevron-right", 16, "ic chev")));
  }

  function snapshotsList() {
    const snaps = state.snapshots;
    if (snaps == null) return skeletonRows(2);
    if (snaps instanceof Error) return banner("danger", "alert-circle", t("Snapshots"), snaps.message);
    if (!snaps.length) return h("div", { class: "card pad muted small" }, t("Nenhum snapshot encontrado"));
    return h("div", { class: "list" }, [...snaps].sort((a, b) => b.timestamp - a.timestamp).map((snap) => {
      const active = state.snapshot?.id === snap.id;
      return h("button", { class: `list-row ${active ? "backup-selected" : ""}`.trim(), onclick: () => selectSnapshot(snap) },
        h("span", { class: `lead ${active ? "tint" : ""}`.trim() }, icon("history", 17)),
        h("div", { class: "body" },
          h("div", { class: "title" }, t("Snapshot {timestamp}", { timestamp: dateTime(snap.timestamp * 1000) })),
          h("div", { class: "subtitle" }, t("{count} arquivo(s) • {size}", { count: snap.file_count, size: bytes(snap.size_bytes) }))),
        h("div", { class: "suffix" }, h("span", { class: "mono faint" }, snap.id.slice(0, 8))));
    }));
  }

  function restorePanel() {
    if (!state.snapshot) return h("div", { class: "card pad muted small" }, t("Selecione um snapshot para inspecionar seus caminhos"));
    const paths = state.paths;
    if (paths == null) return h("div", { class: "card pad muted small row-flex" }, h("span", { class: "spinner" }), t("Carregando caminhos…"));
    if (paths instanceof Error) return banner("danger", "alert-circle", t("Restauração parcial"), paths.message);
    if (!paths.length) return h("div", { class: "card pad muted small" }, t("O snapshot não contém caminhos restauráveis."));

    const shown = paths.slice(0, PATH_LIMIT);
    const summary = h("span", { class: "small muted" });
    const paintSummary = () => {
      summary.textContent = t("{count} caminho(s) disponível(is)", { count: paths.length })
        + (paths.length > PATH_LIMIT ? t(" • exibindo 100, mais {remaining}", { remaining: paths.length - PATH_LIMIT }) : "");
      restoreBtn.disabled = !state.checked.size || !state.target.trim();
    };
    const items = shown.map((path) => {
      const check = h("input", { type: "checkbox", class: "check" });
      check.checked = state.checked.has(path);
      check.addEventListener("change", () => { check.checked ? state.checked.add(path) : state.checked.delete(path); paintSummary(); });
      return h("label", { class: "backup-path" }, check, h("span", { class: "mono selectable" }, path));
    });
    const target = h("input", { class: "input", placeholder: t("Pasta de destino da restauração"), value: state.target, "aria-label": t("Pasta de destino da restauração") });
    target.addEventListener("input", () => { state.target = target.value; paintSummary(); });
    const browse = iconButton("folder", t("Pasta de destino da restauração"), async () => {
      const chosen = await pick({ directory: true, title: t("Pasta de destino da restauração") }).catch(() => null);
      if (chosen) { state.target = target.value = chosen; paintSummary(); }
    });
    const mode = h("select", { class: "input", "aria-label": t("Restauração parcial") },
      h("option", { value: "separate-folder", selected: state.mode === "separate-folder" }, t("Pasta separada (recomendado)")),
      h("option", { value: "replace", selected: state.mode === "replace" }, t("Substituir arquivos existentes")));
    mode.addEventListener("change", () => { state.mode = mode.value; restoreBtn.className = `btn ${state.mode === "replace" ? "danger" : "primary"}`; });
    const restoreBtn = button(t("Restaurar selecionados"), { kind: state.mode === "replace" ? "danger" : "primary", icon: "restart", onClick: restore });
    paintSummary();

    return h("div", { class: "card backup-restore" },
      h("div", { class: "backup-paths" }, items),
      h("div", { class: "backup-restore-bar" },
        summary,
        h("div", { class: "backup-target" }, target, browse),
        mode,
        restoreBtn));
  }

  function paint() {
    const configs = state.configs;
    if (configs == null) return;
    const newBtn = button(t("Nova configuração"), { kind: "primary", size: "sm", icon: "plus", onClick: newConfig });
    const selected = state.selected;
    clear(root,
      state.alert && banner("warn", "alert", t("Backup"), t("Alerta em {config} após {count} falha(s): {message}", state.alert)),
      state.task && h("div", { class: "backup-task" }, transactionPanel(state.task, { onClose: () => { state.task = null; paint(); } })),
      state.error && banner("danger", "alert-circle", t("Backups"), state.error),
      section(t("Backups"), { id: "sec-backups", actions: newBtn },
        configs.length
          ? h("div", { class: "list" }, configs.map(configCard))
          : empty("archive", t("Nenhuma configuração de backup encontrada"), null, button(t("Nova configuração"), { kind: "primary", icon: "plus", onClick: newConfig }))),
      selected && section(selected.id, {
        sub: frequencyLabel(selected.frequency),
        actions: [
          button(t("Executar agora"), { kind: "primary", size: "sm", icon: "play", onClick: () => runNow(selected), disabled: state.task?.state === "running" }),
          button(t("Excluir"), { kind: "danger-soft", size: "sm", icon: "trash", onClick: () => removeConfig(selected) }),
        ],
      },
        h("div", { class: "backup-detail" },
          h("div", {}, h("h4", { class: "backup-sub" }, t("Snapshots")), snapshotsList()),
          h("div", {}, h("h4", { class: "backup-sub" }, t("Restauração parcial")), restorePanel()))),
    );
  }

  const offAlert = on("backup-alert", (alert) => {
    state.alert = { config: alert.config, count: alert.failures, message: alert.message };
    paint();
  });
  ctx.cleanup(offAlert);
  load();
  return { el: root, reload: load };
}

export default {
  id: "backup",
  title: N_("Backup"),
  icon: "history",
  group: "main",
  keywords: ["restic", "snapper", "snapshot", "restaurar", "rollback", "copia", "backup"],
  sections: [
    { id: "backups", label: N_("Backups"), icon: "archive", keywords: ["restic", "copia"] },
    { id: "snapshots", label: N_("Pontos de Restauração"), icon: "history", keywords: ["snapper", "snapshot", "rollback"] },
    { id: "new-snapshot", label: N_("Criar ponto de restauração?"), icon: "plus", keywords: ["criar", "snapshot", "novo"] },
  ],

  render(ctx) {
    let tab = ctx.sub === "snapshots" || ctx.sub === "new-snapshot" ? "snapshots" : "backups";
    const host = h("div");
    let current = null;
    const show = (id) => {
      tab = id;
      current = id === "snapshots" ? snapshotsTab(ctx) : backupsTab(ctx);
      clear(host, current.el);
    };
    const tabs = segmented(
      [{ id: "backups", label: t("Backups"), icon: "archive" }, { id: "snapshots", label: t("Pontos de Restauração"), icon: "history" }],
      tab,
      show,
    );
    show(tab);
    if (ctx.sub === "new-snapshot") current.create();

    return {
      el: h("div", {},
        h("div", { class: "page-hero" }, h("div", {}, h("h2", {}, t("Backup"))), tabs),
        host),
      actions: [h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: () => current?.reload() }, icon("refresh", 17))],
    };
  },
};
