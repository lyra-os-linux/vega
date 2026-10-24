// Máquinas virtuais (libvirt, sem vegad). Paridade com a página do vega-gtk:
// conexões pessoal/sistema, busca, atualização a cada 5 s só com a página
// visível, ciclo de vida, atalho e tela no Lyra VMs, edição offline, criação
// com progresso e cancelamento, e remoção com plano de arquivos revisado.

import { on, pick, vms } from "../api.js";
import { N_, t, locale } from "../i18n.js";
import { localTask } from "../tasks.js";
import {
  h, icon, clear, button, badge, segmented, banner, empty, confirm, toast, sheet, popover, menuItem,
  meter, normalize, skeletonRows,
} from "../ui.js";

const POLL_MS = 5000;
const GIB = 2 ** 30;

const STATE = {
  running: { label: N_("Em execução"), tone: "green", icon: "play" },
  paused: { label: N_("Pausada"), tone: "yellow", icon: "pause" },
  off: { label: N_("Desligada"), tone: "", icon: "power" },
  other: { label: N_("Indisponível ou em transição"), tone: "", icon: "alert-circle" },
};

const ACTIONS = {
  start: { label: N_("Iniciar"), icon: "play" },
  shutdown: { label: N_("Desligar"), icon: "power" },
  pause: { label: N_("Pausar"), icon: "pause" },
  resume: { label: N_("Retomar"), icon: "play" },
  "force-stop": { label: N_("Forçar parada"), icon: "stop" },
  remove: { label: N_("Remover máquina"), icon: "trash" },
};

