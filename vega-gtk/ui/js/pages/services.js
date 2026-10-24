import { call } from "../api.js";
import { N_, t } from "../i18n.js";
import { h, icon, clear, badge, button, segmented, empty, skeletonRows, confirm, fail, normalize } from "../ui.js";

// Ações sobre um serviço, com os mesmos textos e estilos do vega-gtk.
function describe(action, service) {
  if (action === "enable") {
    return service.enabled
      ? { verb: t("Desabilitar"), detail: t("deixará de iniciar automaticamente e será parado"), suggested: false }
      : { verb: t("Habilitar"), detail: t("iniciará agora e automaticamente com o sistema"), suggested: true };
  }
  if (action === "running") {
    return service.active
      ? { verb: t("Parar"), detail: t("será interrompido até uma nova inicialização"), suggested: false }
      : { verb: t("Iniciar"), detail: t("será iniciado nesta sessão"), suggested: true };
  }
  return { verb: t("Reiniciar"), detail: t("será interrompido e iniciado novamente"), suggested: true };
}

export default {
  id: "services",
  title: N_("Serviços"),
  icon: "cog",
  group: "system",
  keywords: ["systemd", "systemctl", "daemon", "unit", "iniciar", "parar", "habilitar"],
  sections: [{ id: "all", label: N_("Todos"), icon: "cog", keywords: ["systemd", "units"] }],

  render(ctx) {
    let all = ctx.sub === "all";
    let items = null;
    let query = "";
    let busy = false;
    let generation = 0;
    const status = h("span", { class: "small faint selectable" }, t("Carregando serviços…"));
    const results = h("div", {}, skeletonRows(6));

    async function load() {
      const mine = ++generation;
      status.textContent = t("Carregando serviços…");
      try {
        const list = await call("services_list", { all });
        if (mine !== generation || !ctx.visible()) return;
        items = list;
        status.textContent = t("{count} serviço(s)", { count: list.length });
      } catch (error) {
        if (mine !== generation) return;
        items = items ?? [];
        status.textContent = error.message;
      }
      paint();
    }

    async function act(service, action) {
      const { verb, detail, suggested } = describe(action, service);
      const ok = await confirm({
        title: t("{verb} serviço?", { verb }),
        text: t("{label} ({name}) {detail}.", { label: service.label, name: service.name, detail }),
        ic: action === "restart" ? "restart" : action === "running" ? (service.active ? "stop" : "play") : "power",
        tone: suggested ? "" : "warn",
        confirmLabel: verb,
        confirmKind: suggested ? "primary" : "danger",
        required: false,
      });
      if (!ok) return;
      busy = true;
      status.textContent = t("{verb} {label}…", { verb, label: service.label });
      paint();
      try {
        if (action === "enable") await call("services_set_enabled", { name: service.name, enabled: !service.enabled });
        else if (action === "running") await call("services_set_running", { name: service.name, running: !service.active });
        else await call("services_restart", { name: service.name });
        busy = false;
        await load();
      } catch (error) {
        busy = false;
        status.textContent = error.message;
        fail(error);
        paint();
      }
    }

    function serviceRow(service) {
      const state = [service.active ? t("Ativo") : t("Parado"), service.enabled ? t("Habilitado") : t("Desabilitado")];
      const can = service.available && !busy;
      const leadTone = !service.available ? "" : service.active ? "green" : service.enabled ? "red" : "";
      return h("div", { class: `list-row svc-row ${service.available ? "" : "svc-unavailable"}`.trim() },
        h("span", { class: `lead ${leadTone}`.trim() }, icon(service.active ? "play" : service.enabled ? "alert" : "stop", 16)),
        h("div", { class: "body" },
          h("div", { class: "title" }, service.label || service.name),
          h("div", { class: "subtitle selectable" }, h("span", { class: "mono" }, service.name), service.description ? ` • ${service.description}` : "")),
        h("div", { class: "suffix svc-suffix" },
          h("span", { class: "svc-badges" },
            badge(state[0], service.active ? "green" : service.enabled ? "red" : ""),
            badge(state[1], "outline")),
          h("span", { class: "svc-actions" },
            button(service.enabled ? t("Desabilitar") : t("Habilitar"), { size: "sm", disabled: !can, onClick: () => act(service, "enable") }),
            button(service.active ? t("Parar") : t("Iniciar"), { size: "sm", kind: service.active ? "danger-soft" : "", disabled: !can, onClick: () => act(service, "running") }),
            button(t("Reiniciar"), { size: "sm", disabled: !(can && service.active), onClick: () => act(service, "restart") }))));
    }

    function paint() {
      if (items == null) return;
      const q = normalize(query.trim());
      const visible = items
        .filter((s) => !q || normalize(`${s.label} ${s.name} ${s.description}`).includes(q))
        // Habilitados e parados primeiro: são os que precisam de atenção.
        .sort((a, b) => (b.enabled && !b.active && b.available) - (a.enabled && !a.active && a.available) || (a.label || a.name).localeCompare(b.label || b.name));
      clear(results, visible.length === 0
        ? empty(q ? "search" : "cog", q ? t("Nenhum resultado encontrado") : t("Serviços"), q ? null : status.textContent)
        : h("div", { class: "list" }, visible.slice(0, 300).map(serviceRow)));
    }

    const search = h("input", { class: "input", placeholder: t("Buscar"), "aria-label": t("Buscar") });
    search.addEventListener("input", () => { query = search.value; paint(); });
    const tabs = segmented([{ id: "main", label: t("Principais") }, { id: "all", label: t("Todos") }], all ? "all" : "main", (id) => {
      all = id === "all";
      items = null;
      clear(results, skeletonRows(6));
      load();
    });

    load();
    return {
      el: h("div", {},
        h("div", { class: "page-hero" }, h("div", {}, h("h2", {}, t("Serviços")), h("p", {}, t("Units do systemd gerenciadas pelo vegad")))),
        h("div", { class: "toolbar" }, h("label", { class: "input-icon" }, icon("search", 15), search), tabs, h("span", { class: "spacer" }), status),
        results),
      actions: [h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: () => load() }, icon("refresh", 17))],
    };
  },
};
