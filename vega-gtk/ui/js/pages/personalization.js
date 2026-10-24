// Personalização: visão geral em cartões e páginas internas de tema, perfil
// do desktop, tela de bloqueio, menu e dock. Tudo é preferência da sessão
// (gsettings/Sheliak), aplicada pelo backend sem passar pelo vegad.

import { on, personalization as call } from "../api.js";
import { N_, t } from "../i18n.js";
import { registerIcons } from "../icons.js";
import { h, icon, clear, button, banner, section, toast } from "../ui.js";

registerIcons({
  type: '<path d="M4 7V5h16v2"/><path d="M9 20h6"/><path d="M12 5v15"/>',
  pointer: '<path d="M4.04 3.36a.5.5 0 0 1 .66-.66l15.4 6.29a.5.5 0 0 1-.06.95l-6.28 1.6a2 2 0 0 0-1.44 1.44l-1.6 6.28a.5.5 0 0 1-.95.06z"/>',
  image: '<rect width="18" height="18" x="3" y="3" rx="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.09-3.09a2 2 0 0 0-2.82 0L6 21"/>',
  "app-window": '<rect x="2" y="4" width="20" height="16" rx="2"/><path d="M10 4v4M2 8h20M6 4v4"/>',
  "panel-top": '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M3 9h18"/>',
  dock: '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M7 17h10"/>',
  puzzle: '<path d="M15.39 4.39a1 1 0 0 0 1.68-.47 2.5 2.5 0 1 1 3.01 3.01 1 1 0 0 0-.47 1.68l1.68 1.68a2.41 2.41 0 0 1 0 3.41l-1.61 1.61a.98.98 0 0 1-.84.28c-.47-.07-.8-.48-.97-.92a2.5 2.5 0 1 0-3.21 3.21c.45.17.86.5.92.97a.98.98 0 0 1-.28.84l-1.61 1.61a2.41 2.41 0 0 1-3.41 0L8.6 19.62a1 1 0 0 0-1.68.47 2.5 2.5 0 1 1-3.01-3.01 1 1 0 0 0 .47-1.68l-1.68-1.68a2.41 2.41 0 0 1 0-3.41L4.31 8.7a.98.98 0 0 1 .84-.28c.47.07.8.48.97.92a2.5 2.5 0 1 0 3.21-3.21c-.45-.17-.86-.5-.92-.97a.98.98 0 0 1 .28-.84l1.61-1.61a2.41 2.41 0 0 1 3.41 0z"/>',
  sliders: '<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M2 14h4M10 8h4M18 16h4"/>',
  "half-circle": '<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 1 0 18z" fill="currentColor"/>',
});

// ---------- Rótulos (mesmos msgids do vega-gtk) ----------

const ACCENT_NAMES = {
  blue: N_("Azul"), teal: N_("Verde-azulado"), green: N_("Verde"), yellow: N_("Amarelo"), orange: N_("Laranja"),
  red: N_("Vermelho"), pink: N_("Rosa"), purple: N_("Roxo"), slate: N_("Cinza"),
};
const ACCENT_COLORS = {
  blue: "#3584e4", teal: "#2190a4", green: "#3a944a", yellow: "#c88800", orange: "#ed5b00",
  red: "#e62d42", pink: "#d56199", purple: "#9141ac", slate: "#6f8396",
};
const PROFILE_NAMES = {
  lyra: N_("Lyra"), ubuntu: N_("Ubuntu"), windows10: N_("Lyra Clássico"),
  windows11: N_("Lyra Central"), macos: N_("Lyra Flutuante"), vanilla: N_("GNOME padrão"),
};
const PROFILES = [
  { id: "lyra", title: N_("Lyra"), text: N_("GNOME mais Dock e Menu do Lyra.") },
  { id: "ubuntu", title: N_("Ubuntu"), text: N_("Dock lateral estendido, aplicativos no topo e barra superior sem menus nem busca.") },
  { id: "vanilla", title: N_("Gnome Vanila"), text: N_("Usa a experiência padrão do GNOME.") },
  { id: "windows10", title: N_("Lyra Clássico"), text: N_("Painel inferior à esquerda e menu L com lista de aplicativos e blocos fixados.") },
  { id: "windows11", title: N_("Lyra Central"), text: N_("Painel inferior centralizado e menu L com pesquisa e grade de aplicativos fixados.") },
  { id: "macos", title: N_("Lyra Flutuante"), text: N_("Dock inferior flutuante e centralizado, com ampliação dos ícones e menu L na barra superior.") },
];
const COMPONENTS = [
  ["dock", N_("Lyra Dock")], ["panel", N_("Lyra Painel")], ["menus", N_("Lyra Menus")],
  ["search", N_("Lyra Busca")], ["animations", N_("Lyra Animações")],
];

