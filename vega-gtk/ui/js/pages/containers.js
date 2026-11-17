// Serviços em contêiner (vega#186): Podman sem root na conta do usuário, cada
// serviço como Quadlet do systemd do usuário (backend/containers.rs). Só os
// serviços do catálogo do Vega; a página manda apenas o id. Primeiro serviço:
// Oracle AI Database Free (vega#190); depois, Microsoft SQL Server (vega#284).

import { call } from "../api.js";
import { N_, t } from "../i18n.js";
import { runSoftware, runContainerInstall } from "../tasks.js";
import { h, icon, clear, button, badge, banner, empty, confirm, toast, fail, section, skeletonRows } from "../ui.js";

const POLL_MS = 5000;

/** Estado mostrado: systemd (state) + saúde do contêiner (health). */
function stateOf(service) {
  if (!service.installed) return { label: t("Não instalado"), tone: "", icon: "download" };
  if (service.state === "active" && service.health === "healthy") return { label: t("Pronto para uso"), tone: "green", icon: "play" };
  if (service.state === "active" || service.state === "activating") return { label: t("Iniciando (a primeira vez cria o banco e leva alguns minutos)"), tone: "yellow", icon: "refresh" };
  if (service.state === "failed") return { label: t("Falhou; veja journalctl --user -u {unit}", { unit: `lyra-${service.id}` }), tone: "red", icon: "alert-circle" };
  return { label: t("Parado"), tone: "", icon: "power" };
}