const gib = (bytes) => new Intl.NumberFormat(locale, { minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(bytes / GIB);

/** Campo numérico com limites, como os SpinButton do vega-gtk. */
function numberInput(value, min, max, step, label) {
  const input = h("input", { class: "input vm-number", type: "number", min: String(min), max: String(max), step: String(step), "aria-label": label });
  input.value = String(value);
  input.read = () => Math.min(max, Math.max(min, Math.round(Number(input.value) || min)));
  input.addEventListener("change", () => (input.value = String(input.read())));
  return input;
}

function field(label, control) {
  return h("div", { class: "list-row vm-field" }, h("div", { class: "body" }, h("div", { class: "title" }, label)), h("div", { class: "suffix" }, control));
}

export default {
  id: "virtualization",
  title: N_("Máquinas virtuais"),
  icon: "monitor",
  group: "main",
  keywords: ["vm", "virt", "libvirt", "qemu", "kvm", "maquina", "virtual", "iso"],

  render(ctx) {
    const state = {
      connection: "personal",
      machines: [],
      loaded: false,
      busy: false,
      pollEnabled: true,
      query: "",
      error: null, // { title?, text } — banner persistente, como o rótulo de status do GTK
      creating: null, // { done, total, cancelling }
    };

    const statusHost = h("div", { class: "vm-status" });
    const createHost = h("div");
    const listHost = h("div", {}, skeletonRows(3));

    // ---------- Barra de ferramentas ----------

    const connectionSwitch = segmented(
      [{ id: "personal", label: t("Pessoais"), icon: "user" }, { id: "system", label: t("Sistema"), icon: "server" }],
      state.connection,
      (id) => {
        if (state.busy) {
          connectionSwitch.select(state.connection);
          return;
        }
        state.connection = id;
        state.machines = [];
        state.loaded = false;
        state.error = null;
        paint();
        load();
      },
    );
    connectionSwitch.setAttribute("aria-label", t("Conexão das máquinas virtuais"));

    const search = h("input", { class: "input", placeholder: t("Buscar máquina virtual…"), "aria-label": t("Buscar máquina virtual") });
    search.addEventListener("input", () => {
      state.query = search.value;
      paintList();
    });

    const refreshBtn = h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: () => load() }, icon("refresh", 17));
    const createBtn = button(t("Nova máquina"), { kind: "primary", icon: "plus", onClick: () => createDialog() });

    // ---------- Estado ----------

    function setBusy(busy) {
      state.busy = busy;
      paintToolbar();
      listHost.classList.toggle("vm-busy", busy);
      listHost.querySelectorAll("button").forEach((b) => (b.disabled = busy || b.dataset.off === "1"));
    }

    function paintToolbar() {
      refreshBtn.disabled = state.busy;
      createBtn.disabled = state.busy || state.connection !== "personal";
      createBtn.title = state.connection === "personal" ? "" : t("Pessoais");
      connectionSwitch.querySelectorAll("button").forEach((b) => (b.disabled = state.busy));
    }

    function showError(text) {
      state.error = { text };
      paintStatus();
    }

    function paintStatus() {
      if (!state.error) return clear(statusHost);
      clear(statusHost, h("div", { class: "banner danger vm-banner" }, icon("alert-circle", 22),
        h("div", { class: "body" }, state.error.title && h("strong", {}, state.error.title), h("span", { class: "selectable vm-pre" }, state.error.text)),
        h("button", { class: "icon-btn sm", "aria-label": t("Fechar"), onclick: () => { state.error = null; paintStatus(); } }, icon("x", 15))));
    }

    // Falha de operação: mostra o erro e pausa a atualização automática.
    function operationFailed(error) {
      state.pollEnabled = false;
      showError(error?.message ?? String(error));
    }

    // ---------- Carregamento ----------

    async function load() {
      if (state.busy) return;
      const connection = state.connection;
      setBusy(true);
      try {
        const machines = await vms("list", { connection });
        if (connection !== state.connection) return;
        state.pollEnabled = true;
        state.error = null;
        const changed = JSON.stringify(machines) !== JSON.stringify(state.machines) || !state.loaded;
        state.machines = machines;
        state.loaded = true;
        paintStatus();
        if (changed) paintList();
      } catch (error) {
        if (connection !== state.connection) return;
        state.pollEnabled = false;
        state.machines = [];
        state.loaded = true;
        state.error = { title: t("Não foi possível acessar as máquinas virtuais. Confira o serviço libvirt e tente novamente."), text: error.message };
        paintStatus();
        paintList();
      } finally {
        setBusy(false);
      }
    }

    /** Executa uma operação na conexão capturada no clique e recarrega. */
    async function run(connection, op, args) {
      if (state.busy || connection !== state.connection) return false;
      setBusy(true);
      toast(t("Executando operação…"), { ic: "clock", timeout: 2500 });
      try {
        await vms(op, { connection, ...args });
        return true;
      } catch (error) {
        operationFailed(error);
        return false;
      } finally {
        setBusy(false);
        if (connection === state.connection) load();
      }
    }

    // ---------- Ações ----------

    async function act(machine, action) {
      const connection = state.connection;
      if (action === "remove") return confirmRemoval(machine);
      if (action === "shutdown" || action === "force-stop") {
        const ok = await confirm({
          title: machine.name,
          text: action === "force-stop" ? t("Interromper imediatamente? Dados não salvos podem ser perdidos.") : t("Solicitar que o sistema convidado desligue?"),
          ic: action === "force-stop" ? "stop" : "power",
          tone: "danger",
          confirmLabel: t("Confirmar"),
          confirmKind: "danger",
        });
        // Nenhuma ação é redirecionada se a conexão mudou durante o diálogo.
        if (!ok || connection !== state.connection) return;
      }
      run(connection, "act", { uuid: machine.uuid, action });
    }

    async function openConsole(machine) {
      try {
        await vms("open-console", { connection: state.connection, uuid: machine.uuid });
      } catch (error) {
        showError(error.message);
      }
    }

    async function createShortcut(machine) {
      const connection = state.connection;
      if (state.busy) return;
      setBusy(true);
      try {
        await vms("shortcut", { connection, uuid: machine.uuid, name: machine.name });
        toast(t("Atalho criado no menu de aplicativos."));
      } catch (error) {
        operationFailed(error);
      } finally {
        setBusy(false);
      }
    }

    async function confirmRemoval(machine) {
      const connection = state.connection;
      if (state.busy) return;
      setBusy(true);
      let plan = null;
      let planError = null;
      try {
        plan = await vms("removal-plan", { connection, uuid: machine.uuid });
      } catch (error) {
        planError = error.message;
      } finally {
        setBusy(false);
      }
      if (connection !== state.connection) return;

      const erase = h("input", { type: "checkbox", class: "check", id: "vm-erase" });
      const canErase = Boolean(plan?.files?.length);
      erase.disabled = !canErase;
      const extra = h("div", { class: "vm-removal" },
        planError && h("div", { class: "banner warn" }, icon("alert", 20), h("div", { class: "body" }, h("span", { class: "selectable vm-pre" }, planError))),
        h("label", { class: `vm-check ${canErase ? "" : "faint"}`.trim(), for: "vm-erase" }, erase, h("span", {}, t("Apagar também os discos e arquivos locais listados abaixo"))),
        canErase && h("ul", { class: "vm-files mono selectable" }, plan.files.map((file) => h("li", {}, file))),
        h("p", { class: "small faint" }, t("A exclusão é permanente. Discos compartilhados e mídias externas não serão apagados.")));
      const ok = await confirm({
        title: machine.name,
        text: t("Remover esta máquina? Por padrão, os discos e arquivos locais serão preservados."),
        ic: "trash",
        tone: "danger",
        extra,
        confirmLabel: t("Remover"),
        confirmKind: "danger",
      });
      if (!ok || connection !== state.connection) return;
      if (erase.checked && canErase) run(connection, "remove-with-files", { uuid: machine.uuid });
      else run(connection, "act", { uuid: machine.uuid, action: "remove" });
    }

    // ---------- Edição ----------

    async function editDialog(machine) {
      const connection = state.connection;
      if (state.busy) return;
      setBusy(true);
      let details;
      try {
        details = await vms("edit-details", { connection, uuid: machine.uuid });
      } catch (error) {
        operationFailed(error);
        return;
      } finally {
        setBusy(false);
      }
      if (connection !== state.connection) return;

      let close;
      const apply = (op, args) => {
        close();
        run(connection, op, { uuid: machine.uuid, ...args });
      };

      const name = h("input", { class: "input", "aria-label": t("Nome da máquina") });
      name.value = machine.name;
      name.style.flex = "1";
      const cpu = numberInput(machine.cpus, 1, 64, 1, t("CPUs virtuais"));
      const ram = numberInput(machine.memory_mib, 512, 262144, 512, t("Memória (MiB)"));

      const disks = details.disks.map((disk) => {
        const title = `${t("Disco")} · ${disk.target}`;
        if (disk.capacity == null) {
          return h("div", { class: "list-row" }, h("span", { class: "lead" }, icon("disk", 18)),
            h("div", { class: "body" }, h("div", { class: "title" }, title), h("div", { class: "subtitle wrap" }, disk.unavailable ?? "")));
        }
        const minimum = Math.floor(disk.capacity / GIB) + 1;
        const size = numberInput(Math.min(minimum, 2048), Math.min(minimum, 2048), 2048, 1, t("Nova capacidade (GiB)"));
        const grow = button(t("Ampliar"), {
          size: "sm",
          disabled: minimum > 2048,
          onClick: async () => {
            const target = size.read();
            const ok = await confirm({
              title: t("Ampliar disco?"),
              text: `${gib(disk.capacity)} GiB → ${target} GiB`,
              ic: "disk",
              extra: h("p", { class: "small muted" }, t("O disco não poderá ser reduzido pelo Vega. Depois, expanda a partição dentro do sistema convidado para usar o novo espaço.")),
              confirmLabel: t("Ampliar"),
            });
            if (ok) apply("grow-disk", { target: disk.target, gib: target });
          },
        });
        return h("div", { class: "list-row" }, h("span", { class: "lead" }, icon("disk", 18)),
          h("div", { class: "body" }, h("div", { class: "title" }, title), h("div", { class: "subtitle mono" }, `${gib(disk.capacity)} GiB`)),
          h("div", { class: "suffix" }, size, grow));
      });

      const media = details.media.map((target) =>
        h("div", { class: "list-row" }, h("span", { class: "lead" }, icon("disk", 18)),
          h("div", { class: "body" }, h("div", { class: "title" }, `${t("Leitor de ISO")} · ${target}`), h("div", { class: "subtitle wrap" }, t("Ejetar a mídia preserva o arquivo ISO no disco."))),
          h("div", { class: "suffix" }, button(t("Ejetar ISO"), { size: "sm", icon: "arrow-up", onClick: () => apply("eject-iso", { target }) }))));

      close = sheet(machine.name, [
        h("p", { class: "muted small" }, t("A máquina deve estar desligada. Cada botão aplica apenas a alteração indicada.")),
        h("div", { class: "pref-group" }, h("h4", {}, t("Nome da máquina")),
          h("div", { class: "row-flex vm-rename" }, name, button(t("Renomear"), { onClick: () => apply("rename", { name: name.value }) }))),
        h("div", { class: "pref-group" }, h("h4", {}, `${t("CPUs virtuais")} · ${t("Memória (MiB)")}`),
          h("div", { class: "list" }, field(t("CPUs virtuais"), cpu), field(t("Memória (MiB)"), ram)),
          h("div", { class: "row-flex vm-actions-end" }, button(t("Aplicar CPU e memória"), { kind: "primary", onClick: () => apply("configure", { cpus: cpu.read(), memory_mib: ram.read() }) }))),
        (disks.length || media.length) && h("div", { class: "pref-group" }, h("h4", {}, t("Disco")), h("div", { class: "list" }, disks, media)),
        h("div", { class: "row-flex vm-actions-end" }, button(t("Fechar"), { onClick: () => close() })),
      ]);
    }

    // ---------- Criação ----------

    function createDialog() {
      if (state.busy || state.connection !== "personal") return;
      const name = h("input", { class: "input", placeholder: t("Nome da máquina"), "aria-label": t("Nome da máquina") });
      name.style.width = "100%";
      const kind = h("select", { class: "input", "aria-label": t("Origem da máquina") },
        h("option", { value: "iso" }, t("Instalar por ISO")),
        h("option", { value: "qcow2" }, t("Importar QCOW2")),
        h("option", { value: "raw" }, t("Importar RAW")));
      const firmware = h("select", { class: "input", "aria-label": t("Firmware") }, h("option", { value: "bios" }, "BIOS"), h("option", { value: "uefi" }, "UEFI"));
      const cpu = numberInput(2, 1, 64, 1, t("CPUs virtuais"));
      const ram = numberInput(2048, 512, 262144, 512, t("Memória (MiB)"));
      const disk = numberInput(32, 4, 2048, 1, t("Disco (GiB)"));
      let source = null;
      const fileLabel = h("span", {}, t("Selecionar arquivo…"));
      const fileBtn = h("button", { class: "btn vm-file", onclick: async () => {
        const chosen = await pick({ title: t("Selecionar arquivo") });
        if (typeof chosen === "string" && chosen) {
          source = chosen;
          fileLabel.textContent = chosen.split("/").pop();
          fileBtn.title = chosen;
        }
      } }, icon("folder", 16), fileLabel);
      kind.addEventListener("change", () => {
        // Importações usam o tamanho do disco de origem.
        disk.disabled = kind.value !== "iso";
        source = null;
        fileLabel.textContent = t("Selecionar arquivo…");
        fileBtn.title = "";
      });

      let close;
      const submit = () => {
        if (!source) {
          toast(t("Selecione a mídia antes de criar a máquina."), { ic: "alert" });
          return;
        }
        close();
        createMachine({ name: name.value, source, kind: kind.value, firmware: firmware.value, cpus: cpu.read(), memory_mib: ram.read(), disk_gib: disk.read() });
      };
      close = sheet(t("Nova máquina pessoal"), [
        h("p", { class: "muted small" }, t("Instale por ISO ou importe uma cópia de um disco. Para importar, desligue a máquina de origem. Os arquivos originais serão preservados.")),
        h("label", { class: "pref-group" }, h("h4", {}, t("Nome da máquina")), name),
        h("div", { class: "list" },
          field(t("Origem da máquina"), kind),
          field(t("Firmware"), firmware),
          field(t("CPUs virtuais"), cpu),
          field(t("Memória (MiB)"), ram),
          field(t("Disco (GiB)"), disk)),
        fileBtn,
        h("div", { class: "row-flex vm-actions-end" }, button(t("Cancelar"), { onClick: () => close() }), button(t("Criar"), { kind: "primary", onClick: submit })),
      ], { wide: false });
      requestAnimationFrame(() => name.focus());
    }

    async function createMachine(request) {
      if (state.busy || state.connection !== "personal") return;
      setBusy(true);
      state.creating = { done: 0, total: 0, cancelling: false };
      const task = localTask(t("Nova máquina pessoal"));
      paintCreate();
      const off = on("vms-create-progress", ({ done, total }) => {
        if (!state.creating) return;
        Object.assign(state.creating, { done, total });
        task.progress(Math.round((100 * done) / Math.max(1, total)));
        paintCreate();
      });
      try {
        await vms("create", request);
        task.finish(true);
        state.creating = null;
        paintCreate();
        setBusy(false);
        load();
      } catch (error) {
        task.finish(false, error.message);
        state.creating = null;
        paintCreate();
        setBusy(false);
        operationFailed(error);
      } finally {
        off();
      }
    }

    function paintCreate() {
      const c = state.creating;
      if (!c) return clear(createHost);
      const fraction = c.total ? c.done / c.total : 0;
      clear(createHost, h("div", { class: "card job-card vm-create" },
        h("div", { class: "job-head" },
          h("span", { class: "lead" }, h("span", { class: "spinner" })),
          h("div", { class: "spacer" }, h("strong", {}, t("Nova máquina pessoal")), h("div", { class: "small muted" }, t("Criando disco e copiando a mídia… A máquina ficará desligada ao concluir."))),
          h("span", { class: "percent" }, `${Math.round(fraction * 100)}%`)),
        meter(fraction, { lg: true, warnAt: 2, critAt: 2 }),
        h("div", { class: "row-flex vm-actions-end" },
          button(t("Cancelar criação"), { kind: "danger-soft", size: "sm", disabled: c.cancelling, onClick: () => {
            c.cancelling = true;
            vms("cancel-create").catch(() => {});
            paintCreate();
          } }))));
    }

    // ---------- Lista ----------

    function machineCard(machine) {
      const st = STATE[machine.state] ?? STATE.other;
      const consoleOk = machine.state === "running" || machine.state === "paused";
      const primary = machine.actions.filter((a) => a !== "remove");
      const more = h("button", { class: "icon-btn vm-more", "aria-label": t("Mais ações"), title: t("Mais ações") }, icon("more", 17));
      more.addEventListener("click", () => {
        const pop = popover(more, [
          menuItem("plus", t("Atalho da tela"), () => { pop.close(); createShortcut(machine); }),
          machine.editable && menuItem("cog", t("Editar máquina"), () => { pop.close(); editDialog(machine); }),
          machine.actions.includes("remove") && h("hr"),
          machine.actions.includes("remove") && menuItem("trash", t(ACTIONS.remove.label), () => { pop.close(); act(machine, "remove"); }),
        ], "menu");
        pop.querySelector(".menu-item")?.setAttribute("title", t("Adiciona a tela ao menu de aplicativos. Se a máquina estiver desligada, o console aguardará o início pelo Vega."));
      });
      const consoleBtn = button(t("Abrir tela"), { kind: consoleOk ? "primary" : "", size: "sm", icon: "monitor", disabled: !consoleOk, onClick: () => openConsole(machine) });
      if (!consoleOk) consoleBtn.dataset.off = "1";
      return h("div", { class: `card vm-card vm-${machine.state}` },
        h("div", { class: "vm-head" },
          h("span", { class: `vm-avatar ${st.tone}`.trim() }, icon("monitor", 22)),
          h("div", { class: "vm-meta" },
            h("div", { class: "vm-name selectable" }, machine.name),
            h("div", { class: "small muted" }, `${machine.cpus} vCPU · ${machine.memory_mib} MiB`)),
          badge(t(st.label), st.tone, st.icon),
          more),
        h("div", { class: "vm-actions" },
          consoleBtn,
          primary.map((action) => button(t(ACTIONS[action].label), {
            size: "sm",
            icon: ACTIONS[action].icon,
            kind: action === "force-stop" ? "danger-soft" : "",
            onClick: () => act(machine, action),
          }))));
    }

    function paintList() {
      if (!state.loaded) return clear(listHost, skeletonRows(3));
      const q = normalize(state.query.trim());
      const visible = state.machines.filter((m) => !q || normalize(m.name).includes(q));
      if (!visible.length) {
        return clear(listHost, state.machines.length
          ? empty("search", t("Nenhuma máquina corresponde à busca."))
          : empty("monitor", t("Nenhuma máquina nesta conexão."), null,
              state.connection === "personal" && !state.error && button(t("Nova máquina"), { kind: "primary", icon: "plus", onClick: () => createDialog() })));
      }
      clear(listHost, h("div", { class: "vm-grid" }, visible.map(machineCard)));
      if (state.busy) setBusy(true);
    }

    function paint() {
      paintToolbar();
      paintStatus();
      paintList();
    }

    // Atualização a cada 5 s só com a página visível, sem operação em curso
    // e enquanto a última consulta não falhou.
    const timer = setInterval(() => {
      if (ctx.visible() && !state.busy && state.pollEnabled) load();
    }, POLL_MS);
    ctx.cleanup(() => clearInterval(timer));

    paint();
    load();

    const el = h("div", {},
      h("div", { class: "page-hero" },
        h("div", {}, h("h2", {}, t("Máquinas virtuais")), h("p", {}, t("Gerencie aqui e abra a tela da máquina no Lyra VMs. Fechar a janela mantém a máquina em execução."))),
        connectionSwitch),
      h("div", { class: "toolbar" }, h("label", { class: "input-icon" }, icon("search", 15), search), h("span", { class: "spacer" }), createBtn),
      statusHost,
      createHost,
      listHost);
    return { el, actions: [refreshBtn] };
  },
};
