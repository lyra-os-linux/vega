// Software: populares, desenvolvedores, buscar, instalados, atualizações e repositórios (ui/software.rs e
// application.rs do vega-gtk). Toda ação vira transação do vegad, acompanhada
// pelo painel de transação e pelo botão de tarefas do cabeçalho.

import { call, pick } from "../api.js";
import { refreshUpdates } from "../health.js";
import { N_, t } from "../i18n.js";
import { local, saveLocal } from "../store.js";
import { runSoftware, runWindows, runWindowsFile, runWindowsRemove, runNpm, runAppImage, runningTasks, onTasks, transactionPanel } from "../tasks.js";
import { h, icon, clear, button, badge, segmented, list, section, empty, skeletonRows, confirm, sheet, toast, fail, tile, normalize, meter, banner } from "../ui.js";
import { preparationPanel } from "./software/preparation.js";
import { requireRestart } from "../restart.js";
import { flatpakPanel } from "./software/flatpak.js";
import { POPULAR } from "./software/popular-catalog.js";
import { DEVELOPER } from "./software/developer-catalog.js";
import { NETWORK } from "./software/network-catalog.js";
import { WINDOWS, WINDOWS_BASE } from "./software/windows-catalog.js";

const TABS = ["popular", "developers", "network", "windows", "search", "installed", "updates"];
// Vitrines curadas (Populares e Desenvolvedores): mesmo cartão, catálogos diferentes.
const SHOWCASES = {
  popular: {
    catalog: POPULAR,
    loading: N_("Carregando aplicativos populares…"),
    intro: N_("Aplicativos que mais procuram no Lyra OS. Instale com um clique."),
    introLocked: N_("Aplicativos que mais procuram no Lyra OS. Os do Flathub ficam disponíveis depois de habilitar o Flatpak."),
  },
  developers: {
    catalog: DEVELOPER,
    loading: N_("Carregando ferramentas de desenvolvimento…"),
    intro: N_("Linguagens e ferramentas de programação. Instale com um clique."),
    introLocked: N_("Linguagens e ferramentas de programação. As do Flathub ficam disponíveis depois de habilitar o Flatpak."),
  },
  network: {
    catalog: NETWORK,
    loading: N_("Carregando ferramentas de redes e TI…"),
    intro: N_("Ferramentas para administrar redes e dar suporte técnico. Instale com um clique."),
    introLocked: N_("Ferramentas para administrar redes e dar suporte técnico. As do Flathub ficam disponíveis depois de habilitar o Flatpak."),
  },
  windows: {
    catalog: WINDOWS,
    base: WINDOWS_BASE,
    loading: N_("Carregando apps do Windows…"),
    intro: N_("Programas do Windows que rodam no Lyra OS pelo Wine. Instale com um clique, sem senha."),
    introLocked: N_("Programas do Windows que rodam no Lyra OS pelo Wine. O Bottles fica disponível depois de habilitar o Flatpak."),
  },
};
/** Origens instaladas na conta do usuário: sem detalhes no vegad, removidas pelo cartão. */
const USER_ORIGINS = new Set(["windows", "npm", "appimage"]);
const ORIGIN_RANK = { official: 0, flathub: 1 };

// Estado que sobrevive à troca de página, como os widgets do vega-gtk.
const memory = {
  tab: "popular",
  query: "",
  results: null,
  queue: new Map(), // "origem\u0000id" → pacote
  chosenOrigin: new Map(), // nome em minúsculas → origem escolhida
  lastTask: null,
};

const queueKey = (pkg) => `${pkg.origin}\u0000${pkg.id}`;
const originLabel = (origin) => (origin === "official" ? t("Oficial") : origin === "flathub" ? "Flathub" : origin === "windows" ? "Windows" : origin === "npm" ? "npm" : origin === "appimage" ? "AppImage" : origin);
const rankOrigin = (a, b) => (ORIGIN_RANK[a.origin] ?? 2) - (ORIGIN_RANK[b.origin] ?? 2);

function packageIcon(pkg, size = 40) {
  if (pkg.icon_data) {
    const img = h("img", { class: "sw-icon", src: pkg.icon_data, alt: "", loading: "lazy" });
    img.style.width = img.style.height = `${size}px`;
    return img;
  }
  const el = tile(pkg.name || pkg.id || "?");
  el.style.width = el.style.height = `${size}px`;
  return el;
}

/** Agrupa por nome (minúsculas) mantendo a ordem do backend; origens ordenadas oficial < flathub < outra. */
function groupByName(packages) {
  const groups = new Map();
  for (const pkg of packages) {
    const key = (pkg.name || pkg.id).toLowerCase();
    if (!groups.has(key)) groups.set(key, { key, items: [] });
    groups.get(key).items.push(pkg);
  }
  for (const group of groups.values()) group.items.sort(rankOrigin);
  return [...groups.values()];
}

/** Progresso por pacote (Baixando/Instalando %) vindo da transação em andamento. */
function packageProgress(name) {
  for (const task of runningTasks()) {
    const progress = task.packages?.[name];
    if (progress) return progress;
  }
  return null;
}

