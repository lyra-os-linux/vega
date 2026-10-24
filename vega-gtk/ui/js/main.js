import { call, on, personalization, win } from "./api.js";
import { svg } from "./icons.js";
import { loadCatalog, t } from "./i18n.js";
import { pages, groups } from "./pages/index.js";
import { badges, loadMetadata, loadPrefs, local, onBadges, prefs, saveLocal, savePrefs } from "./store.js";
import { mountTasksButton } from "./tasks.js";
import { startHealth } from "./health.js";
import { h, icon, clear, overlay, popover, menuItem, sheet, segmented, normalize, toggle, row, list, fail } from "./ui.js";

const $ = (id) => document.getElementById(id);
const byId = new Map(pages.map((p) => [p.id, p]));
let appInfo = { version: "", start: "dashboard" };

// ---------- Tema e cor de destaque (seguem o GNOME) ----------

const ACCENTS = {
  blue: "#3584e4", teal: "#2190a4", green: "#3a944a", yellow: "#c88800", orange: "#ed5b00",
  red: "#e62d42", pink: "#d56199", purple: "#9141ac", slate: "#6f8396",
};

function applyInterface({ color_scheme, accent } = {}) {
  const dark = color_scheme === "prefer-dark";
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  document.documentElement.dataset.density = local.density;
  let color = ACCENTS[accent] ?? ACCENTS.blue;
  if (dark) color = `color-mix(in srgb, ${color} 78%, #fff)`;
  document.documentElement.style.setProperty("--accent", color);
}

// ---------- Navegação ----------

let current = null;

function navItem(page) {
  const count = badges[page.id];
  return h(
    "button",
    { class: "nav-item", "aria-current": current?.page.id === page.id ? "page" : null, onclick: () => navigate(page.id), dataset: { page: page.id } },
    icon(page.icon, 18),
    h("span", {}, t(page.title)),
    count?.value ? h("span", { class: `count ${count.tone ?? ""}`.trim() }, count.value) : null,
  );
}

function renderNav() {
  clear(
    $("nav"),
    groups.map((g) => h("div", { class: "nav-group" }, g.title && h("div", { class: "nav-group-title" }, t(g.title)), pages.filter((p) => p.group === g.id).map(navItem))),
  );
}
onBadges(renderNav);

export function navigate(id, sub) {
  const hash = `#/${id}${sub ? `/${encodeURIComponent(sub)}` : ""}`;
  if (location.hash === hash) route();
  else location.hash = hash;
}

function route() {
  const [, id = prefs.start_page, sub] = location.hash.split("/");
  const page = byId.get(id) ?? byId.get("dashboard");
  current?.cleanups.forEach((fn) => fn());

  const cleanups = [];
  current = { page, cleanups };
  const ctx = {
    sub: sub ? decodeURIComponent(sub) : undefined,
    navigate,
    cleanup: (fn) => cleanups.push(fn),
    setCrumb: (text) => ($("page-crumb").textContent = text ?? ""),
    /** true enquanto esta página estiver aberta e a janela visível. */
    visible: () => current?.cleanups === cleanups && !document.hidden,
  };
  $("page-title").textContent = t(page.title);
  $("page-crumb").textContent = "";
  document.title = `${t(page.title)} — Vega`;

  const host = $("page");
  host.classList.remove("enter");
  let view;
  try {
    view = page.render(ctx);
  } catch (error) {
    console.error(error);
    view = { el: h("div", { class: "empty" }, h("h3", {}, t("Não foi possível abrir esta página")), h("p", {}, String(error))) };
  }
  clear(host, view.el);
  clear($("header-actions"), view.actions ?? []);
  void host.offsetWidth;
  host.classList.add("enter");
  $("scroller").scrollTop = 0;
  renderNav();
  closeSidebar();
}
window.addEventListener("hashchange", route);

// ---------- Barra lateral adaptativa ----------

function openSidebar() {
  $("sidebar").classList.add("open");
  const scrim = h("div", { class: "sidebar-scrim", onclick: closeSidebar });
  scrim.id = "sidebar-scrim";
  document.body.append(scrim);
  $("sidebar").querySelector("[aria-current=page]")?.focus();
}
function closeSidebar() {
  $("sidebar").classList.remove("open");
  $("sidebar-scrim")?.remove();
}

// ---------- Busca (páginas, seções e ações) ----------