const VIEWS = {
  appearance: N_("Tema"),
  profile: N_("Perfil da área de trabalho"),
  screensaver: N_("Tela de bloqueio"),
  menu: N_("Menu"),
  dock: N_("Dock"),
};

const CONTEXT_HINT = N_("Controles indisponíveis dependem do perfil, das opções ou dos componentes ativos. Os valores salvos são preservados.");

// ---------- Controles ----------

/** Linha com controle à direita; `control.setEnabled` liga/desliga a linha. */
function prefRow(title, control, { subtitle, hint } = {}) {
  const el = h("div", { class: "list-row pz-row" },
    h("div", { class: "body" }, h("div", { class: "title" }, title), subtitle && h("div", { class: "subtitle wrap" }, subtitle)),
    h("div", { class: "suffix" }, control.el));
  control.setEnabled = (enabled) => {
    control.input.disabled = !enabled;
    el.classList.toggle("pz-disabled", !enabled);
    el.title = !enabled && hint ? hint : "";
  };
  return el;
}

function switchControl(onChange, label) {
  const input = h("input", { type: "checkbox", role: "switch", "aria-label": label });
  input.addEventListener("change", () => onChange(input.checked));
  return { el: h("label", { class: "switch" }, input, h("span", { class: "track" })), input, get: () => input.checked, set: (v) => (input.checked = Boolean(v)) };
}

function selectControl(options, onChange, label) {
  const input = h("select", { class: "input pz-select", "aria-label": label }, options.map(([value, text]) => h("option", { value }, text)));
  input.addEventListener("change", () => onChange(input.value));
  return { el: input, input, get: () => input.value, set: (v) => (input.value = options.some(([o]) => o === v) ? v : options[0][0]) };
}

function numberControl({ min, max, step = 1 }, onChange, label) {
  const input = h("input", { class: "input pz-number", type: "number", min: String(min), max: String(max), step: String(step), "aria-label": label });
  const clamp = () => {
    let value = Math.round(Number(input.value));
    if (!Number.isFinite(value)) value = min;
    value = Math.min(max, Math.max(min, value));
    input.value = String(value);
    return value;
  };
  input.addEventListener("change", () => onChange(clamp()));
  return { el: input, input, get: clamp, set: (v) => (input.value = String(v)) };
}

function statusLine(text = t("Carregando configuração…")) {
  const el = h("p", { class: "pz-status small" }, text);
  el.show = (message, error = false) => {
    el.textContent = message;
    el.classList.toggle("error", error);
  };
  return el;
}

function group(title, rows, description) {
  return h("section", { class: "section" },
    h("div", { class: "section-head" }, h("h3", {}, title)),
    description && h("p", { class: "small muted pz-group-text" }, description),
    h("div", { class: "list" }, rows));
}

// ---------- Ilustrações ----------

/** Miniatura do desktop de cada perfil, desenhada com CSS. */
function profileMock(id) {
  const icons = (n) => Array.from({ length: n }, () => h("i"));
  const startMenu = (centered) => h("div", { class: `pz-start ${centered ? "centered" : ""}` },
    h("div", { class: "pz-start-search" }),
    h("div", { class: "pz-start-body" }, !centered && h("div", { class: "pz-start-list" }, icons(4)), h("div", { class: "pz-start-grid" }, icons(6))));
  return h("div", { class: `pz-mock pz-mock-${id}`, "aria-hidden": "true" },
    h("div", { class: "pz-mock-win" }),
    id !== "windows10" && id !== "windows11" && h("div", { class: "pz-mock-panel" }, id === "macos" && h("b", {}, "L"), icons(3)),
    (id === "windows10" || id === "windows11") && startMenu(id === "windows11"),
    h("div", { class: "pz-mock-dock" }, (id === "windows10" || id === "windows11") && h("b", {}, "L"), icons(4), id === "ubuntu" && h("span", { class: "pz-grow" }), id === "ubuntu" && h("b", {}, "L")));
}

function windowMock(dark) {
  return h("div", { class: `pz-window ${dark ? "dark" : "light"}`, "aria-hidden": "true" },
    h("div", { class: "pz-window-head" }, h("i"), h("i"), h("i")),
    h("div", { class: "pz-window-body" }, h("span", { class: "a" }), h("span", { class: "b" }), h("span", { class: "c" })));
}

