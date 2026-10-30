// Liberar espaço (add-space-cleanup): as categorias do sistema vêm do vegad
// (Cleanup, space-cleanup-v1); as da pasta pessoal são medidas e limpas pelo
// próprio Vega, na sessão. Nada fora destas categorias é listado ou apagado.
import { call, on } from "../api.js";
import { N_, t } from "../i18n.js";
import { hasCapability } from "../store.js";
import { refreshDisk } from "../health.js";
import { h, icon, clear, button, badge, section, banner, bytes, confirm, sheet, toast, skeletonRows } from "../ui.js";

const CATEGORIES = {
  coredumps: { icon: "alert", title: N_("Relatórios de falha (core dumps)"), text: N_("Cópias da memória de programas que travaram. Servem para depurar; podem ser apagadas.") },
  journal: { icon: "logs", title: N_("Logs antigos do sistema"), text: N_("O journal fica em até 500 MB. O boot atual e o anterior são sempre mantidos.") },
  "package-cache": { icon: "package", title: N_("Pacotes baixados"), text: N_("Cópias de pacotes que o Zypper já instalou. São baixados de novo se precisar.") },
  "flatpak-unused": { icon: "box", title: N_("Runtimes Flatpak sem uso (sistema)"), text: N_("Bibliotecas que nenhum aplicativo instalado usa mais.") },
  "old-kernels": { icon: "layers", title: N_("Kernels antigos"), text: N_("Segue a regra do openSUSE: o kernel em uso e o mais novo ficam. Um snapshot é criado antes."), explicit: true },
  snapshots: { icon: "history", title: N_("Snapshots fora da política"), text: N_("Aplica a limpeza automática do Snapper, que respeita a retenção configurada."), explicit: true },
  trash: { icon: "trash", title: N_("Lixeira"), text: N_("Arquivos que você já mandou para a lixeira.") },
  thumbnails: { icon: "grid", title: N_("Miniaturas"), text: N_("Prévias de imagens e vídeos; são recriadas quando necessário.") },
  "user-cache": { icon: "folder", title: N_("Cache dos aplicativos"), text: N_("Arquivos temporários por aplicativo, em ~/.cache. Escolha quais apagar."), explicit: true },
  "user-flatpak-unused": { icon: "box", title: N_("Runtimes Flatpak sem uso (usuário)"), text: N_("Bibliotecas da sua instalação Flatpak que nenhum aplicativo usa mais.") },
};

const SYSTEM = new Set(["coredumps", "journal", "package-cache", "flatpak-unused", "old-kernels", "snapshots"]);

const REASONS = {
  nothing: N_("Nada para limpar"),
  busy: N_("Outra operação do sistema está em andamento"),
  "not-configured": N_("O Snapper não está configurado"),
  protected: N_("Bloqueado: a remoção tiraria o kernel em uso ou o mais novo"),
  failed: N_("Não foi possível medir"),
};

function reasonText(reason) {
  if (!reason) return "";
  if (reason.startsWith("missing-tool:")) return t("Requer o pacote {package}", { package: reason.slice("missing-tool:".length) });
  return REASONS[reason] ? t(REASONS[reason]) : t("Indisponível");
}

function sizeText(category) {
  if (category.size_unknown) {
    const count = category.count ?? 0;
    return count ? t("pelo menos {count} • tamanho desconhecido", { count }) : t("tamanho desconhecido");
  }
  return bytes(category.bytes);
}

function cleanable(category) {
  return category.can_clean !== false && (category.bytes > 0 || (category.count ?? 0) > 0) && !category.reason;
}