function paletteEntries() {
  const entries = [];
  for (const page of pages) {
    entries.push({ section: t("Páginas"), icon: page.icon, label: t(page.title), where: "", keys: [t(page.title), page.title, ...(page.keywords ?? [])], run: () => navigate(page.id) });
    for (const sec of page.sections ?? []) {
      entries.push({ section: t("Configurações"), icon: sec.icon ?? page.icon, label: t(sec.label), where: t(page.title), keys: [t(sec.label), sec.label, ...(sec.keywords ?? [])], run: () => navigate(page.id, sec.id) });
    }
  }
  entries.push(
    { section: t("Ações"), icon: "cog", label: t("Configurações"), keys: [t("Configurações"), "preferencias"], run: openPreferences },
    { section: t("Ações"), icon: "keyboard", label: t("Atalhos de teclado"), keys: [t("Atalhos de teclado")], run: openShortcuts },
    { section: t("Ações"), icon: "star", label: t("Sobre o Vega"), keys: [t("Sobre o Vega")], run: openAbout },
  );
  return entries;
}

function openPalette(initial = "") {
  const input = h("input", { placeholder: t("Buscar configuração…"), "aria-label": t("Buscar"), role: "combobox", "aria-expanded": "true", "aria-controls": "palette-list" });
  input.value = initial;
  const listEl = h("div", { class: "palette-list", id: "palette-list", role: "listbox" });
  const all = paletteEntries();
  let results = [];
  let selected = 0;

  const score = (entry, q) => {
    if (!q) return entry.section === t("Páginas") ? 1 : 0;
    let best = 0;
    for (const key of entry.keys) {
      const k = normalize(key);
      if (k.startsWith(q)) best = Math.max(best, 3);
      else if (k.split(/\s+/).some((w) => w.startsWith(q))) best = Math.max(best, 2);
      else if (k.includes(q)) best = Math.max(best, 1);
    }
    return best;
  };

  const render = () => {
    const q = normalize(input.value.trim());
    results = all.map((e) => ({ e, s: score(e, q) })).filter((r) => r.s > 0).sort((a, b) => b.s - a.s).map((r) => r.e);
    selected = Math.min(selected, Math.max(0, results.length - 1));
    let lastSection = null;
    clear(
      listEl,
      results.length === 0
        ? h("div", { class: "empty", style: { padding: "30px" } }, h("p", {}, t("Nada encontrado para “{query}”", { query: input.value })))
        : results.map((entry, i) => {
            const head = entry.section !== lastSection && h("div", { class: "palette-section" }, entry.section);
            lastSection = entry.section;
            return [
              head,
              h("button", { class: "palette-item", role: "option", "aria-selected": String(i === selected), onclick: () => choose(i), onmousemove: () => { if (selected !== i) { selected = i; paint(); } } },
                icon(entry.icon, 17), h("span", {}, entry.label), entry.where && h("span", { class: "where" }, entry.where)),
            ];
          }),
    );
  };
  const paint = () => {
    listEl.querySelectorAll(".palette-item").forEach((el, i) => el.setAttribute("aria-selected", String(i === selected)));
    listEl.querySelectorAll(".palette-item")[selected]?.scrollIntoView({ block: "nearest" });
  };
  const choose = (i) => {
    const entry = results[i];
    if (!entry) return;
    close();
    entry.run();
  };

  input.addEventListener("input", () => { selected = 0; render(); });
  input.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown") { e.preventDefault(); selected = Math.min(results.length - 1, selected + 1); paint(); }
    else if (e.key === "ArrowUp") { e.preventDefault(); selected = Math.max(0, selected - 1); paint(); }
    else if (e.key === "Enter") { e.preventDefault(); choose(selected); }
  });

  const close = overlay(
    h("div", { class: "palette", role: "dialog", "aria-label": t("Buscar") },
      h("div", { class: "palette-input" }, icon("search", 19), input, h("kbd", {}, "Esc")),
      listEl,
      h("div", { class: "palette-foot" }, h("span", {}, h("kbd", {}, "↑↓"), " ", t("navegar")), h("span", {}, h("kbd", {}, "Enter"), " ", t("abrir")))),
    { top: true },
  );
  render();
  requestAnimationFrame(() => input.focus());
}

// ---------- Configurações, atalhos e sobre ----------