export default {
  id: "containers",
  title: N_("Contêineres"),
  icon: "box",
  group: "main",
  keywords: ["podman", "container", "conteiner", "oracle", "sql server", "mssql", "banco", "database", "sap", "quadlet"],

  render(ctx) {
    let services = null;
    let busy = false;
    const body = h("div", {}, skeletonRows(2));

    async function load() {
      try {
        services = await call("containers_list");
      } catch (error) {
        clear(body, empty("alert-circle", t("Falha: {message}", { message: String(error) })));
        return;
      }
      paint();
    }

    async function installPodman() {
      const started = await runSoftware({ kind: "install", origin: "official", id: "podman" }, t("Instalando {name}…", { name: "Podman" }));
      const result = await started.done;
      if (!result.success) fail(result.message, t("Não foi possível concluir a operação"));
      return result.success;
    }

    async function install(service) {
      const ramOk = service.ram_mib >= service.min_ram_mib;
      const diskOk = service.free_disk_gib >= service.min_disk_gib;
      if (!ramOk || !diskOk) {
        return fail(t("{name} precisa de {ram} GB de memória e {disk} GB livres na pasta pessoal. Este computador tem {haveRam} GB de memória e {haveDisk} GB livres.", {
          name: service.name, ram: Math.ceil(service.min_ram_mib / 1024), disk: service.min_disk_gib,
          haveRam: Math.floor(service.ram_mib / 1024), haveDisk: service.free_disk_gib,
        }), t("Requisitos não atendidos"));
      }
      // Licença do fabricante (EULA do SQL Server): aceite explícito, nunca implícito.
      const accept = service.license && h("input", { type: "checkbox", id: "ct-license" });
      const ok = await confirm({
        title: t("Instalar {name}?", { name: service.name }),
        text: t("O Vega baixa a imagem oficial ({image}) com o Podman, na sua conta, sem senha, e cria o serviço. O download tem alguns GB; se falhar, tente de novo que o Podman reaproveita o que já baixou."
          , { image: service.image })
          + (service.podman ? "" : " " + t("Antes, instala o Podman, com a senha de administrador.")),
        facts: [
          { icon: "key", text: t("A senha do administrador ({users}) é gerada pelo Vega e fica só na sua conta; copie-a aqui depois.", { users: service.users }) },
          { icon: "database", text: t("Conexão: {connection}, só deste computador.", { connection: service.connection }) },
          { icon: "power", text: t("O serviço não liga sozinho com o computador; use Iniciar.") },
        ],
        extra: accept && h("div", {},
          h("label", { class: "row-flex", for: "ct-license" }, accept, t("Li e aceito a licença do fabricante")),
          h("button", { class: "btn flat sm", onclick: () => call("open_uri", { uri: service.license.url }).catch(fail) }, icon("external", 14), t("Ler os termos da licença"))),
        ic: "box",
        confirmLabel: t("Instalar"),
        required: true,
      });
      if (!ok) return;
      if (accept && !accept.checked) return fail(t("É preciso aceitar a licença para instalar."), t("Instalação cancelada"));
      busy = true;
      paint();
      try {
        if (!service.podman && !(await installPodman())) return;
        const started = await runContainerInstall(service.id, t("Baixando {name}…", { name: service.name }), Boolean(accept?.checked));
        const result = await started.done;
        if (!result.success) fail(result.message, t("Não foi possível concluir a operação"));
        else toast(t("{name} instalado. Use Iniciar para ligar o banco.", { name: service.name }));
      } catch (error) {
        fail(error, t("Não foi possível concluir a operação"));
      } finally {
        busy = false;
        load();
      }
    }

    async function action(service, verb) {
      try {
        await call("containers_action", { id: service.id, action: verb });
      } catch (error) {
        return fail(error, t("Não foi possível concluir a operação"));
      }
      setTimeout(load, 800);
    }

    async function copyPassword(service) {
      try {
        await navigator.clipboard.writeText(await call("containers_password", { id: service.id }));
        toast(t("Senha copiada"));
      } catch (error) {
        fail(error, t("Não foi possível concluir a operação"));
      }
    }

    async function remove(service) {
      const keep = h("input", { type: "checkbox", checked: true, id: "ct-keep" });
      const ok = await confirm({
        title: t("Remover {name}?", { name: service.name }),
        text: t("O serviço para e a imagem sai do computador, sem senha."),
        extra: h("label", { class: "row-flex", for: "ct-keep" }, keep, t("Manter os dados (o banco volta igual se instalar de novo)")),
        ic: "trash",
        tone: "danger",
        confirmLabel: t("Remover"),
        confirmKind: "danger",
        required: true,
      });
      if (!ok) return;
      try {
        await call("containers_remove", { id: service.id, keepData: keep.checked });
      } catch (error) {
        return fail(error, t("Não foi possível concluir a operação"));
      }
      load();
    }

    function row(service) {
      const state = stateOf(service);
      const running = service.state === "active" || service.state === "activating";
      const actions = !service.installed
        ? [button(t("Instalar"), { kind: "primary", size: "sm", icon: "download", disabled: busy, onClick: () => install(service) })]
        : [
          running
            ? button(t("Parar"), { size: "sm", icon: "stop", onClick: () => action(service, "stop") })
            : button(t("Iniciar"), { kind: "primary", size: "sm", icon: "play", onClick: () => action(service, "start") }),
          button(t("Copiar senha"), { size: "sm", icon: "key", onClick: () => copyPassword(service) }),
          button(t("Remover"), { size: "sm", icon: "trash", kind: "danger", disabled: running, title: running ? t("Pare o serviço antes de remover") : undefined, onClick: () => remove(service) }),
        ];
      return h("div", { class: "card" },
        h("div", { class: "row-flex" }, icon("database", 22), h("strong", {}, service.name), h("div", { class: "spacer" }), badge(state.label, state.tone, state.icon)),
        h("p", { class: "small muted selectable" }, service.image),
        service.installed && h("p", { class: "small selectable" }, t("Conexão: {connection} (usuário {users})", { connection: service.connection, users: service.users })),
        service.installed && h("p", { class: "small muted" }, t("Terminal:"), " ", h("code", { class: "selectable" }, service.terminal)),
        h("div", { class: "row-flex" }, actions));
    }

    function paint() {
      if (!services) return;
      clear(body,
        !services[0]?.podman && banner("info", "box", t("O Podman ainda não está instalado"), t("Ele é instalado junto com o primeiro serviço, com a senha de administrador.")),
        section(t("Serviços disponíveis"), { sub: String(services.length) }, h("div", { class: "grid auto" }, services.map(row))));
    }

    load();
    const timer = setInterval(() => { if (ctx.visible() && !busy) load(); }, POLL_MS);
    ctx.cleanup(() => clearInterval(timer));

    return {
      el: h("div", {},
        h("div", { class: "page-hero" }, h("div", {}, h("h2", {}, t("Contêineres")), h("p", {}, t("Serviços de estudo e desenvolvimento em contêiner, com o Podman na sua conta, sem root")))),
        body),
    };
  },
};