/** Prévia do dock refletindo posição, tamanho, extensão e alinhamento. */
function dockMock(s) {
  const n = 5;
  const size = Math.round(6 + ((s.icon_size - 24) / 72) * 8);
  const dock = h("div", { class: "pz-dock-bar" }, Array.from({ length: n }, () => h("i")));
  dock.style.setProperty("--icon", `${size}px`);
  const root = h("div", { class: `pz-dockmock pos-${s.position} ${s.extend_to_edges ? "extended" : "compact"} align-${s.content_alignment}`, "aria-hidden": "true" },
    h("div", { class: "pz-dockmock-panel" }), h("div", { class: "pz-dockmock-win" }), dock);
  if (!s.extend_to_edges) root.style.setProperty("--margin", `${Math.min(12, 2 + s.edge_margin / 4)}px`);
  return root;
}

// ---------- Visão geral ----------

function overviewView(ctx) {
  const status = h("div");
  const cardsHost = h("div", {}, h("div", { class: "pz-loading" }, h("span", { class: "spinner" })));
  let generation = 0;

  const open = async (app, tweaksPage, title) => {
    const request = ++generation;
    try {
      const selected = await call("open_app", { app, tweaks_page: tweaksPage ?? null });
      // Um clique mais novo substitui este pedido.
      if (request === generation && selected !== false) clear(status);
    } catch (error) {
      if (request === generation) clear(status, banner("danger", "alert-circle", title, error.message));
    }
  };

  const card = ({ ic, title, text, value, tooltip, enabled = true, onClick, swatch, tone }) =>
    h("button", { class: `card interactive pz-card ${tone ?? ""}`.trim(), disabled: !enabled, title: tooltip ?? "", "aria-label": title, onclick: onClick },
      h("span", { class: "pz-card-ic" }, icon(ic, 22)),
      h("span", { class: "pz-card-body" },
        h("strong", {}, title),
        h("span", { class: "pz-card-text" }, text),
        h("span", { class: "pz-card-value" }, swatch && h("i", { class: "pz-swatch", style: { background: swatch } }), value)),
      icon("chevron-right", 16, "ic pz-chev"));

  const render = (o) => {
    const app = (name) => o.apps[name];
    const appTooltip = (name) => (app(name).available ? t("Abrir {name}", { name: app(name).name }) : t("Aplicativo não instalado"));
    const tweaks = (ic, title, text, page, value) => card({
      ic, title, text, enabled: app("tweaks").available, tooltip: appTooltip("tweaks"),
      value: app("tweaks").available ? value ?? appTooltip("tweaks") : t("Aplicativo não instalado"),
      onClick: () => open("tweaks", page, title),
    });
    const gnomeApp = (name, ic, title, text, value, extra = {}) => card({
      ic, title, text, enabled: app(name).available, tooltip: appTooltip(name),
      value: app(name).available ? value ?? appTooltip(name) : t("Aplicativo não instalado"),
      onClick: () => open(name, null, title), ...extra,
    });
    const page = (sub, ic, title, text, value, enabled = true, tooltip) =>
      card({ ic, title, text, value: value ?? t("Configurar"), enabled, tooltip: tooltip ?? t("Configurar"), onClick: () => ctx.navigate("screen", sub) });

    const theme = { light: t("Claro"), dark: t("Escuro"), default: t("Padrão do sistema") }[o.theme] ?? t("Configurar");
    const accent = o.accent && ACCENT_NAMES[o.accent] ? t(ACCENT_NAMES[o.accent]) : null;
    const lyraTooltip = o.sheliak ? t("Configurar") : t("Requer o perfil Lyra com Sheliak ativo");

    clear(cardsHost,
      h("div", { class: "grid c2 pz-grid" },
        page("appearance", "half-circle", t("Tema"), t("Aparência clara ou escura para a área de trabalho e os aplicativos"), theme),
        gnomeApp("background", "palette", t("Cores"), t("Cor de destaque dos botões, seleções e elementos da interface"), accent, { swatch: accent && ACCENT_COLORS[o.accent] }),
        tweaks("grid", t("Ícones"), t("Conjunto de ícones dos aplicativos e do sistema"), "appearance", o.icon_theme),
        tweaks("type", t("Fontes"), t("Família, tamanho e renderização dos textos"), "fonts", o.font_name),
        tweaks("pointer", t("Cursores"), t("Tema e tamanho do ponteiro do mouse"), "appearance", o.cursor_theme),
        tweaks("app-window", t("Janelas"), t("Botões da barra de título, foco e comportamento das janelas"), "windows"),
        gnomeApp("background", "image", t("Papel de Parede"), t("Imagem e comportamento do plano de fundo")),
        page("screensaver", "lock", t("Tela de bloqueio"), t("Bloqueio automático e tempo de inatividade da tela"))),
      section(t("Ambiente Lyra"), {}, h("div", { class: "grid c3 pz-grid" },
        page("profile", "layers", t("Perfil da área de trabalho"), t("Escolha entre Lyra, Ubuntu, Lyra Clássico, Lyra Central, Lyra Flutuante e GNOME"), t(PROFILE_NAMES[o.profile] ?? PROFILE_NAMES.vanilla)),
        page("menu", "panel-top", t("Menu"), t("Painel, relógio e menu de aplicativos do Lyra"), o.sheliak ? t("Configurar") : t("Requer o perfil Lyra com Sheliak ativo"), o.sheliak, lyraTooltip),
        page("dock", "dock", t("Dock"), t("Posição, ícones e comportamento do dock do Lyra"), o.sheliak ? t("Configurar") : t("Requer o perfil Lyra com Sheliak ativo"), o.sheliak, lyraTooltip))),
      section(t("Aplicativos do GNOME"), {}, h("div", { class: "grid c3 pz-grid" },
        gnomeApp("settings", "cog", t("Configurações do GNOME"), t("Preferências de tela, teclado, mouse e outros ajustes da sessão")),
        gnomeApp("tweaks", "sliders", t("Ajustes do GNOME"), t("Opções adicionais de aparência, fontes e janelas")),
        gnomeApp("extensions", "puzzle", t("Extensões do GNOME"), t("Gerencie as extensões e suas preferências")))),
    );
  };

  const load = () => call("overview").then(render).catch((error) => clear(cardsHost, banner("danger", "alert-circle", t("Personalização"), error.message)));
  load();
  // A cor de destaque muda nas Configurações do GNOME com a página aberta.
  ctx.cleanup(on("interface-changed", load));

  return h("div", {},
    h("div", { class: "page-hero" }, h("div", {}, h("h2", {}, t("Personalização")), h("p", {}, t("Ajuste a aparência e o comportamento do GNOME")))),
    status,
    cardsHost);
}