function openPreferences() {
  const select = (value, options, onChange, label) => {
    const el = h("select", { class: "input", "aria-label": label }, options.map(([v, text]) => h("option", { value: v, selected: v === value }, text)));
    el.addEventListener("change", () => onChange(el.value));
    return el;
  };
  const save = (changes) => savePrefs(changes).catch((error) => fail(error));
  const number = h("input", { class: "input", type: "number", min: "1", max: "60", step: "1", value: String(prefs.refresh_interval_minutes), "aria-label": t("Atualizar informações do painel") });
  number.style.width = "90px";
  number.addEventListener("change", () => {
    const value = Math.min(60, Math.max(1, Math.round(Number(number.value) || 5)));
    number.value = String(value);
    save({ refresh_interval_minutes: value });
  });
  const sw = (key, label) => toggle(prefs[key], (on) => save({ [key]: on }), label);

  sheet(t("Configurações"), [
    h("div", { class: "pref-group" }, h("h4", {}, t("Geral")), list(
      row({ title: t("Página inicial"), suffix: select(prefs.start_page, pages.map((p) => [p.id, t(p.title)]), (v) => save({ start_page: v }), t("Página inicial")) }),
      row({ title: t("Idioma do Vega"), subtitle: t("A alteração será aplicada ao reabrir o Vega."), wrap: true, suffix: select(prefs.language, [["system", t("Seguir o sistema")], ["pt_BR", "Português (Brasil)"], ["en_US", "English"], ["es_ES", "Español"]], (v) => save({ language: v }), t("Idioma do Vega")) }),
      row({ title: t("Atualizar informações do painel"), subtitle: t("Intervalo em minutos para atualizar os dados exibidos no Vega."), wrap: true, suffix: number }),
      row({ title: t("Confirmar ações administrativas"), subtitle: t("Confirmar ações já definidas. Formulários, propostas da IA e revisões de segurança sempre serão exibidos."), wrap: true, suffix: sw("confirm_actions", t("Confirmar ações administrativas")) }),
    )),
    h("div", { class: "pref-group" }, h("h4", {}, t("Notificações")), list(
      row({ title: t("Atualizações disponíveis"), suffix: sw("notify_updates", t("Atualizações disponíveis")) }),
      row({ title: t("Falhas em serviços"), suffix: sw("notify_service_failures", t("Falhas em serviços")) }),
      row({ title: t("Conclusão de backups"), suffix: sw("notify_backups", t("Conclusão de backups")) }),
    )),
    h("div", { class: "pref-group" }, h("h4", {}, t("Privacidade da IA")), list(
      row({ title: t("Ocultar dados sensíveis"), subtitle: t("Remove credenciais e identificadores antes do envio"), wrap: true, suffix: sw("redact_ai_data", t("Ocultar dados sensíveis")) }),
      row({ title: t("Salvar histórico local"), subtitle: t("Mantém as conversas da IA neste dispositivo"), wrap: true, suffix: sw("save_ai_history", t("Salvar histórico local")) }),
    )),
    h("div", { class: "pref-group" }, h("h4", {}, t("Densidade")), segmented(
      [{ id: "comfortable", label: t("Confortável") }, { id: "compact", label: t("Compacta") }],
      local.density,
      (id) => { local.density = id; saveLocal(); document.documentElement.dataset.density = id; },
    )),
  ], { wide: false });
}

function openShortcuts() {
  const rows = [
    [t("Buscar no Vega"), "Ctrl K"], [t("Configurações"), "Ctrl ,"],
    [t("Ir para a página 1–9"), "Alt 1…9"], [t("Atualizar página"), "F5"], [t("Atalhos de teclado"), "Ctrl ?"], [t("Fechar diálogo"), "Esc"],
  ];
  sheet(t("Atalhos de teclado"), list(rows.map(([label, keys]) => h("div", { class: "list-row" }, h("div", { class: "body" }, label), h("div", { class: "suffix" }, keys.split(" ").map((k) => h("kbd", {}, k)))))), { wide: false });
}

function openAbout() {
  const link = (label, uri) => h("button", { class: "btn flat sm", onclick: () => call("open_uri", { uri }).catch(fail) }, icon("external", 14), label);
  sheet(t("Sobre o Vega"), [
    h("div", { style: { "text-align": "center", padding: "6px 0 4px" } },
      h("div", { class: "brand-mark", style: { width: "72px", height: "72px", "border-radius": "20px", margin: "0 auto 12px" } }, icon("star", 38)),
      h("h2", { style: { "font-size": "22px", "font-weight": "800" } }, "Vega"),
      h("p", { class: "muted" }, `${t("Centro de controle")} · ${appInfo.version}`),
      h("p", { class: "small faint" }, "Lyra OS · Rodrigo Brito")),
    h("div", { class: "row-flex", style: { "justify-content": "center" } },
      link(t("Site"), "https://github.com/lyra-os-linux/vega"),
      link(t("Relatar problema"), "https://github.com/lyra-os-linux/vega/issues"),
      link("GPL-3.0", "https://www.gnu.org/licenses/gpl-3.0.html")),
  ], { wide: false });
}

function openMainMenu(anchor) {
  const pop = popover(anchor, [
    menuItem("cog", t("Configurações"), () => { pop.close(); openPreferences(); }, "Ctrl ,"),
    menuItem("keyboard", t("Atalhos de teclado"), () => { pop.close(); openShortcuts(); }, "Ctrl ?"),
    h("hr"),
    menuItem("star", t("Sobre o Vega"), () => { pop.close(); openAbout(); }),
  ], "menu");
}

// ---------- Janela (barra de título própria) ----------

