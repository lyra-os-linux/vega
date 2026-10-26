// Software: buscar, instalados, atualizações e repositórios (ui/software.rs e
// application.rs do vega-gtk). Toda ação vira transação do vegad, acompanhada
// pelo painel de transação e pelo botão de tarefas do cabeçalho.

import { call } from "../api.js";
import { refreshUpdates } from "../health.js";
import { N_, t } from "../i18n.js";
import { local, saveLocal } from "../store.js";
import { runSoftware, runningTasks, onTasks, transactionPanel } from "../tasks.js";
import { h, icon, clear, button, badge, segmented, list, section, empty, skeletonRows, confirm, sheet, toast, fail, tile, normalize, meter } from "../ui.js";
import { preparationPanel } from "./software/preparation.js";
import { flatpakPanel } from "./software/flatpak.js";

const TABS = ["search", "installed", "updates", "repos"];
const ORIGIN_RANK = { official: 0, flathub: 1 };

// Estado que sobrevive à troca de página, como os widgets do vega-gtk.
const memory = {
  tab: "search",
  query: "",
  results: null,
  queue: new Map(), // "origem\u0000id" → pacote
  chosenOrigin: new Map(), // nome em minúsculas → origem escolhida
  lastTask: null,
};

const queueKey = (pkg) => `${pkg.origin}\u0000${pkg.id}`;
const originLabel = (origin) => (origin === "official" ? t("Oficial") : origin === "flathub" ? "Flathub" : origin);
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
  keywords: ["pacotes", "zypper", "flatpak", "programas", "aplicativos", "instalar", "remover", "atualizar", "repositorio"],
  sections: [
    { id: "search", label: N_("Buscar aplicativos e pacotes"), icon: "search", keywords: ["buscar", "instalar", "flathub"] },
    { id: "installed", label: N_("Instalados"), icon: "box", keywords: ["remover", "desinstalar"] },
    { id: "updates", label: N_("Atualizações"), icon: "download", keywords: ["update", "atualizar"] },
    { id: "repos", label: N_("Repositórios do sistema"), icon: "database", keywords: ["repo", "obs", "packman", "chave"] },
  ],

  render(ctx) {
    if (TABS.includes(ctx.sub)) memory.tab = ctx.sub;
    let tab = memory.tab;
    let view = local.softwareView === "cards" ? "cards" : "list";
    let installedFilter = "";
    let installed = null;
    let updates = null;
    let repos = null;
    let loadError = null;
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

    async function updateAll() {
      const ok = await confirm({
        title: t("Atualizar tudo?"),
        text: t("Executar a atualização completa do sistema e dos Flatpaks agora?"),
        ic: "download",
        confirmLabel: t("Atualizar"),
        required: false,
      });
      if (ok) await transact({ kind: "update-all" }, t("Atualizar tudo"));
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
      refreshUpdates();
      paint();
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
      if (loadError) return empty("alert-circle", t("Falha: {message}", { message: loadError }));
      if (!repos) return skeletonRows(5);
      const name = h("input", { class: "input", placeholder: t("nome"), "aria-label": t("nome") });
      const url = h("input", { class: "input", placeholder: "https://…", "aria-label": t("URL"), inputmode: "url" });
      const add = button(t("Adicionar repositório"), { kind: "primary", icon: "plus", disabled: true, onClick: () => addRepo(name.value.trim(), url.value.trim()) });
      const sync = () => (add.disabled = !name.value.trim() || !url.value.trim());
      name.addEventListener("input", sync);
      url.addEventListener("input", sync);
      url.addEventListener("keydown", (e) => { if (e.key === "Enter" && !add.disabled) add.click(); });
      return h("div", {},
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
      repos = null;
      load();
    }

    async function addRepo(name, url) {
      let started;
      try {
        started = await runSoftware({ kind: "add-repo", name, url }, t("Adicionando {name}…", { name }));
      } catch (error) {
        return fail(error, t("Não foi possível concluir a operação"));
      }
      showTask(started.task);
      const result = await started.done;
      const key = started.task.keyPending;
      if (!result.success && key) {
        toast(t("Repositório adicionado — assinatura pendente de aprovação"), { ic: "key" });
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
        if (trust) await transact({ kind: "trust-repo-key", repo: key.repo, key_id: key.key_id }, t("Confiando na chave…"));
      } else if (!result.success) {
        fail(result.message, t("Não foi possível concluir a operação"));
      }
      repos = null;
      load();
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
        { id: "search", label: t("Buscar"), icon: "search" },
        { id: "installed", label: t("Instalados"), icon: "box" },
        { id: "updates", label: t("Atualizações"), icon: "download" },
        { id: "repos", label: t("Repositórios"), icon: "database" },
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
      clear(body, tab === "search" ? drawSearch() : tab === "installed" ? drawInstalled() : tab === "updates" ? drawUpdates() : drawRepos());
    }

    async function load() {
      const current = tab;
      try {
        if (current === "installed" && !installed) installed = await call("software_installed");
        else if (current === "updates" && !updates) updates = await call("software_updates");
        else if (current === "repos" && !repos) repos = await call("software_repos");
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
      if (tab !== "repos" && (now || now !== lastRunning)) paint();
      if (now !== lastRunning) paintToolbar();
      lastRunning = now;
    }));

    paintToolbar();
    paint();
    load();

    const reload = () => {
      if (tab === "installed") installed = null;
      if (tab === "updates") updates = null;
      if (tab === "repos") repos = null;
      if (tab === "search" && memory.query) runSearch();
      paintToolbar();
      paint();
      load();
      if (tab === "updates") refreshUpdates();
    };

    const el = h("div", { class: "sw-page" },
      h("div", { class: "page-hero" },
        h("div", {}, h("h2", {}, t("Software")), h("p", {}, t("Busque, instale e atualize aplicativos do sistema e do Flathub."))),
        tabs),
      preparationPanel(ctx),
      flatpakPanel(() => reload()),
      panelHost,
      toolbarHost,
      body);
    return { el, actions: [h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: reload }, icon("refresh", 17))] };
  },
};