// ---------- Tema ----------

function appearanceView() {
  const host = h("div", { class: "pz-loading" }, h("span", { class: "spinner" }));
  const render = (o) => {
    const unavailable = o.theme == null;
    // "default" não é oferecido: o cartão mostra a aparência efetiva (claro).
    let dark = o.theme === "dark";
    const cards = [false, true].map((isDark) => {
      const el = h("button", { class: "card pz-theme", role: "radio", disabled: unavailable, "aria-checked": String(isDark === dark), onclick: () => choose(isDark) },
        h("span", { class: "pz-tick" }, icon("check", 13)),
        windowMock(isDark),
        h("strong", {}, isDark ? t("Escuro") : t("Claro")));
      el.dark = isDark;
      return el;
    });
    const choose = async (isDark) => {
      if (isDark === dark) return;
      try {
        await call("theme_set", { dark: isDark });
        dark = isDark;
        cards.forEach((c) => c.setAttribute("aria-checked", String(c.dark === dark)));
      } catch (error) {
        toast(error.message, { ic: "alert-circle" });
      }
    };
    clear(host,
      unavailable && h("p", { class: "muted" }, t("Este sistema não tem os esquemas do GNOME para aparência; as opções abaixo ficam desativadas.")),
      h("div", { class: "pz-themes", role: "radiogroup", "aria-label": t("Tema") }, cards));
    host.className = "";
  };
  call("overview").then(render).catch((error) => clear(host, banner("danger", "alert-circle", t("Tema"), error.message)));
  return host;
}

// ---------- Perfil ----------