/** Monta a seção. `refreshVolumes` atualiza o resumo do disco depois de limpar. */
export function cleanupSection(ctx, refreshVolumes) {
  const body = h("div", {}, skeletonRows(3));
  const status = h("p", { class: "small faint hw-status selectable" });
  let categories = [];
  let job = null; // { id, ids, resolve }
  let busy = false;

  const unlisten = [
    on("cleanup-event", (event) => {
      if (!job || event.job !== job.id) return;
      if (event.type === "progress") {
        status.textContent = t("Limpando: {name}…", { name: t(CATEGORIES[event.id]?.title ?? event.id) });
      } else if (event.type === "category") {
        job.freed += event.freed;
        if (event.result === "failed") job.failed.push(event.id);
      } else if (event.type === "finished") {
        const done = job;
        job = null;
        done.resolve(event);
      }
    }),
  ];
  ctx.cleanup?.(() => unlisten.forEach((fn) => fn()));

  async function runSystem(ids) {
    if (!ids.length) return { freed: 0, failed: [] };
    const state = { freed: 0, failed: [] };
    const finished = new Promise((resolve) => (state.resolve = resolve));
    state.id = await call("cleanup_start", { ids });
    state.ids = ids;
    job = state;
    const event = await finished;
    if (event.message) state.failed.push(event.message);
    return { freed: event.freed || state.freed, failed: state.failed };
  }

  async function runUser(id, entries = []) {
    status.textContent = t("Limpando: {name}…", { name: t(CATEGORIES[id].title) });
    try {
      return { freed: await call("cleanup_user_clean", { id, entries }), failed: [] };
    } catch (error) {
      return { freed: 0, failed: [`${t(CATEGORIES[id].title)}: ${error.message}`] };
    }
  }

  /** Confirma listando cada categoria e o tamanho, limpa e mede de novo. */
  async function clean(selection) {
    if (busy || !selection.length) return;
    const ok = await confirm({
      title: t("Liberar espaço?"),
      text: t("Os itens abaixo serão apagados. Não é possível desfazer."),
      ic: "trash",
      facts: selection.map(({ category, entries, bytes: size }) => ({
        icon: CATEGORIES[category.id].icon,
        text: `${t(CATEGORIES[category.id].title)}${entries?.length ? ` (${entries.join(", ")})` : ""} — ${size !== undefined ? bytes(size) : sizeText(category)}`,
        warn: category.id === "old-kernels" || category.id === "user-cache",
      })),
      confirmLabel: t("Limpar"),
      confirmKind: "danger",
    });
    if (!ok) return;
    busy = true;
    paint();
    let freed = 0;
    const failed = [];
    try {
      const system = selection.filter(({ category }) => SYSTEM.has(category.id)).map(({ category }) => category.id);
      const result = await runSystem(system);
      freed += result.freed;
      failed.push(...result.failed.map((id) => t(CATEGORIES[id]?.title ?? id)));
      for (const { category, entries } of selection.filter(({ category }) => !SYSTEM.has(category.id))) {
        const result = await runUser(category.id, entries);
        freed += result.freed;
        failed.push(...result.failed);
      }
    } catch (error) {
      failed.push(error.message);
    }
    busy = false;
    status.textContent = failed.length ? t("Falhou: {list}", { list: failed.join("; ") }) : "";
    toast(t("{size} liberados", { size: bytes(freed) }), { ic: failed.length ? "alert" : "check-circle" });
    if (selection.some(({ category }) => category.id === "old-kernels")) {
      toast(t("O espaço dos kernels volta ao disco quando os snapshots antigos forem limpos."), { ic: "info", timeout: 8000 });
    }
    await load();
    refreshVolumes?.();
    refreshDisk();
  }

  function chooseCache(category) {
    const chosen = new Set();
    const total = h("span", { class: "small muted" });
    const go = button(t("Limpar selecionados"), { kind: "danger", icon: "trash", disabled: true });
    const paintTotal = () => {
      const size = category.entries.filter((e) => chosen.has(e.name)).reduce((sum, e) => sum + e.bytes, 0);
      total.textContent = t("{count} pasta(s) • {size}", { count: chosen.size, size: bytes(size) });
      go.disabled = !chosen.size || busy;
      // "Selecionar todos" acompanha as escolhas uma a uma.
      all.checked = chosen.size > 0 && chosen.size === category.entries.length;
      all.indeterminate = chosen.size > 0 && chosen.size < category.entries.length;
    };
    const all = h("input", { type: "checkbox", class: "check" });
    const checks = category.entries.map((entry) => {
      const check = h("input", { type: "checkbox", class: "check" });
      check.addEventListener("change", () => { check.checked ? chosen.add(entry.name) : chosen.delete(entry.name); paintTotal(); });
      return check;
    });
    all.addEventListener("change", () => {
      category.entries.forEach((entry, i) => {
        checks[i].checked = all.checked;
        all.checked ? chosen.add(entry.name) : chosen.delete(entry.name);
      });
      paintTotal();
    });
    const rows = category.entries.map((entry, i) =>
      h("label", { class: "cleanup-entry" }, checks[i], h("span", { class: "mono selectable spacer" }, entry.name), h("span", { class: "small muted" }, bytes(entry.bytes))));
    const header = rows.length > 0 && h("label", { class: "cleanup-entry cleanup-all" }, all, h("strong", { class: "spacer" }, t("Selecionar todos")),
      h("span", { class: "small muted" }, bytes(category.entries.reduce((sum, e) => sum + e.bytes, 0))));
    paintTotal();
    const close = sheet(t("Cache dos aplicativos"), h("div", {},
      banner("warning", "alert", t("Feche os aplicativos antes"), t("Um aplicativo aberto pode recriar o cache ou perder dados temporários.")),
      h("div", { class: "cleanup-entries" }, header, rows.length ? rows : h("p", { class: "muted" }, t("Nenhuma pasta em ~/.cache"))),
      h("div", { class: "row-flex cleanup-sheet-actions" }, total, h("span", { class: "spacer" }), go)));
    go.onclick = () => {
      const entries = [...chosen];
      const size = category.entries.filter((e) => chosen.has(e.name)).reduce((sum, e) => sum + e.bytes, 0);
      close();
      clean([{ category, entries, bytes: size }]);
    };
  }

  function categoryRow(category) {
    const meta = CATEGORIES[category.id];
    if (!meta) return null;
    const ready = cleanable(category);
    const reason = reasonText(category.reason);
    const detail = [];
    if (category.id === "old-kernels" && category.items?.length) detail.push(category.items.join(", "));
    if (category.id === "flatpak-unused" || category.id === "user-flatpak-unused") detail.push(...(category.items ?? []).slice(0, 4));
    if (category.id === "user-cache" && category.entries?.length) {
      detail.push(category.entries.slice(0, 3).map((e) => `${e.name} — ${bytes(e.bytes)}`).join(" • "));
    }
    if (category.id === "coredumps" && category.count) detail.push(t("{count} arquivo(s)", { count: category.count }));
    const action = category.id === "user-cache"
      ? button(t("Escolher…"), { size: "sm", disabled: busy || !category.entries?.length, onClick: () => chooseCache(category) })
      : button(t("Limpar"), { size: "sm", disabled: busy || !ready, onClick: () => clean([{ category }]) });
    return h("div", { class: "list-row cleanup-row" },
      h("span", { class: `lead ${ready ? "tint" : ""}`.trim() }, icon(meta.icon, 18)),
      h("div", { class: "body" },
        h("div", { class: "title" }, t(meta.title)),
        h("div", { class: "subtitle" }, t(meta.text)),
        detail.length > 0 && h("div", { class: "small faint selectable cleanup-detail" }, detail.join(" • ")),
        reason && category.reason !== "nothing" && h("div", { class: "small muted" }, reason)),
      h("div", { class: "suffix" }, badge(ready || category.size_unknown ? sizeText(category) : t("Vazio"), ready ? "" : "outline"), action));
  }

  function paint() {
    const safe = categories.filter((c) => !CATEGORIES[c.id]?.explicit && cleanable(c));
    const total = categories.filter(cleanable).reduce((sum, c) => sum + (c.bytes || 0), 0);
    const safeTotal = safe.reduce((sum, c) => sum + (c.bytes || 0), 0);
    clear(body,
      h("div", { class: "row-flex cleanup-head" },
        h("p", { class: "small muted spacer" }, t("Recuperável: {total}. O seguro sem escolher: {safe}.", { total: bytes(total), safe: bytes(safeTotal) })),
        button(t("Limpar tudo o que for seguro"), { kind: "primary", icon: "trash", disabled: busy || !safe.length, onClick: () => clean(safe.map((category) => ({ category }))) })),
      h("div", { class: "list" }, categories.map(categoryRow)),
      status);
  }

  async function load() {
    const [system, user] = await Promise.allSettled([
      hasCapability("space-cleanup-v1") ? call("cleanup_scan") : Promise.resolve([]),
      call("cleanup_user_scan"),
    ]);
    categories = [
      ...(system.status === "fulfilled" ? system.value : []),
      ...(user.status === "fulfilled" ? user.value.filter((c) => c.available) : []),
    ];
    const problems = [system, user].filter((r) => r.status === "rejected").map((r) => r.reason?.message ?? String(r.reason));
    paint();
    if (problems.length) status.textContent = problems.join("; ");
  }

  load();
  const el = section(t("Liberar espaço"), {
    id: "cleanup",
    actions: [h("button", { class: "icon-btn", "aria-label": t("Medir de novo"), title: t("Medir de novo"), onclick: () => { if (!busy) { clear(body, skeletonRows(3)); load(); } } }, icon("refresh", 17))],
  }, body);
  if (ctx.sub === "cleanup") requestAnimationFrame(() => el.scrollIntoView({ block: "start" }));
  return el;
}