function setupWindow() {
  const controls = $("window-controls");
  const make = (name, label, fn) => h("button", { class: "wc", "aria-label": label, title: label, onclick: fn }, icon(name, 13));
  const maxBtn = make("maximize", t("Maximizar"), () => win.toggleMaximize());
  controls.append(make("minus", t("Minimizar"), () => win.minimize()), maxBtn, make("x", t("Fechar"), () => win.close()));

  document.addEventListener("mousedown", (e) => {
    if (e.button !== 0 || !e.target.closest("[data-drag]") || e.target.closest("button, input, select")) return;
    if (e.detail === 2) win.toggleMaximize();
    else win.startDragging();
  });
  for (const dir of ["North", "South", "East", "West", "NorthEast", "NorthWest", "SouthEast", "SouthWest"]) {
    document.body.append(h("div", { class: "resize", dataset: { dir }, onmousedown: (e) => { e.preventDefault(); win.startResizeDragging(dir); } }));
  }
  const sync = async () => {
    const max = await win.isMaximized();
    document.body.classList.toggle("maximized", max);
    clear(maxBtn, icon(max ? "restore" : "maximize", 13));
    maxBtn.title = maxBtn.ariaLabel = max ? t("Restaurar") : t("Maximizar");
  };
  win.onResized(sync);
  sync();
}

// ---------- Inicialização ----------

function setupShell() {
  $("brand-mark").innerHTML = svg("star", 15);
  $("search-ic").innerHTML = svg("search", 15);
  $("menu-btn").innerHTML = svg("menu", 17);
  $("sidebar-toggle").innerHTML = svg("sidebar", 17);
  $("host-avatar").innerHTML = svg("monitor", 18);
  $("search-label").textContent = t("Buscar no Vega");
  $("menu-btn").title = $("menu-btn").ariaLabel = t("Menu principal");
  $("sidebar-toggle").title = $("sidebar-toggle").ariaLabel = t("Mostrar navegação");
  $("menu-btn").addEventListener("click", (e) => openMainMenu(e.currentTarget));
  $("search-trigger").addEventListener("click", () => openPalette());
  $("sidebar-toggle").addEventListener("click", openSidebar);
  $("host-chip").addEventListener("click", () => navigate("dashboard"));
  mountTasksButton($("jobs-btn"));

  $("scroller").addEventListener("scroll", (e) => $("headerbar").classList.toggle("scrolled", e.target.scrollTop > 4), { passive: true });

  document.addEventListener("keydown", (e) => {
    if (document.querySelector(".overlay")) return;
    const mod = e.ctrlKey || e.metaKey;
    if (mod && (e.key === "k" || e.key === "f")) { e.preventDefault(); openPalette(); }
    else if (mod && e.key === ",") { e.preventDefault(); openPreferences(); }
    else if (mod && e.key === "?") { e.preventDefault(); openShortcuts(); }
    else if (e.key === "F5") { e.preventDefault(); route(); }
    else if (e.altKey && /^[1-9]$/.test(e.key)) { e.preventDefault(); const p = pages[Number(e.key) - 1]; if (p) navigate(p.id); }
    else if (!mod && !e.altKey && e.key.length === 1 && /\S/.test(e.key) && !e.target.closest("input, textarea, select, [contenteditable]")) {
      // Digitar em qualquer lugar abre a busca, como no GNOME Settings.
      e.preventDefault();
      openPalette(e.key);
    }
  });

  // Pedidos de outra execução (`--virtual-machines`, notificador).
  on("open-page", (page) => {
    const [id, sub] = page.split("/");
    navigate(id, sub);
  });
}

async function loadHost() {
  try {
    const status = await call("system_status");
    $("host-name").textContent = status.distro || "Lyra OS";
    $("backend-label").textContent = t("vegad {version} conectado", { version: status.version });
    $("backend-dot").classList.remove("off");
  } catch {
    $("host-name").textContent = "Lyra OS";
    $("backend-label").textContent = t("Backend indisponível");
    $("backend-dot").classList.add("off");
  }
}

async function start() {
  window.vegaLog?.("start");
  appInfo = await call("app_info");
  await Promise.all([loadCatalog(call, appInfo.locale), loadPrefs().catch(() => {})]);
  try {
    applyInterface(await personalization("interface"));
  } catch {
    applyInterface({ color_scheme: window.matchMedia("(prefers-color-scheme: dark)").matches ? "prefer-dark" : "default" });
  }
  on("interface-changed", applyInterface);
  setupShell();
  setupWindow();
  loadHost();
  await loadMetadata();
  startHealth();

  const [id, sub] = appInfo.start.split("/");
  if (!location.hash) history.replaceState(null, "", `#/${byId.has(id) ? id : "dashboard"}${sub ? `/${sub}` : ""}`);
  route();
  requestAnimationFrame(() => call("app_ready"));
}

start().catch((error) => window.vegaLog?.(`start: ${error?.stack ?? error}`));