function profileView() {
  const host = h("div", { class: "pz-loading" }, h("span", { class: "spinner" }));
  const status = statusLine("");
  let state = null;
  let applying = false;

  const render = () => {
    const s = state;
    const enabled = (id) => (id === "vanilla" ? true : id === "macos" ? s.installed && s.macos_supported : s.installed);
    const cards = PROFILES.map((p) =>
      h("button", { class: "card pz-profile", role: "radio", "aria-checked": String(p.id === s.active), "aria-label": t(p.title), "aria-description": t(p.text), disabled: !enabled(p.id) || applying, onclick: () => apply(p.id) },
        h("span", { class: "pz-tick" }, icon("check", 13)),
        profileMock(p.id),
        h("span", { class: "pz-profile-meta" }, h("strong", {}, t(p.title)), h("span", {}, t(p.text)))));

    const components = s.suite
      ? group(t("Componentes"), COMPONENTS.map(([role, title]) => {
          const control = switchControl((on) => setComponent(role, on), t(title));
          control.set(s.components?.[role] ?? false);
          const row = prefRow(t(title), control, { subtitle: role === "animations" ? t("Efeitos ao minimizar e restaurar janelas.") : null });
          control.setEnabled(Boolean(s.components) && s.editable && !applying);
          return row;
        }))
      : null;

    const desktop = switchControl((on) => setDesktopIcons(on), t("Área de trabalho ativa"));
    desktop.set(s.desktop_icons ?? false);
    const desktopRow = prefRow(t("Área de trabalho ativa"), desktop, {
      subtitle: s.desktop_icons != null ? t("Mostrar arquivos, pastas, discos e lixeira. Desativar preserva seus arquivos.") : t("Ícones na área de trabalho indisponíveis."),
    });
    desktop.setEnabled(s.desktop_icons != null);

    clear(host,
      h("div", { class: "grid c3 pz-profiles", role: "radiogroup", "aria-label": t("Perfil da área de trabalho") }, cards),
      status,
      components,
      group(t("Área de trabalho"), [desktopRow]));
    host.className = "";
  };

  const reload = async () => {
    state = await call("profile_state");
    render();
  };

  const apply = async (id) => {
    if (applying || id === state.active) return;
    applying = true;
    // Seleção otimista; em erro, volta ao perfil realmente confirmado.
    const previous = state.active;
    state = { ...state, active: id };
    render();
    host.querySelector(`.pz-profile[aria-checked="true"]`)?.classList.add("pz-busy");
    try {
      state = await call("profile_apply", { id });
      status.show("");
    } catch (error) {
      status.show(error.message, true);
      state = { ...state, active: previous };
      try { state = await call("profile_state"); } catch { /* mantém o anterior */ }
    } finally {
      applying = false;
      render();
    }
  };

  const setComponent = async (role, on) => {
    try {
      const result = await call("component_set", { role, on });
      status.show(result.error ?? "", Boolean(result.error));
      state = result.state;
    } catch (error) {
      status.show(error.message, true);
      await reload().catch(() => {});
      return;
    }
    render();
  };

  const setDesktopIcons = async (on) => {
    try {
      const result = await call("desktop_icons_set", { on });
      status.show(result.error ?? "", Boolean(result.error));
      // Mostra o estado real lido de volta.
      state = { ...state, desktop_icons: result.state };
    } catch (error) {
      status.show(error.message, true);
    }
    render();
  };

  reload().catch((error) => clear(host, banner("danger", "alert-circle", t("Perfil da área de trabalho"), error.message)));
  return host;
}

// ---------- Tela de bloqueio ----------

function screensaverView() {
  const status = statusLine();
  const lock = switchControl((on) => delay.setEnabled(on), t("Bloquear automaticamente"));
  const delay = numberControl({ min: 0, max: 3600, step: 5 }, () => {}, t("Atraso para bloquear (segundos)"));
  const idle = numberControl({ min: 0, max: 7200, step: 30 }, () => {}, t("Apagar a tela após (segundos)"));
  const apply = button(t("Aplicar"), { kind: "primary" });
  const rows = [
    prefRow(t("Bloquear automaticamente"), lock),
    prefRow(t("Atraso para bloquear (segundos)"), delay, { subtitle: t("Contado após a tela apagar. Zero bloqueia imediatamente.") }),
    prefRow(t("Apagar a tela após (segundos)"), idle, { subtitle: t("Tempo sem atividade. Zero mantém a tela ativa.") }),
  ];
  const setAvailable = (available) => {
    lock.setEnabled(available);
    delay.setEnabled(available && lock.get());
    idle.setEnabled(available);
    apply.disabled = !available;
  };
  setAvailable(false);
  apply.addEventListener("click", async () => {
    try {
      await call("screensaver_apply", { settings: { lock_enabled: lock.get(), lock_delay_secs: delay.get(), idle_delay_secs: idle.get() } });
      status.show(t("Configuração aplicada."));
    } catch (error) {
      status.show(error.message, true);
    }
  });
  call("screensaver_get").then((s) => {
    if (!s) {
      status.show(t("Bloqueio de tela não está disponível neste sistema."));
      return;
    }
    lock.set(s.lock_enabled);
    delay.set(s.lock_delay_secs);
    idle.set(s.idle_delay_secs);
    setAvailable(true);
    status.show(t("Configuração atual carregada"));
  }).catch((error) => status.show(error.message, true));

  return h("div", {},
    status,
    group(t("Bloqueio de tela"), rows, t("Essas preferências valem só para a sua sessão, como nas Configurações do GNOME.")),
    h("div", { class: "pz-actions" }, apply));
}