export default {
  id: "software",
  title: N_("Software"),
  icon: "package",
  group: "main",
  keywords: ["pacotes", "zypper", "flatpak", "programas", "aplicativos", "instalar", "remover", "atualizar", "repositorio", "populares", "desenvolvedores"],
  sections: [
    { id: "popular", label: N_("Populares"), icon: "star", keywords: ["populares", "recomendados", "spotify", "steam", "discord", "obs", "gimp"] },
    { id: "windows", label: N_("Apps do Windows"), icon: "layers", keywords: ["windows", "wine", "bottles", "exe", "winetricks"] },
    { id: "developers", label: N_("Desenvolvedores"), icon: "terminal", keywords: ["programar", "programacao", "linguagens", "python", "rust", "java", "node", "docker", "podman", "ide", "compilador"] },
    { id: "network", label: N_("Redes e TI"), icon: "server", keywords: ["rede", "suporte", "remoto", "ssh", "rdp", "vnc", "nmap", "mikrotik", "winbox", "helpdesk"] },
    { id: "search", label: N_("Buscar aplicativos e pacotes"), icon: "search", keywords: ["buscar", "instalar", "flathub"] },
    { id: "installed", label: N_("Instalados"), icon: "box", keywords: ["remover", "desinstalar"] },
    { id: "updates", label: N_("Atualizações"), icon: "download", keywords: ["update", "atualizar"] },
    { id: "repos", label: N_("Repositórios do sistema"), icon: "database", keywords: ["repo", "obs", "packman", "chave"] },
  ],

  render(ctx) {
    if (TABS.includes(ctx.sub)) memory.tab = ctx.sub;
    if (!TABS.includes(memory.tab)) memory.tab = "popular";
    let tab = memory.tab;
    let view = local.softwareView === "cards" ? "cards" : "list";
    let installedFilter = "";
    let installed = null;
    let updates = null;
    // Repositórios: janela própria (botão acima das abas), não uma aba.
    let repos = null;
    let reposError = null;
    let reposHost = null;
    let loadError = null;
    // Vitrines: chaves "origem\u0000id" instaladas e se o Flatpak está ativo.
    let popularInstalled = null;
    let flatpakEnabled = null;
    let enablingWindows = false;
    // Kernel em execução roda 32 bits? E a configuração de boot (para saber
    // se ia32_emulation=1 já foi aplicado e só falta reiniciar).
    let windowsIa32 = null;
    let bootStatus = null;
    let searching = false;
    let searchStatus = "";
    let updatingOne = false;

    const body = h("div", { class: "sw-body" });
    const panelHost = h("div", { class: "sw-transaction" });
    const toolbarHost = h("div");

    // ---------- Painel de transação ----------
    const showTask = (task) => {
      memory.lastTask = task;
      clear(panelHost, transactionPanel(task, { onClose: () => { memory.lastTask = null; clear(panelHost); } }));
    };
    const running = runningTasks().find((task) => task.kind === "software");
    if (running) showTask(running);
    else if (memory.lastTask) showTask(memory.lastTask);

    /** Inicia a transação, mostra o painel e trata o fim como o vega-gtk. */
    async function transact(action, title, { reload = true } = {}) {
      let started;
      try {
        started = await runSoftware(action, title);
      } catch (error) {
        fail(error, t("Não foi possível concluir a operação"));
        return null;
      }
      showTask(started.task);
      paint();
      const result = await started.done;
      if (!result.success) fail(result.message, t("Não foi possível concluir a operação"));
      if (reload) {
        // As atualizações são limpas mesmo em falha: o estado pode ter mudado.
        if (tab === "updates") updates = null;
        if (tab === "installed") installed = null;
        if (SHOWCASES[tab]) popularInstalled = null;
        if (tab === "search" && memory.query) runSearch();
        load();
      }
      refreshUpdates();
      return result;
    }

    // ---------- Ações de pacote ----------
    async function packageAction(pkg) {
      const verb = pkg.installed ? t("Remover") : t("Instalar");
      const ok = await confirm({
        title: t("{verb} {name}?", { verb, name: pkg.name }),
        text: t("A operação será autorizada pelo polkit e acompanhada até a conclusão."),
        ic: pkg.installed ? "trash" : "download",
        tone: pkg.installed ? "danger" : "",
        confirmLabel: verb,
        confirmKind: pkg.installed ? "danger" : "primary",
        required: true,
      });
      if (!ok) return;
      await transact({ kind: pkg.installed ? "remove" : "install", origin: pkg.origin, id: pkg.id }, t("{verb} {name}", { verb, name: pkg.name }));
    }

    async function updateOne(pkg) {
      if (updatingOne) return;
      const verb = t("Atualizar");
      const ok = await confirm({
        title: t("{verb} {name}?", { verb, name: pkg.name }),
        text: t("A operação será autorizada pelo polkit e acompanhada até a conclusão."),
        ic: "download",
        confirmLabel: verb,
        required: true,
      });
      if (!ok) return;
      updatingOne = true;
      paint();
      try {
        await transact({ kind: "update-package", origin: pkg.origin, id: pkg.id }, t("Atualizando {name}…", { name: pkg.name }));
      } finally {
        updatingOne = false;
        paint();
      }
    }

    // Os Flatpaks se atualizam pela sessão do usuário, sem senha; só os
    // pacotes do sistema passam pelo vegad e pedem autorização.
    async function updateAll() {
      const pending = updates ?? [];
      const flatpaks = pending.filter((pkg) => pkg.origin === "flathub");
      const native = pending.some((pkg) => pkg.origin !== "flathub");
      const ok = await confirm({
        title: t("Atualizar tudo?"),
        text: native
          ? t("Atualizar os pacotes do sistema e os Flatpaks agora? Os pacotes do sistema pedem a senha de administrador; os Flatpaks, não.")
          : t("Atualizar os Flatpaks agora? Não é preciso senha."),
        ic: "download",
        confirmLabel: t("Atualizar"),
        required: false,
      });
      if (!ok) return;
      for (const [index, pkg] of flatpaks.entries()) {
        const result = await transact({ kind: "update-package", origin: pkg.origin, id: pkg.id },
          t("Atualizando {name} ({current}/{total})…", { name: pkg.name, current: index + 1, total: flatpaks.length }), { reload: false });
        if (!result) break;
      }
      if (native) await transact({ kind: "update-all-native" }, t("Atualizar pacotes do sistema"));
      else load();
    }

    async function clearCache() {
      const ok = await confirm({
        title: t("Limpar cache?"),
        text: t("Remover o cache de pacotes e runtimes Flatpak órfãos agora?"),
        ic: "trash",
        confirmLabel: t("Limpar"),
        required: false,
      });
      if (ok) await transact({ kind: "clear-cache" }, t("Limpar cache"), { reload: false });
    }

    async function installQueue() {
      const queue = [...memory.queue.values()];
      if (!queue.length) return;
      const ok = await confirm({
        title: t("Instalar {count} pacote(s)?", { count: queue.length }),
        text: t("Cada pacote será instalado em sua própria transação, autorizada pelo polkit."),
        ic: "download",
        confirmLabel: t("Instalar"),
        required: true,
      });
      if (!ok) return;
      // Flathub primeiro: o polkit dos pacotes do sistema pede senha uma vez, no fim.
      queue.sort((a, b) => (a.origin === "flathub" ? 0 : 1) - (b.origin === "flathub" ? 0 : 1));
      const failures = [];
      let installedCount = 0;
      for (const [index, pkg] of queue.entries()) {
        let started;
        try {
          started = await runSoftware({ kind: "install", origin: pkg.origin, id: pkg.id }, t("Instalando {name} ({current}/{total})…", { name: pkg.name, current: index + 1, total: queue.length }));
        } catch (error) {
          // Sem a confirmação da anterior, as demais não são iniciadas.
          failures.push(`${pkg.name}: ${error.message}`);
          for (const rest of queue.slice(index + 1)) failures.push(`${rest.name}: ${t("Não iniciado: a fila foi interrompida após perder a confirmação da operação anterior.")}`);
          break;
        }
        showTask(started.task);
        const result = await started.done;
        if (result.success) {
          installedCount += 1;
          memory.queue.delete(queueKey(pkg));
        } else {
          failures.push(`${pkg.name}: ${result.message}`);
        }
        paint();
      }
      if (failures.length) fail(t("Falha ao instalar {failed} de {total} pacote(s): {details}", { failed: failures.length, total: queue.length, details: failures.join("; ") }));
      else toast(t("{count} pacote(s) instalado(s) com sucesso", { count: installedCount }));
      if (memory.query) runSearch();
      installed = null;
      popularInstalled = null;
      refreshUpdates();
      paint();
      load();
    }

    // ---------- Detalhes ----------
    function openDetails(pkg) {
      const content = h("div", { class: "sw-details" }, h("div", { class: "row-flex" }, h("span", { class: "spinner" }), t("Carregando detalhes…")));
      const close = sheet(`${pkg.name} • ${originLabel(pkg.origin)}`, content);
      call("software_details", { origin: pkg.origin, id: pkg.id }).then(
        (d) => {
          const fact = (label, value) => value && [h("dt", {}, label), h("dd", { class: "selectable", title: value }, value)];
          clear(content,
            h("div", { class: "sw-details-head" },
              packageIcon(pkg, 64),
              h("div", { class: "spacer" },
                h("h3", {}, d.name || pkg.name),
                h("div", { class: "row-flex" }, badge(originLabel(d.origin), "outline"), d.installed && badge(t("Instalado"), "green", "check"))),
              button(d.installed ? t("Remover") : t("Instalar"), {
                kind: d.installed ? "danger" : "primary",
                icon: d.installed ? "trash" : "download",
                onClick: () => { close(); packageAction({ ...pkg, installed: d.installed }); },
              })),
            d.description && h("p", { class: "sw-desc selectable" }, d.description),
            h("dl", { class: "kv card pad" },
              fact(t("Disponível"), d.available_version),
              fact(t("Instalado"), d.installed_version),
              fact(t("Download"), d.download_size),
              fact(t("Em disco"), d.installed_size),
              fact(t("Licenças"), d.licenses.join(", ")),
              fact("URL", d.url),
              fact(t("Mantenedor"), d.maintainer)),
            d.dependencies.length > 0 && h("details", { class: "sw-deps" }, h("summary", {}, t("Dependências ({count})", { count: d.dependencies.length })), h("div", { class: "mono selectable" }, d.dependencies.join("\n"))));
        },
        (error) => clear(content, empty("alert-circle", t("Detalhes indisponíveis"), error.message)),
      );
    }

    // ---------- Linhas e cartões ----------
    function progressFor(name) {
      const progress = packageProgress(name);
      if (!progress) return null;
      const label = progress.phase === "download" ? t("Baixando") : t("Instalando");
      return h("div", { class: "sw-pkg-progress" }, h("span", { class: "small muted" }, `${label} ${progress.percent}%`), meter(progress.percent / 100));
    }

    function queueToggle(pkg) {
      const input = h("input", { type: "checkbox", class: "check" });
      input.checked = memory.queue.has(queueKey(pkg));
      input.addEventListener("click", (e) => e.stopPropagation());
      input.addEventListener("change", () => {
        if (input.checked) memory.queue.set(queueKey(pkg), pkg);
        else memory.queue.delete(queueKey(pkg));
        paintToolbar();
      });
      return h("label", { class: "sw-queue", title: t("Adicionar à fila de instalação"), onclick: (e) => e.stopPropagation() }, input, h("span", { class: "small" }, t("Fila")));
    }

    function originPills(group, current, onPick) {
      if (group.items.length < 2) return badge(originLabel(current.origin), current.origin === "official" ? "accent" : "outline");
      return h("div", { class: "sw-origins", role: "radiogroup", "aria-label": t("Escolha a origem antes de instalar") },
        group.items.map((pkg) => h("button", {
          class: "sw-origin", role: "radio", "aria-checked": String(pkg === current),
          title: t("Usar a origem {origin}", { origin: originLabel(pkg.origin) }),
          onclick: (e) => { e.stopPropagation(); onPick(pkg); },
        }, originLabel(pkg.origin))));
    }

    /** Linha/cartão de um grupo de busca (um nome, várias origens). */
    function searchItem(group) {
      const chosen = memory.chosenOrigin.get(group.key);
      const current = group.items.find((pkg) => pkg.origin === chosen) ?? group.items[0];
      const anyInstalled = group.items.some((pkg) => pkg.installed);
      const pick = (pkg) => { memory.chosenOrigin.set(group.key, pkg.origin); paint(); };
      const suffix = [
        anyInstalled && badge(t("Instalado"), "green", "check"),
        !current.installed && queueToggle(current),
      ];
      return item(current, { pills: originPills(group, current, pick), suffix, onOpen: () => openDetails(current) });
    }

    function item(pkg, { pills, suffix = [], onOpen, description } = {}) {
      const desc = description ?? pkg.description;
      if (view === "cards") {
        return h("button", { class: "card interactive sw-card", onclick: onOpen },
          h("div", { class: "sw-card-top" }, packageIcon(pkg, 48), h("div", { class: "spacer" }), suffix),
          h("strong", { class: "sw-card-name" }, pkg.name),
          h("p", { class: "sw-card-desc" }, desc || " "),
          h("div", { class: "sw-card-foot" }, pills ?? badge(originLabel(pkg.origin), "outline")),
          progressFor(pkg.name));
      }
      return h("div", { class: "list-row clickable sw-row", tabindex: "0", role: "button", onclick: onOpen, onkeydown: (e) => { if (e.key === "Enter" && e.target === e.currentTarget) onOpen?.(); } },
        h("span", { class: "sw-lead" }, packageIcon(pkg, 40)),
        h("div", { class: "body" },
          h("div", { class: "title" }, pkg.name),
          desc && h("div", { class: "subtitle" }, desc),
          progressFor(pkg.name)),
        h("div", { class: "suffix" }, pills ?? badge(originLabel(pkg.origin), "outline"), suffix));
    }

    const collection = (items) => (view === "cards" ? h("div", { class: "grid auto sw-cards" }, items) : list(items));

    // ---------- Vitrines (Populares e Desenvolvedores) ----------
    /** Instalação em andamento do cartão: progresso do pacote (zypper) ou da transação. */
    function cardProgress(app) {
      const task = runningTasks().find((item) => item.kind === "software" && ["install", "remove"].includes(item.action?.kind)
        && item.action.origin === app.origin && item.action.id === app.id);
      if (!task) return null;
      const percent = task.packages?.[app.id]?.percent ?? task.percent;
      const removing = task.action.kind === "remove";
      const label = removing
        ? (percent != null ? t("Removendo… {percent}%", { percent }) : t("Removendo…"))
        : (percent != null ? t("Instalando… {percent}%", { percent }) : t("Instalando…"));
      return h("div", { class: "sw-pkg-progress sw-card-progress" },
        h("span", { class: "small muted" }, label),
        percent != null
          ? meter(percent / 100)
          : h("div", { class: "meter indeterminate" }, h("i", { style: { width: "30%" } })));
    }

    /** Cartão de vitrine: Instalar, "Instalado" ou bloqueado até habilitar o Flatpak. */
    /** `gateNote`: motivo de a seção estar bloqueada (base do Windows ou suporte a 32 bits). */
    function popularCard(app, { gateNote = null } = {}) {
      // `alternatives`: pacotes que já entregam a mesma ferramenta (o Rust do
      // sistema no lugar do rustup). Se um deles está instalado, o cartão
      // fica desabilitado como "Instalado".
      const installedId = [app.id, ...(app.alternatives ?? [])]
        .find((id) => popularInstalled.has(queueKey({ origin: app.origin, id })));
      const pkg = { origin: app.origin, id: app.id, name: app.name, installed: false };
      const installed = Boolean(installedId);
      const needsFlatpak = !installed && app.origin === "flathub" && !flatpakEnabled;
      const locked = needsFlatpak || (!installed && Boolean(gateNote));
      // Instalado ou bloqueado: o cartão não abre detalhes nem instala.
      const disabled = installed || locked;
      const busy = runningTasks().some((task) => task.kind === "software");
      // Sem `logo` (marca registrada): ícone genérico com a inicial.
      const logo = app.logo
        ? h("img", { class: "sw-icon", src: `img/popular/${app.logo}.png`, alt: "", loading: "lazy", width: "48", height: "48" })
        : packageIcon(app, 48);
      const progress = cardProgress(app);
      let action;
      if (progress) action = progress;
      // Apps do Windows instalados podem ser removidos aqui mesmo: não
      // aparecem na aba Instalados, que lista pacotes e Flatpaks.
      else if (installed && USER_ORIGINS.has(app.origin)) action = [
        badge(t("Instalado"), "green", "check"),
        button(t("Remover"), { size: "sm", icon: "trash", kind: "danger", disabled: busy, onClick: (e) => { e.stopPropagation(); removeUserApp(app); } }),
      ];
      else if (installed) action = button(t("Instalado"), { size: "sm", icon: "check", disabled: true });
      else if (locked) action = button(t("Instalar"), { size: "sm", icon: "download", disabled: true, title: needsFlatpak ? t("Habilite o Flatpak para instalar") : gateNote });
      else action = button(t("Instalar"), { kind: "primary", size: "sm", icon: "download", disabled: busy, onClick: (e) => {
        e.stopPropagation();
        if (app.origin === "windows") installWindows(app);
        else if (app.origin === "npm") installNpm(app);
        else if (app.origin === "appimage") installAppImage(app);
        else if (app.repo?.package) installFromRepoPackage(app);
        else if (app.repo) installFromRepo(app);
        else packageAction(pkg);
      } });
      // Apps instalados na conta do usuário não têm detalhes no vegad.
      const open = disabled || USER_ORIGINS.has(app.origin) ? undefined : () => openDetails(pkg);
      // div, e não button: o cartão contém o botão Instalar.
      return h("div", {
        class: `card sw-card sw-popular ${locked ? "locked" : installed ? "installed" : "interactive"}`,
        role: disabled ? undefined : "button",
        tabindex: disabled ? undefined : "0",
        "aria-disabled": disabled ? "true" : undefined,
        onclick: open,
        onkeydown: open && ((e) => { if (e.key === "Enter" && e.target === e.currentTarget) open(); }),
      },
        h("div", { class: "sw-card-top" }, logo, h("div", { class: "spacer" }), badge(originLabel(app.origin), app.origin === "official" ? "accent" : "outline")),
        h("strong", { class: "sw-card-name" }, app.name),
        h("p", { class: "sw-card-desc" }, t(app.text)),
        locked && h("p", { class: "sw-locked-note small" }, icon("lock", 13), needsFlatpak
          ? t("Habilite o Flatpak primeiro para instalar este app.")
          : gateNote),
        h("div", { class: "sw-card-foot" }, action));
    }

    const isInstalled = (app) => popularInstalled.has(queueKey(app));

    const IA32_ON = /(^|\s)ia32_emulation=(1|y|yes|on|true)(\s|$)/i;
    /** ia32_emulation=1 já está no boot, mas o kernel em execução ainda é o antigo. */
    const ia32AwaitsReboot = () => !windowsIa32 && IA32_ON.test(bootStatus?.cmdline ?? "");

    /** Bloqueio dos apps do Windows: base faltando, 32 bits desligado ou reinício pendente. */
    function windowsGate(base) {
      const busy = runningTasks().some((task) => task.kind === "software");
      if (!base.every(isInstalled)) {
        return {
          note: t("Habilite os apps do Windows primeiro para instalar este app."),
          banner: banner("info", "layers", t("Apps do Windows desabilitados"),
            t("Para instalar programas do Windows, o Vega precisa do Wine e do Winetricks (e do Bottles, se o Flatpak estiver habilitado)."),
            button(enablingWindows ? t("Habilitando…") : t("Habilitar apps do Windows"), { kind: "primary", disabled: busy || enablingWindows, onClick: enableWindows })),
        };
      }
      if (windowsIa32) return null;
      if (ia32AwaitsReboot()) {
        return {
          note: t("Reinicie o computador para instalar este app."),
          banner: banner("warn", "restart", t("Reinicie o computador para concluir"),
            t("O suporte a programas de 32 bits já está nos parâmetros de boot e passa a valer depois de reiniciar.")),
        };
      }
      return {
        note: t("Ative o suporte a programas de 32 bits primeiro."),
        banner: banner("warn", "cpu", t("Suporte a programas de 32 bits desligado"),
          t("Quase todos os programas do Windows são de 32 bits, e o kernel do Lyra OS vem com esse suporte desligado. Sem ele, o Wine não consegue abrir os instaladores."),
          button(enablingWindows ? t("Ativando…") : t("Ativar suporte a 32 bits"), { kind: "primary", disabled: busy || enablingWindows || !bootStatus, onClick: enableIa32 })),
      };
    }

    function drawShowcase({ catalog, base, loading, intro, introLocked }) {
      if (loadError) return empty("alert-circle", t("Falha: {message}", { message: loadError }));
      if (!popularInstalled || flatpakEnabled == null) return h("div", {}, h("p", { class: "sw-status" }, t(loading)), skeletonRows(5));
      const gate = base ? windowsGate(base) : null;
      const busy = runningTasks().some((task) => task.kind === "software");
      return h("div", {},
        gate && h("div", { class: "sw-flatpak" }, gate.banner),
        h("p", { class: "sw-status" }, t(flatpakEnabled ? intro : introLocked)),
        base && !gate && section(t("Outro programa"), {},
          h("div", { class: "card pad sw-run-exe" },
            h("div", { class: "row-flex" }, icon("play", 20),
              h("div", { class: "spacer" },
                h("strong", {}, t("Rodar um programa do Windows")),
                h("p", { class: "small muted" }, t("Escolha o instalador (.exe ou .msi) que você baixou do site do fabricante. Ele roda pelo Wine, na sua conta, sem senha."))),
              button(t("Escolher arquivo…"), { kind: "primary", icon: "folder", disabled: busy, onClick: runExe })))),
        catalog.map((category) => section(t(category.title), { sub: String(category.apps.length) },
          h("div", { class: "grid auto sw-cards" }, category.apps.map((app) => popularCard(app, { gateNote: category.gated ? gate?.note : null }))))));
    }

    /**
     * Acrescenta ia32_emulation=1 aos parâmetros de boot pelo vegad (polkit e
     * snapshot), mantendo a entrada padrão e o tempo de espera atuais.
     */
    async function enableIa32() {
      if (!bootStatus) return false;
      const ok = await confirm({
        title: t("Ativar suporte a programas de 32 bits?"),
        text: t("O Vega vai acrescentar ia32_emulation=1 aos parâmetros de boot, com snapshot, e pedir a senha de administrador. O openSUSE desliga esse suporte por padrão para reduzir a superfície de ataque do kernel; ligado, programas de 32 bits voltam a rodar. Vale depois de reiniciar o computador."),
        ic: "cpu",
        tone: "warn",
        confirmLabel: t("Ativar"),
        required: true,
      });
      if (!ok) return false;
      enablingWindows = true;
      paint();
      const cmdline = [...bootStatus.cmdline.split(/\s+/).filter((arg) => arg && !arg.startsWith("ia32_emulation=")), "ia32_emulation=1"].join(" ");
      let done = false;
      try {
        await call("kernel_apply_boot", { entry: bootStatus.default_entry, timeout: bootStatus.timeout, cmdline });
        done = true;
      } catch (error) {
        fail(error, t("Não foi possível concluir a operação"));
      }
      enablingWindows = false;
      popularInstalled = null;
      paint();
      load();
      if (done) await requireRestart("ia32");
      return done;
    }

    /** Instala o que falta da base do Windows: Flatpak (Bottles) primeiro, depois um pacote por vez. */
    async function enableWindows() {
      const missing = WINDOWS.find((category) => category.id === "base").apps
        .filter((app) => !isInstalled(app) && (app.origin !== "flathub" || flatpakEnabled))
        .sort((a, b) => (a.origin === "flathub" ? 0 : 1) - (b.origin === "flathub" ? 0 : 1));
      if (!missing.length) return;
      const ok = await confirm({
        title: t("Habilitar apps do Windows?"),
        text: t("Serão instalados: {names}. Os pacotes do sistema pedem a senha de administrador; o Bottles, não.", { names: missing.map((app) => app.name).join(", ") }),
        ic: "layers",
        confirmLabel: t("Habilitar"),
        required: true,
      });
      if (!ok) return;
      enablingWindows = true;
      paint();
      try {
        for (const [index, app] of missing.entries()) {
          const result = await transact({ kind: "install", origin: app.origin, id: app.id },
            t("Instalando {name} ({current}/{total})…", { name: app.name, current: index + 1, total: missing.length }), { reload: false });
          if (!result?.success) break;
        }
      } finally {
        enablingWindows = false;
        popularInstalled = null;
        paint();
      }
      // Base pronta: o próximo passo é o suporte a 32 bits do kernel.
      await load();
      if (WINDOWS_BASE.every(isInstalled) && !windowsIa32 && !ia32AwaitsReboot()) await enableIa32();
    }

    /** Escolhe um .exe/.msi e roda no prefixo do Vega. */
    async function runExe() {
      let path;
      try {
        path = await pick({ title: t("Escolha o programa do Windows"), filters: [{ name: t("Programas do Windows"), extensions: ["exe", "msi", "EXE", "MSI"] }] });
      } catch (error) {
        return fail(error);
      }
      if (!path || Array.isArray(path)) return;
      const name = String(path).split("/").pop();
      const ok = await confirm({
        title: t("Rodar {name}?", { name }),
        text: t("Rode só programas de fontes confiáveis, como o site do fabricante. Nem todo programa do Windows funciona no Wine; se a instalação terminar, o programa aparece no menu de aplicativos."),
        ic: "play",
        tone: "warn",
        confirmLabel: t("Rodar"),
        required: true,
      });
      if (!ok) return;
      let started;
      try {
        started = await runWindowsFile(path, t("Rodando {name}…", { name }));
      } catch (error) {
        return fail(error, t("Não foi possível concluir a operação"));
      }
      showTask(started.task);
      paint();
      const result = await started.done;
      if (!result.success) fail(result.message, t("Não foi possível concluir a operação"));
      else toast(t("{name} terminou.", { name }));
      popularInstalled = null;
      load();
    }

    /** Remove um app instalado na conta do usuário (Windows, npm ou AppImage). */
    async function removeUserApp(app) {
      const npm = app.origin === "npm";
      const appimage = app.origin === "appimage";
      const ok = await confirm({
        title: t("Remover {name}?", { name: app.name }),
        text: npm
          ? t("A ferramenta é removida da sua conta pelo npm, sem senha. As configurações e o login dela continuam na sua pasta pessoal.")
          : appimage
            ? t("O AppImage e o atalho saem da sua conta, sem senha. Os aplicativos que ele instalou e as configurações dele continuam no sistema.")
            : t("O desinstalador do programa roda pelo Wine, na sua conta, sem senha. Arquivos que você salvou com ele, como documentos, não são apagados."),
        ic: "trash",
        tone: "danger",
        confirmLabel: t("Remover"),
        confirmKind: "danger",
        required: true,
      });
      if (!ok) return;
      let started;
      try {
        const title = t("Removendo {name}…", { name: app.name });
        started = npm ? await runNpm(false, app.id, title)
          : appimage ? await runAppImage(false, app.id, title)
          : await runWindowsRemove(app.id, title);
      } catch (error) {
        return fail(error, t("Não foi possível concluir a operação"));
      }
      showTask(started.task);
      paint();
      const result = await started.done;
      if (!result.success) fail(result.message, t("Não foi possível concluir a operação"));
      popularInstalled = null;
      load();
    }

    /**
     * Assistente de terminal pelo npm, na conta do usuário. Sem o npm do
     * sistema, instala antes o Node.js 24 + npm (pede a senha).
     */
    async function installNpm(app) {
      const needsNode = !isInstalled({ origin: "official", id: "npm24" });
      const ok = await confirm({
        title: t("Instalar {name}?", { name: app.name }),
        text: needsNode
          ? t("O Vega instala antes o Node.js 24 e o npm, com a senha de administrador. Depois instala a ferramenta na sua conta, sem senha; o comando fica em ~/.local/bin.")
          : t("A ferramenta é instalada na sua conta pelo npm, sem senha; o comando fica em ~/.local/bin."),
        ic: "download",
        confirmLabel: t("Instalar"),
        required: true,
      });
      if (!ok) return;
      if (needsNode) {
        const result = await transact({ kind: "install", origin: "official", id: "npm24" }, t("Instalando {name}…", { name: "Node.js 24 + npm" }), { reload: false });
        if (!result?.success) return;
      }
      let started;
      try {
        started = await runNpm(true, app.id, t("Instalando {name}…", { name: app.name }));
      } catch (error) {
        return fail(error, t("Não foi possível concluir a operação"));
      }
      showTask(started.task);
      paint();
      const result = await started.done;
      if (!result.success) fail(result.message, t("Não foi possível concluir a operação"));
      popularInstalled = null;
      load();
    }

    /**
     * AppImage oficial na conta do usuário. Sem o FUSE 2 (libfuse2), que quase
     * todo AppImage exige, instala antes o pacote (pede a senha).
     */
    async function installAppImage(app) {
      const needsFuse = !isInstalled({ origin: "official", id: "libfuse2" });
      const ok = await confirm({
        title: t("Instalar {name}?", { name: app.name }),
        text: t("O Vega baixa o AppImage oficial da versão mais recente, confere o checksum publicado pelo projeto e instala na sua conta, sem senha. O aplicativo se atualiza sozinho.")
          + (needsFuse ? " " + t("Antes, instala o suporte a AppImage (libfuse2), com a senha de administrador.") : ""),
        ic: "download",
        confirmLabel: t("Instalar"),
        required: true,
      });
      if (!ok) return;
      if (needsFuse) {
        const result = await transact({ kind: "install", origin: "official", id: "libfuse2" }, t("Instalando {name}…", { name: "libfuse2" }), { reload: false });
        if (!result?.success) return;
      }
      let started;
      try {
        started = await runAppImage(true, app.id, t("Instalando {name}…", { name: app.name }));
      } catch (error) {
        return fail(error, t("Não foi possível concluir a operação"));
      }
      showTask(started.task);
      paint();
      const result = await started.done;
      if (!result.success) fail(result.message, t("Não foi possível concluir a operação"));
      popularInstalled = null;
      load();
    }

    async function installWindows(app) {
      const ok = await confirm({
        title: t("Instalar {name}?", { name: app.name }),
        text: t("O instalador é baixado do site do fabricante e roda pelo Wine, na sua conta, sem senha. O app aparece no menu de aplicativos."),
        ic: "download",
        confirmLabel: t("Instalar"),
        required: true,
      });
      if (!ok) return;
      let started;
      try {
        started = await runWindows(app.id, t("Instalando {name}…", { name: app.name }));
      } catch (error) {
        return fail(error, t("Não foi possível concluir a operação"));
      }
      showTask(started.task);
      paint();
      const result = await started.done;
      if (!result.success) fail(result.message, t("Não foi possível concluir a operação"));
      popularInstalled = null;
      load();
    }

    async function loadPopular() {
      const [details, packages, windows, npmApps, appImages, ia32, boot] = await Promise.all([
        call("software_details", { origin: "official", id: "flatpak" }).catch(() => null),
        call("software_installed"),
        call("windows_installed").catch(() => []),
        call("npm_installed").catch(() => []),
        call("appimage_installed").catch(() => []),
        call("windows_ia32").catch(() => true),
        call("kernel_boot_status").catch(() => null),
      ]);
      windowsIa32 = ia32;
      bootStatus = boot;
      flatpakEnabled = Boolean(details?.installed);
      popularInstalled = new Set([
        ...packages.map(queueKey),
        ...windows.map((id) => queueKey({ origin: "windows", id })),
        ...npmApps.map((id) => queueKey({ origin: "npm", id })),
        ...appImages.map((id) => queueKey({ origin: "appimage", id })),
      ]);
    }

    // ---------- Abas ----------
    function drawSearch() {
      if (searching) return h("div", {}, h("p", { class: "sw-status" }, t("Consultando as origens disponíveis…")), skeletonRows(5));
      if (searchStatus) return empty("alert-circle", searchStatus);
      if (!memory.results) return empty("search", t("Buscar aplicativos e pacotes"), t("Os detalhes e a ação disponível aparecerão aqui."));
      if (!memory.results.length) return empty("search", t("Nenhum resultado encontrado"));
      const groups = groupByName(memory.results);
      return h("div", {},
        h("p", { class: "sw-status" }, t("Escolha a origem antes de instalar")),
        collection(groups.map(searchItem)));
    }

    function drawInstalled() {
      if (loadError) return empty("alert-circle", t("Falha: {message}", { message: loadError }));
      if (!installed) return h("div", {}, h("p", { class: "sw-status" }, t("Carregando pacotes instalados…")), skeletonRows(6));
      const q = normalize(installedFilter);
      const items = installed.filter((pkg) => !q || normalize(`${pkg.name} ${pkg.id} ${pkg.description}`).includes(q));
      if (!items.length) return empty("search", t("Nenhum resultado encontrado"));
      const shown = items.slice(0, 400);
      return h("div", {},
        collection(shown.map((pkg) => item(pkg, { onOpen: () => openDetails(pkg) }))),
        items.length > shown.length && h("p", { class: "sw-status" }, t("Mostrando {shown} de {total}", { shown: shown.length, total: items.length })));
    }

    function drawUpdates() {
      if (loadError) return empty("alert-circle", t("Falha: {message}", { message: loadError }));
      if (!updates) return h("div", {}, h("p", { class: "sw-status" }, t("Verificando atualizações…")), skeletonRows(5));
      if (!updates.length) return empty("check-circle", t("Nenhuma atualização disponível"), t("Tudo em dia"), null, "green");
      // Agrupado por repositório, na ordem em que o backend entrega.
      const groups = new Map();
      for (const pkg of updates) {
        const name = pkg.repository || t("Outros");
        if (!groups.has(name)) groups.set(name, []);
        groups.get(name).push(pkg);
      }
      const busy = updatingOne || runningTasks().some((task) => task.kind === "software");
      return h("div", {}, [...groups].map(([repo, items]) =>
        section(repo, { sub: String(items.length) }, collection(items.map((pkg) => item(pkg, {
          description: pkg.description || (pkg.origin === "flathub" ? t("Atualização disponível") : ""),
          onOpen: () => openDetails(pkg),
          suffix: [h("button", { class: "btn sm", disabled: busy, onclick: (e) => { e.stopPropagation(); updateOne(pkg); } }, icon("download", 14), t("Atualizar"))],
        }))))));
    }

    function drawRepos() {
      if (reposError) return empty("alert-circle", t("Falha: {message}", { message: reposError }));
      if (!repos) return skeletonRows(5);
      const name = h("input", { class: "input", placeholder: t("nome"), "aria-label": t("nome") });
      const url = h("input", { class: "input", placeholder: "https://…", "aria-label": t("URL"), inputmode: "url" });
      const add = button(t("Adicionar repositório"), { kind: "primary", icon: "plus", disabled: true, onClick: () => addRepo(name.value.trim(), url.value.trim()) });
      const sync = () => (add.disabled = !name.value.trim() || !url.value.trim());
      name.addEventListener("input", sync);
      url.addEventListener("input", sync);
      url.addEventListener("keydown", (e) => { if (e.key === "Enter" && !add.disabled) add.click(); });
      return h("div", { class: "sw-repos" },
        section(t("Repositórios do sistema"), { sub: String(repos.length) },
          repos.length === 0 ? empty("database", t("Nenhum resultado encontrado")) : list(repos.map((repo) =>
            h("div", { class: "list-row" },
              h("span", { class: `lead ${repo.enabled ? "green" : ""}`.trim() }, icon("database", 18)),
              h("div", { class: "body" }, h("div", { class: "title selectable" }, repo.name)),
              h("div", { class: "suffix" },
                badge(repo.enabled ? t("Ativo") : t("Inativo"), repo.enabled ? "green" : ""),
                button(repo.enabled ? t("Desativar") : t("Ativar"), { size: "sm", onClick: () => toggleRepo(repo) }))))),
          h("p", { class: "sw-status" }, t("Selecione um repositório. A lista mostra o estado informado pelo gerenciador de pacotes da distribuição."))),
        section(t("Novo repositório"), {},
          h("div", { class: "card pad sw-repo-form" },
            h("div", { class: "sw-repo-fields" }, name, url, add),
            h("p", { class: "small muted" }, t("Se o repositório for assinado, a chave é baixada e mostrada pra você aprovar antes de confiar nela.")))));
    }

    /** Recarrega a lista de repositórios, se a janela estiver aberta. */
    async function refreshRepos() {
      repos = null;
      reposError = null;
      if (!reposHost) return;
      clear(reposHost, drawRepos());
      try {
        repos = await call("software_repos");
      } catch (error) {
        reposError = error.message;
      }
      if (reposHost) clear(reposHost, drawRepos());
    }

    function openRepos() {
      reposHost = h("div", {});
      sheet(t("Repositórios"), reposHost, { onClose: () => { reposHost = null; } });
      refreshRepos();
    }

    async function toggleRepo(repo) {
      const enabled = !repo.enabled;
      const verb = enabled ? t("Ativar") : t("Desativar");
      const ok = await confirm({ title: t("{verb} repositório?", { verb }), text: repo.name, ic: "database", confirmLabel: verb, required: false });
      if (!ok) return;
      try {
        await call("software_set_repo_enabled", { repo: repo.name, enabled });
        toast(t("Repositório alterado com sucesso"));
      } catch (error) {
        fail(error);
      }
      refreshRepos();
    }

    /** Mostra a chave pendente e, se o usuário confiar, aprova; devolve se ficou confiável. */
    async function approveKey(key) {
      const text = key.key_id
        ? t("\"{repo}\" está assinado por:\n{user}\nImpressão digital: {fingerprint}\n\nConfiar nessa chave e continuar?", { repo: key.repo, user: key.user_id, fingerprint: key.fingerprint })
        : t("Não foi possível obter uma chave verificável para \"{repo}\" (assinado por: {user}). Deseja continuar mesmo assim, sem verificação de assinatura?", { repo: key.repo, user: key.user_id });
      const trust = await confirm({
        title: t("Confiar na chave do repositório?"),
        ic: "key",
        tone: "warn",
        extra: h("p", { class: "sw-pre selectable" }, text),
        confirmLabel: t("Confiar"),
        required: true,
      });
      if (!trust) return false;
      return Boolean((await transact({ kind: "trust-repo-key", repo: key.repo, key_id: key.key_id }, t("Confiando na chave…"), { reload: false }))?.success);
    }

    const fingerprintOf = (value) => String(value ?? "").replace(/\s+/g, "").toUpperCase();

    /**
     * Cartão cujo repositório vem num pacote do Lyra (vscode-repo,
     * cursor-repo, claude-code-repo): instala o pacote, pede ao vegad a chave
     * do repositório (TrustRepoKey propõe a chave e emite RepoKeyPending),
     * confere a impressão digital esperada, deixa o usuário aprovar e instala
     * o aplicativo. Sem isso o zypper ignora o repositório para sempre.
     */
    async function installFromRepoPackage(app) {
      const { package: repoPackage, name: repoName, fingerprint } = app.repo;
      const ok = await confirm({
        title: t("Instalar {name}?", { name: app.name }),
        text: t("O {name} vem do repositório oficial do fabricante. O Vega instala o registro desse repositório, mostra a chave de assinatura para você aprovar e instala o {name}. Pede a senha de administrador.", { name: app.name }),
        ic: "download",
        confirmLabel: t("Instalar"),
        required: true,
      });
      if (!ok) return;
      if (!isInstalled({ origin: "official", id: repoPackage })) {
        const added = await transact({ kind: "install", origin: "official", id: repoPackage }, t("Instalando {name}…", { name: repoPackage }), { reload: false });
        if (!added?.success) return;
      }
      let started;
      try {
        started = await runSoftware({ kind: "trust-repo-key", repo: repoName, key_id: fingerprint }, t("Conferindo a chave do repositório…"));
      } catch (error) {
        return fail(error, t("Não foi possível concluir a operação"));
      }
      showTask(started.task);
      const result = await started.done;
      if (!result.success) {
        const key = started.task.keyPending;
        if (!key) return fail(result.message, t("Não foi possível concluir a operação"));
        if (fingerprintOf(key.fingerprint) !== fingerprintOf(fingerprint)) {
          return fail(t("A chave do repositório ({got}) não é a esperada ({want}). Nada foi instalado.", { got: key.fingerprint, want: fingerprint }), t("Chave inesperada"));
        }
        if (!(await approveKey(key))) return;
      }
      await transact({ kind: "install", origin: app.origin, id: app.id }, t("{verb} {name}", { verb: t("Instalar"), name: app.name }));
    }

    /** Adiciona o repositório (aprovando a chave, se pedir); devolve se ficou pronto para uso. */
    async function addRepo(name, url) {
      let started;
      let ready = false;
      try {
        started = await runSoftware({ kind: "add-repo", name, url }, t("Adicionando {name}…", { name }));
      } catch (error) {
        fail(error, t("Não foi possível concluir a operação"));
        return false;
      }
      showTask(started.task);
      const result = await started.done;
      const key = started.task.keyPending;
      if (!result.success && key) {
        toast(t("Repositório adicionado — assinatura pendente de aprovação"), { ic: "key" });
        ready = await approveKey(key);
      } else if (!result.success) {
        fail(result.message, t("Não foi possível concluir a operação"));
      } else {
        ready = true;
      }
      refreshRepos();
      return ready;
    }

    /**
     * Cartão com `repo` (pacote só no repositório do fabricante, como o .NET da
     * Microsoft): adiciona o repositório se faltar (ou o reativa), com a chave
     * aprovada pelo usuário, e então instala o pacote.
     */
    async function installFromRepo(app) {
      const ok = await confirm({
        title: t("Instalar {name}?", { name: app.name }),
        text: t("O pacote vem do repositório oficial do fabricante ({repo}). Se ele ainda não estiver no sistema, o Vega o adiciona e mostra a chave de assinatura para você aprovar.", { repo: app.repo.url }),
        ic: "download",
        confirmLabel: t("Instalar"),
        required: true,
      });
      if (!ok) return;
      let current;
      try {
        current = await call("software_repos");
      } catch (error) {
        return fail(error, t("Não foi possível concluir a operação"));
      }
      const existing = current.find((repo) => repo.name === app.repo.name);
      if (!existing) {
        if (!(await addRepo(app.repo.name, app.repo.url))) return;
      } else if (!existing.enabled) {
        try {
          await call("software_set_repo_enabled", { repo: existing.name, enabled: true });
        } catch (error) {
          return fail(error, t("Não foi possível concluir a operação"));
        }
      }
      await transact({ kind: "install", origin: app.origin, id: app.id }, t("{verb} {name}", { verb: t("Instalar"), name: app.name }));
    }

    async function runSearch() {
      const query = memory.query.trim();
      if (query.length < 2) {
        searchStatus = t("Digite ao menos dois caracteres para buscar");
        memory.results = null;
        return paint();
      }
      searching = true;
      searchStatus = "";
      paintToolbar();
      paint();
      try {
        memory.results = await call("software_search", { query });
      } catch (error) {
        memory.results = null;
        searchStatus = t("Falha: {message}", { message: error.message });
      }
      searching = false;
      paintToolbar();
      paint();
    }

    // ---------- Barra de ferramentas ----------
    const tabs = segmented(
      [
        { id: "popular", label: t("Populares"), icon: "star" },
        { id: "developers", label: t("Desenvolvedores"), icon: "terminal" },
        { id: "network", label: t("Redes e TI"), icon: "server" },
        { id: "windows", label: t("Apps do Windows"), icon: "layers" },
        { id: "search", label: t("Buscar"), icon: "search" },
        { id: "installed", label: t("Instalados"), icon: "box" },
        { id: "updates", label: t("Atualizações"), icon: "download" },
      ],
      tab,
      (id) => { tab = memory.tab = id; loadError = null; paintToolbar(); load(); paint(); },
    );

    const viewToggle = () => h("div", { class: "seg sw-view", role: "radiogroup", "aria-label": t("Visualização em lista") },
      [["list", "logs", t("Visualização em lista")], ["cards", "grid", t("Visualização em cartões")]].map(([id, ic, label]) =>
        h("button", { role: "radio", "aria-selected": String(view === id), "aria-checked": String(view === id), title: label, "aria-label": label, onclick: () => { view = id; local.softwareView = id; saveLocal(); paintToolbar(); paint(); } }, icon(ic, 15))));

    function paintToolbar() {
      const parts = [];
      if (tab === "search") {
        const input = h("input", { class: "input", type: "search", placeholder: t("Buscar aplicativos e pacotes"), "aria-label": t("Buscar aplicativos e pacotes"), value: memory.query, disabled: searching });
        input.addEventListener("input", () => (memory.query = input.value));
        input.addEventListener("keydown", (e) => { if (e.key === "Enter") runSearch(); });
        parts.push(
          h("label", { class: "input-icon sw-search" }, icon("search", 15), input),
          button(searching ? t("Buscando…") : t("Buscar"), { kind: "primary", disabled: searching, onClick: runSearch }),
          h("span", { class: "spacer" }),
          memory.queue.size > 0 && button(t("Instalar ({count})", { count: memory.queue.size }), { kind: "primary", icon: "download", onClick: installQueue }),
          button(t("Limpar cache"), { icon: "trash", onClick: clearCache }),
          viewToggle());
        if (!memory.query) requestAnimationFrame(() => input.focus());
      } else if (tab === "installed") {
        const input = h("input", { class: "input", type: "search", placeholder: t("Buscar"), "aria-label": t("Buscar"), value: installedFilter });
        input.addEventListener("input", () => { installedFilter = input.value; paint(); });
        parts.push(h("label", { class: "input-icon sw-search" }, icon("search", 15), input), h("span", { class: "spacer" }),
          installed && h("span", { class: "small faint" }, String(installed.length)), viewToggle());
      } else if (tab === "updates") {
        const busy = runningTasks().some((task) => task.kind === "software");
        parts.push(
          h("span", { class: "sw-count" }, updates == null ? t("Verificando atualizações…") : updates.length ? t("{count} pacote(s) pendente(s)", { count: updates.length }) : t("Tudo em dia")),
          h("span", { class: "spacer" }),
          button(t("Atualizar tudo"), { kind: "primary", icon: "download", disabled: busy || !updates?.length, onClick: updateAll }),
          viewToggle());
      }
      clear(toolbarHost, parts.length ? h("div", { class: "toolbar" }, parts) : null);
    }

    function paint() {
      clear(body, SHOWCASES[tab] ? drawShowcase(SHOWCASES[tab]) : tab === "search" ? drawSearch() : tab === "installed" ? drawInstalled() : drawUpdates());
    }

    async function load() {
      const current = tab;
      try {
        if (SHOWCASES[current] && !popularInstalled) await loadPopular();
        else if (current === "installed" && !installed) installed = await call("software_installed");
        else if (current === "updates" && !updates) updates = await call("software_updates");
        else return;
        loadError = null;
      } catch (error) {
        loadError = error.message;
      }
      if (tab === current) {
        paintToolbar();
        paint();
      }
    }

    // Progresso por pacote e estado dos botões acompanham as transações.
    let lastRunning = runningTasks().length;
    ctx.cleanup(onTasks(() => {
      const now = runningTasks().length;
      if (now || now !== lastRunning) paint();
      if (now !== lastRunning) paintToolbar();
      lastRunning = now;
    }));

    paintToolbar();
    paint();
    load();

    const reload = () => {
      if (tab === "installed") installed = null;
      if (tab === "updates") updates = null;
      if (reposHost) refreshRepos();
      // O Flatpak pode ter sido habilitado agora (painel acima).
      popularInstalled = null;
      flatpakEnabled = null;
      if (tab === "search" && memory.query) runSearch();
      paintToolbar();
      paint();
      load();
      if (tab === "updates") refreshUpdates();
    };

    const el = h("div", { class: "sw-page" },
      h("div", { class: "page-hero" },
        h("div", {}, h("h2", {}, t("Software")), h("p", {}, t("Busque, instale e atualize aplicativos do sistema e do Flathub."))),
        button(t("Repositórios"), { icon: "database", onClick: openRepos }),
        tabs),
      preparationPanel(ctx),
      flatpakPanel(() => reload()),
      panelHost,
      toolbarHost,
      body);
    // "Repositórios do sistema" na busca do Vega abre a janela.
    if (ctx.sub === "repos") requestAnimationFrame(openRepos);
    return { el, actions: [h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: reload }, icon("refresh", 17))] };
  },
};