// ---------- Menu e painel ----------

function menuView() {
  const status = statusLine();
  let context = {};
  let suppress = true;
  // Preferência de posição preservada quando o perfil fixa os menus à esquerda.
  let savedPosition = "left";
  const c = {};
  const changed = () => {
    update();
    if (suppress) return;
    const settings = Object.fromEntries(Object.entries(c).map(([key, control]) => [key, control.get()]));
    settings.panel_menu_position = savedPosition;
    call("menu_apply", { settings }).then(() => status.show(t("Configuração aplicada."))).catch((error) => status.show(error.message, true));
  };
  const sw = (key, title) => (c[key] = switchControl(changed, title));
  const rows = {};
  const row = (key, title, control, options) => (rows[key] = prefRow(title, control, options));

  row("panel_height", t("Altura do painel"), (c.panel_height = numberControl({ min: 24, max: 64 }, changed, t("Altura do painel"))));
  row("floating_panel", t("Painel flutuante"), sw("floating_panel", t("Painel flutuante")));
  row("panel_margin", t("Margem do painel"), (c.panel_margin = numberControl({ min: 0, max: 32 }, changed, t("Margem do painel"))),
    { hint: t("Disponível com painel flutuante nos perfis compatíveis e dock não estendido. Janelas maximizadas suspendem a margem temporariamente.") });
  row("show_clock", t("Relógio"), sw("show_clock", t("Relógio")));
  row("show_panel_indicators", t("Indicadores do lado direito"), sw("show_panel_indicators", t("Indicadores do lado direito")));
  row("show_applications_menu", t("Menu Aplicativos"), sw("show_applications_menu", t("Menu Aplicativos")));
  row("show_places_menu", t("Menu Locais"), sw("show_places_menu", t("Menu Locais")));
  row("show_system_menu", t("Menu Sistema"), sw("show_system_menu", t("Menu Sistema")));
  row("show_search_menu", t("Menu Busca"), sw("show_search_menu", t("Menu Busca")));
  row("hide_workspace_button", t("Ocultar botão de áreas de trabalho"), sw("hide_workspace_button", t("Ocultar botão de áreas de trabalho")));
  const position = selectControl([["left", t("Esquerda")], ["center", t("Centro")], ["right", t("Direita")]], (value) => {
    if (!context.fixed_menu_position) savedPosition = value;
    changed();
  }, t("Posição na barra"));
  row("panel_menu_position", t("Posição na barra"), position, { hint: t("No perfil Lyra Flutuante, os menus ficam à esquerda. A preferência dos outros perfis é preservada.") });
  row("show_application_icons", t("Ícones dos aplicativos"), sw("show_application_icons", t("Ícones dos aplicativos")));
  row("sort_applications_menu", t("Ordem alfabética"), sw("sort_applications_menu", t("Ordem alfabética")));
  row("open_application_submenus_sideways", t("Submenus laterais"), sw("open_application_submenus_sideways", t("Submenus laterais")));
  row("show_place_bookmarks", t("Marcadores em Locais"), sw("show_place_bookmarks", t("Marcadores em Locais")));
  row("show_place_volumes", t("Dispositivos em Locais"), sw("show_place_volumes", t("Dispositivos em Locais")));
  row("show_system_about", t("Item Sobre no menu Sistema"), sw("show_system_about", t("Item Sobre no menu Sistema")));

  const update = () => {
    const x = context;
    const get = (key) => c[key].get();
    c.panel_height.setEnabled(x.panel && !x.fixed_panel);
    c.floating_panel.setEnabled(x.panel && !x.fixed_panel);
    c.panel_margin.setEnabled(x.panel && !x.fixed_panel && !x.extended_dock && get("floating_panel"));
    for (const key of ["show_clock", "show_panel_indicators", "hide_workspace_button"]) c[key].setEnabled(x.panel);
    for (const key of ["show_applications_menu", "show_places_menu", "show_system_menu"]) c[key].setEnabled(x.menus);
    c.show_search_menu.setEnabled(x.search);
    c.show_system_about.setEnabled(x.menus && get("show_system_menu"));
    position.setEnabled((x.menus || x.search) && !x.fixed_menu_position);
    for (const key of ["show_application_icons", "sort_applications_menu", "open_application_submenus_sideways"]) {
      c[key].setEnabled(x.menus && get("show_applications_menu") && !x.fixed_menu_position);
    }
    c.show_place_bookmarks.setEnabled(x.menus && get("show_places_menu"));
    c.show_place_volumes.setEnabled(x.menus && get("show_places_menu"));
  };
  update();

  call("menu_get").then(({ settings, context: ctxState }) => {
    context = ctxState ?? {};
    if (!settings) {
      status.show(t("A extensão Sheliak não está instalada."));
      update();
      return;
    }
    for (const [key, control] of Object.entries(c)) control.set(settings[key]);
    savedPosition = settings.panel_menu_position;
    position.set(context.fixed_menu_position ? "left" : savedPosition);
    suppress = false;
    update();
    status.show(t("Configuração atual carregada"));
  }).catch((error) => status.show(error.message, true));

  return h("div", {},
    status,
    h("p", { class: "small muted" }, t(CONTEXT_HINT)),
    group(t("Barra superior"), ["panel_height", "floating_panel", "panel_margin", "show_clock", "show_panel_indicators"].map((k) => rows[k])),
    group(t("Menus da barra superior"), ["show_applications_menu", "show_places_menu", "show_system_menu", "show_search_menu", "hide_workspace_button", "panel_menu_position"].map((k) => rows[k])),
    group(t("Conteúdo dos menus"), ["show_application_icons", "sort_applications_menu", "open_application_submenus_sideways", "show_place_bookmarks", "show_place_volumes", "show_system_about"].map((k) => rows[k])));
}

// ---------- Dock ----------

function dockView() {
  const status = statusLine();
  const previewHost = h("div", { class: "pz-dockpreview card" });
  let context = {};
  let suppress = true;
  const c = {};
  const settings = () => Object.fromEntries(Object.entries(c).map(([key, control]) => [key, control.get()]));
  const changed = () => {
    update();
    if (suppress) return;
    call("dock_apply", { settings: settings() })
      .then((next) => {
        status.show(t("Configuração aplicada."));
        if (next) {
          context = next;
          update();
        }
      })
      .catch((error) => status.show(error.message, true));
  };
  const sw = (key, title) => (c[key] = switchControl(changed, title));
  const sel = (key, options, title) => (c[key] = selectControl(options, changed, title));
  const rows = {};
  const row = (key, title, control, options) => (rows[key] = prefRow(title, control, options));

  row("position", t("Posição"), sel("position", [["bottom", t("Inferior")], ["left", t("Esquerda")], ["right", t("Direita")]], t("Posição")));
  row("icon_size", t("Tamanho dos ícones"), (c.icon_size = numberControl({ min: 24, max: 96 }, changed, t("Tamanho dos ícones"))));
  row("edge_margin", t("Margem da borda"), (c.edge_margin = numberControl({ min: 0, max: 48 }, changed, t("Margem da borda"))), { hint: t("Disponível quando o dock não está estendido.") });
  row("animation", t("Efeitos do dock"), sw("animation", t("Efeitos do dock")), { subtitle: t("Ampliação dos ícones e transições do dock.") });
  // Cada modo (compacto/estendido) guarda o próprio alinhamento: ao trocar,
  // carrega o alinhamento salvo do outro modo antes de aplicar.
  c.extend_to_edges = switchControl(async (extended) => {
    if (!suppress) {
      try {
        const alignment = await call("dock_alignment", { extended });
        if (alignment) c.content_alignment.set(alignment);
      } catch { /* mantém o alinhamento atual */ }
    }
    changed();
  }, t("Estender até as bordas"));
  row("extend_to_edges", t("Estender até as bordas"), c.extend_to_edges);
  row("content_alignment", t("Alinhamento"), sel("content_alignment", [["start", t("Início")], ["center", t("Centro")], ["end", t("Fim")]], t("Alinhamento")));
  row("hide_mode", t("Visibilidade do dock"), sel("hide_mode", [["intelligent", t("Ocultação inteligente")], ["autohide", t("Auto hide")], ["always", t("Sempre ativo")]], t("Visibilidade do dock")));
  row("hide_delay_ms", t("Atraso para ocultar (ms)"), (c.hide_delay_ms = numberControl({ min: 100, max: 3000, step: 100 }, changed, t("Atraso para ocultar (ms)"))));
  row("fullscreen_hide", t("Ocultar em tela cheia"), sw("fullscreen_hide", t("Ocultar em tela cheia")));
  row("show_running", t("Aplicativos em execução"), sw("show_running", t("Aplicativos em execução")));
  row("running_apps_position", t("Posição dos apps abertos"), sel("running_apps_position", [["start", t("Início")], ["end", t("Fim")]], t("Posição dos apps abertos")), { hint: t("Disponível quando os aplicativos em execução são exibidos.") });
  row("show_trash", t("Lixeira"), sw("show_trash", t("Lixeira")));
  row("show_apps_button", t("Botão de aplicativos"), sw("show_apps_button", t("Botão de aplicativos")));
  row("minimize_animation", t("Animação ao minimizar"), sel("minimize_animation", [["zoom", t("Zoom ao ícone")], ["fade", t("Desvanecer")], ["none", t("Sem animação")]], t("Animação ao minimizar")), { hint: t("Requer Lyra Animações ativo.") });

  const update = () => {
    const dock = Boolean(context.dock);
    for (const key of ["position", "hide_mode", "icon_size", "animation", "extend_to_edges", "content_alignment", "show_running", "show_trash", "show_apps_button", "fullscreen_hide"]) c[key].setEnabled(dock);
    c.edge_margin.setEnabled(dock && !c.extend_to_edges.get());
    c.running_apps_position.setEnabled(dock && c.show_running.get());
    c.hide_delay_ms.setEnabled(dock && c.hide_mode.get() !== "always");
    c.minimize_animation.setEnabled(Boolean(context.animations));
    if (!suppress) clear(previewHost, dockMock(settings()));
  };
  update();

  call("dock_get").then(({ settings: current, context: ctxState }) => {
    context = ctxState ?? {};
    if (!current) {
      status.show(t("A extensão Sheliak não está instalada."));
      update();
      return;
    }
    for (const [key, control] of Object.entries(c)) control.set(current[key]);
    suppress = false;
    update();
    status.show(t("Configuração atual carregada"));
  }).catch((error) => status.show(error.message, true));

  return h("div", {},
    status,
    previewHost,
    h("p", { class: "small muted" }, t(CONTEXT_HINT)),
    group(t("Aparência"), ["position", "icon_size", "edge_margin", "animation", "extend_to_edges", "content_alignment"].map((k) => rows[k])),
    group(t("Comportamento"), ["hide_mode", "hide_delay_ms", "fullscreen_hide"].map((k) => rows[k])),
    group(t("Elementos exibidos"), ["show_running", "running_apps_position", "show_trash", "show_apps_button"].map((k) => rows[k])),
    group(t("Lyra Animações"), [rows.minimize_animation], t("Efeitos ao minimizar e restaurar janelas.")));
}

// ---------- Página ----------

const DETAIL = { appearance: appearanceView, profile: profileView, screensaver: screensaverView, menu: menuView, dock: dockView };

export default {
  id: "screen",
  title: N_("Personalização"),
  icon: "palette",
  group: "system",
  keywords: ["aparencia", "tema", "escuro", "claro", "cores", "papel de parede", "fontes", "icones", "sheliak", "gnome"],
  sections: [
    { id: "appearance", label: N_("Tema"), icon: "half-circle", keywords: ["escuro", "claro", "dark", "light"] },
    { id: "profile", label: N_("Perfil da área de trabalho"), icon: "layers", keywords: ["perfil", "ubuntu", "lyra", "layout", "vanilla", "area de trabalho"] },
    { id: "dock", label: N_("Dock"), icon: "dock", keywords: ["dock", "icones", "barra de tarefas"] },
    { id: "menu", label: N_("Menu"), icon: "panel-top", keywords: ["painel", "barra superior", "relogio", "menu"] },
    { id: "screensaver", label: N_("Tela de bloqueio"), icon: "lock", keywords: ["bloqueio", "senha", "inatividade", "tela"] },
  ],

  render(ctx) {
    const view = DETAIL[ctx.sub];
    if (!view) return { el: overviewView(ctx) };
    ctx.setCrumb(t(VIEWS[ctx.sub]));
    const back = h("button", { class: "icon-btn", "aria-label": t("Voltar à personalização"), title: t("Voltar à personalização"), onclick: () => ctx.navigate("screen") }, icon("arrow-left", 18));
    return {
      el: h("div", { class: "pz-detail" },
        h("div", { class: "page-hero pz-detail-head" }, h("div", { class: "row-flex" }, back, h("h2", {}, t(VIEWS[ctx.sub])))),
        view(ctx)),
    };
  },
};
