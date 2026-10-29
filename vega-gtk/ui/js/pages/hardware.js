import { call } from "../api.js";
import { N_, t } from "../i18n.js";
import { hasCapability } from "../store.js";
import { runSoftware, transactionPanel } from "../tasks.js";
import { diagnosticsCard, diagnosticsSection } from "./hardware-diagnostics.js";
import { h, icon, clear, button, badge, segmented, row, list, section, banner, skeletonRows, confirm, toast } from "../ui.js";

// ---------- Componentes ----------

/** `lspci` traz "0000:00:02.0 VGA compatible controller [0300]: Intel … [8086:9a68] (rev 01)". */
function gpuName(raw) {
  const afterClass = raw.includes("]: ") ? raw.slice(raw.indexOf("]: ") + 3) : raw;
  return afterClass.replace(/\s*\[[0-9a-f]{4}:[0-9a-f]{4}\].*$/i, "").replace(/\s*\(rev [^)]*\)$/i, "").trim() || raw;
}

function componentCard(ic, title, values, raw) {
  return h("div", { class: "card hw-card" },
    h("div", { class: "hw-card-head" }, h("span", { class: "hw-ic" }, icon(ic, 20)), h("span", { class: "hw-label" }, title)),
    values.length
      ? values.map((value, index) => h("div", { class: `hw-value selectable ${index ? "secondary" : ""}`.trim(), title: raw?.[index] ?? value }, value))
      : h("div", { class: "hw-value faint" }, "—"));
}

function components(inventory, firmware) {
  if (inventory.error) {
    return banner("danger", "alert-circle", t("Componentes"), inventory.error);
  }
  const gpus = inventory.value.gpu.split("\n").map((s) => s.trim()).filter(Boolean);
  return h("div", { class: "grid c2 hw-grid" },
    componentCard("cpu", t("Processador"), [inventory.value.cpu]),
    componentCard("monitor", t("Vídeo"), gpus.map(gpuName), gpus),
    componentCard("layers", t("Memória"), [inventory.value.ram]),
    componentCard("shield", t("Firmware"), [firmware.error ?? firmware.value]));
}

// ---------- NVIDIA ----------

const NVIDIA_STATES = {
  "no-gpu": N_("Nenhuma GPU NVIDIA detectada."),
  "unsupported-gpu": N_("Esta GPU não está na lista de suporte do driver NVIDIA qualificado."),
  "unsupported-system": N_("Esta integração requer Lyra OS ou openSUSE Leap 16.1, em x86_64."),
  "unknown-secure-boot": N_("Não foi possível verificar o Secure Boot. A instalação está bloqueada."),
  "unknown-boot-kernel": N_("Não foi possível identificar o kernel padrão. Revise a configuração de inicialização."),
  conflict: N_("Há outro driver NVIDIA instalado. A migração precisa ser revisada antes de continuar."),
  inconsistent: N_("Os pacotes NVIDIA estão incompletos ou com versões incompatíveis. Revise a instalação antes de reiniciar."),
  "kernel-missing": N_("O módulo NVIDIA correspondente não está disponível para o kernel em uso ou para o kernel padrão."),
  "unsigned-module": N_("O módulo encontrado não corresponde ao pacote assinado pela SUSE."),
  available: N_("O driver NVIDIA qualificado está disponível para instalação opcional."),
  unmanaged: N_("Driver oficial instalado. Ative a integração Lyra para manter bibliotecas e módulo em versões correspondentes."),
  "reboot-required": N_("Driver instalado. Reinicie para carregar o módulo e conferir o funcionamento."),
  active: N_("Driver NVIDIA ativo; bibliotecas, módulo e integração Lyra correspondem."),
};
const UPDATE_VEGAD = N_("Atualize o vegad para usar a integração NVIDIA atual.");

const SECURE_BOOT = {
  enabled: N_("Habilitado"),
  disabled: N_("Desabilitado"),
  "not-applicable": N_("Não se aplica (BIOS)"),
};

const stateMessage = (state) => t(NVIDIA_STATES[state] ?? UPDATE_VEGAD);
const installable = (status) => status.supported && (status.state === "available" || status.state === "unmanaged");

/** Tom visual do estado: verde ativo, destaque quando há ação, amarelo para revisão. */
function stateTone(state) {
  if (state === "active") return "green";
  if (state === "available" || state === "unmanaged") return "tint";
  if (state === "reboot-required") return "yellow";
  if (state === "no-gpu" || state === "unsupported-gpu" || state === "unsupported-system") return "";
  return "red";
}

function nvidiaCard(ctx) {
  const root = h("div", { class: "card nv-card", id: "sec-nvidia" });
  let installing = null; // { task }
  let lastError = null;

  async function refresh() {
    clear(root, h("div", { class: "nv-head" }, h("span", { class: "nv-logo" }, icon("monitor", 22)), h("div", { class: "spacer" }, h("strong", {}, t("NVIDIA")), h("div", { class: "small muted" }, t("Carregando…"))), h("span", { class: "spinner" })));
    if (!hasCapability("nvidia-official-v1")) return paintUnavailable(t(UPDATE_VEGAD));
    try {
      paint(await call("software_nvidia_status"));
    } catch (error) {
      paintUnavailable(error.message);
    }
  }

  function header(title, subtitle, tone, badgeEl) {
    return h("div", { class: "nv-head" },
      h("span", { class: `nv-logo ${tone}`.trim() }, icon("monitor", 22)),
      h("div", { class: "spacer" }, h("strong", {}, title), subtitle && h("div", { class: "small muted selectable" }, subtitle)),
      badgeEl);
  }

  function refreshButton() {
    return button(t("Atualizar diagnóstico"), { icon: "refresh", size: "sm", disabled: Boolean(installing), onClick: refresh });
  }

  function paintUnavailable(detail) {
    clear(root,
      header(t("NVIDIA"), t("Diagnóstico NVIDIA indisponível"), "", null),
      h("p", { class: "small muted selectable nv-detail" }, detail),
      h("div", { class: "row-flex" }, refreshButton()));
  }

  function paint(status) {
    const tone = stateTone(status.state);
    const secure = t(SECURE_BOOT[status.secure_boot] ?? N_("Desconhecido"));
    const facts = [
      h("li", {}, icon(status.secure_boot === "enabled" ? "shield-check" : "shield", 16), t("Secure Boot: {state}", { state: secure })),
      status.recovery_snapshot > 0 && h("li", {}, icon("history", 16), t("Snapshot de recuperação: {id}", { id: status.recovery_snapshot })),
    ];
    const canInstall = installable(status);
    const showInstall = status.state === "available" || status.state === "unmanaged";
    const label = status.state === "unmanaged" ? t("Ativar integração Lyra") : t("Instalar driver NVIDIA");
    clear(root,
      header(status.gpu ? gpuName(status.gpu) : t("NVIDIA"), stateMessage(status.state), tone,
        status.installed ? badge(t("Instalado"), status.state === "active" ? "green" : "yellow", "check") : null),
      status.detail && h("p", { class: "small muted selectable nv-detail" }, status.detail),
      h("ul", { class: "nv-facts" }, facts),
      lastError && banner("danger", "alert-circle", t("A instalação não foi concluída. Consulte os detalhes antes de reiniciar."), lastError),
      installing && transactionPanel(installing.task),
      h("div", { class: "row-flex" },
        showInstall && !installing && button(label, { kind: "primary", icon: "download", disabled: !canInstall, onClick: () => install(status) }),
        refreshButton()));
  }

  async function install(status) {
    // Revisa repositórios, confiança e recuperação: sempre obrigatório.
    const ok = await confirm({
      title: t("Instalar a integração NVIDIA?"),
      text: t("O Vega usará os repositórios Lyra no OBS, NVIDIA oficial e openSUSE Leap 16.1. Serão instalados apenas os RPMs qualificados, com verificação de assinaturas e módulo assinado pela SUSE.\n\nÉ necessário criar um snapshot de recuperação. O Vega não remove drivers conflitantes nem troca o kernel automaticamente. Uma falha pode exigir recuperação pelo snapshot. Reinicie somente após uma instalação concluída e verificada.\n\nA autorização de administrador será solicitada ao continuar."),
      ic: "monitor",
      confirmLabel: t("Continuar"),
      required: true,
    });
    if (!ok) return;
    lastError = null;
    try {
      const { task, done } = await runSoftware({ kind: "install-nvidia" }, t("Instalando a integração NVIDIA…"));
      installing = { task };
      paint(status);
      const result = await done;
      if (!result.success) lastError = result.message;
    } catch (error) {
      lastError = error.message;
    }
    installing = null;
    if (!ctx.visible()) return;
    await refresh();
  }

  refresh();
  return root;
}

// ---------- Kernel e bootloader ----------

const NOT_DETECTED = "não detectado";

function kernelTab() {
  const root = h("div", {}, skeletonRows(4));
  const status = h("p", { class: "small faint hw-status selectable" });

  async function load() {
    let installed;
    try {
      installed = await call("kernel_installed");
    } catch (error) {
      clear(root, banner("danger", "alert-circle", t("Kernels instalados"), error.message));
      return;
    }
    let available = null;
    let availableError = null;
    try {
      const all = await call("kernel_available");
      const lower = installed.map((k) => k.toLowerCase());
      available = all.filter((k) => !lower.includes(k.toLowerCase()));
    } catch (error) {
      availableError = error.message;
    }
    let boot = null;
    let entries = null;
    let bootError = null;
    try {
      [boot, entries] = await Promise.all([call("kernel_boot_status"), call("kernel_boot_entries")]);
    } catch (error) {
      bootError = error.message;
    }
    status.textContent = bootError ?? availableError ?? t("Informações carregadas pelo vegad");
    paint(installed, available, availableError, boot, entries, bootError);
  }

  const kernelRows = (values, emptyText, ic, tint) =>
    values.length
      ? list(values.map((name) => row({ icon: ic, tint, title: h("span", { class: "mono kernel-name" }, name) })))
      : list(row({ icon: "info", title: emptyText }));

  function paint(installed, available, availableError, boot, entries, bootError) {
    clear(root,
      h("div", { class: "grid c2" },
        section(t("Kernels instalados"), { id: "sec-kernel" }, kernelRows(installed, t("Nenhum kernel listado"), "terminal", "tint")),
        section(t("Kernels disponíveis"), {},
          availableError ? banner("danger", "alert-circle", t("Kernels disponíveis"), availableError)
            : kernelRows(available, t("Todos os kernels disponíveis já estão instalados"), "download"))),
      bootSection(boot, entries, bootError),
      status);
  }

  function bootSection(boot, entries, bootError) {
    if (bootError) return section(t("Bootloader"), { id: "sec-boot" }, banner("danger", "alert-circle", t("Bootloader"), bootError));
    const supported = Boolean(boot.loader) && boot.loader !== NOT_DETECTED;
    const choices = [...entries];
    if (boot.default_entry && !choices.includes(boot.default_entry)) choices.unshift(boot.default_entry);

    const entry = h("select", { class: "input", "aria-label": t("Entrada padrão") }, choices.map((value) => h("option", { value, selected: value === boot.default_entry }, value)));
    const timeout = h("input", { class: "input", type: "number", min: "0", max: "120", step: "1", value: String(boot.timeout), "aria-label": t("Timeout em segundos") });
    const cmdline = h("input", { class: "input mono", value: boot.cmdline, spellcheck: "false", "aria-label": t("Parâmetros do kernel") });
    const apply = button(t("Aplicar bootloader"), { kind: "primary", icon: "check", disabled: !supported });
    apply.addEventListener("click", async () => {
      const value = Math.min(120, Math.max(0, Math.round(Number(timeout.value) || 0)));
      timeout.value = String(value);
      const defaultEntry = entry.value;
      const ok = await confirm({
        title: t("Alterar configuração de boot?"),
        text: t("Aplicar entrada padrão '{entry}', timeout de {timeout} segundo(s) e os parâmetros informados? Um snapshot será criado quando possível.", { entry: defaultEntry || t("padrão atual"), timeout: value }),
        ic: "power",
        confirmLabel: t("Aplicar"),
        required: false,
      });
      if (!ok) return;
      apply.disabled = true;
      status.textContent = t("Aplicando configuração de boot…");
      try {
        await call("kernel_apply_boot", { entry: defaultEntry, timeout: value, cmdline: cmdline.value.trim() });
        toast(t("Informações carregadas pelo vegad"));
        await load();
      } catch (error) {
        status.textContent = error.message;
        apply.disabled = false;
      }
    });

    const field = (label, control) => h("label", { class: "boot-field" }, h("span", { class: "small muted" }, label), control);
    return section(t("Bootloader"), { id: "sec-boot" },
      h("div", { class: "grid c2 boot-grid" },
        h("div", { class: "card pad" },
          h("dl", { class: "kv" },
            h("dt", {}, t("Detectado")), h("dd", { class: "selectable" }, boot.loader || "—"),
            h("dt", {}, t("Entrada padrão")), h("dd", { class: "selectable", title: boot.default_entry }, boot.default_entry || t("padrão atual")),
            h("dt", {}, t("Timeout")), h("dd", {}, t("{seconds} segundos", { seconds: boot.timeout })),
            h("dt", {}, t("Parâmetros")), h("dd", { class: "mono selectable", title: boot.cmdline }, boot.cmdline || t("Nenhum parâmetro adicional")))),
        h("div", {},
          h("div", { class: "small muted boot-entries-title" }, t("Entradas")),
          entries.length ? list(entries.map((name) => row({ icon: "power", title: name, suffix: name === boot.default_entry ? badge(t("Padrão atual"), "accent") : null })))
            : list(row({ icon: "info", title: t("Nenhuma entrada listada") })))),
      h("div", { class: "card pad boot-editor" },
        h("div", { class: "boot-fields" }, field(t("Entrada padrão"), entry), field(t("Timeout em segundos"), timeout)),
        field(t("Parâmetros do kernel"), cmdline),
        h("div", { class: "row-flex boot-actions" }, apply)));
  }

  load();
  return root;
}

// ---------- Página ----------

export default {
  id: "hardware",
  title: N_("Hardware e Kernel"),
  icon: "cpu",
  group: "system",
  keywords: ["placa de video", "gpu", "driver", "nvidia", "processador", "memoria", "firmware", "kernel", "boot", "grub", "diagnostico", "saude", "teste"],
  sections: [
    { id: "nvidia", label: N_("NVIDIA"), icon: "monitor", keywords: ["nvidia", "driver", "gpu", "placa de video"] },
    diagnosticsSection,
    { id: "kernel", label: N_("Kernel"), icon: "terminal", keywords: ["kernel", "linux"] },
    { id: "boot", label: N_("Bootloader"), icon: "power", keywords: ["boot", "grub", "systemd-boot", "inicializacao"] },
  ],

  render(ctx) {
    let tab = ctx.sub === "kernel" || ctx.sub === "boot" ? "kernel" : "hardware";
    const body = h("div");

    async function hardwareTab() {
      const settle = (promise) => promise.then((value) => ({ value }), (error) => ({ error: error.message }));
      const [inventory, firmware] = await Promise.all([settle(call("hardware_inventory")), settle(call("hardware_firmware"))]);
      return { inventory, firmware };
    }

    function draw() {
      if (tab === "kernel") {
        clear(body, kernelTab());
        return;
      }
      const host = h("div", {}, skeletonRows(2));
      const nvidia = nvidiaCard(ctx);
      clear(body, section(t("Componentes"), {}, host), section(t("NVIDIA"), {}, nvidia), section(t("Diagnóstico"), {}, diagnosticsCard(ctx)));
      hardwareTab().then(({ inventory, firmware }) => clear(host, components(inventory, firmware)));
    }

    const tabs = segmented([{ id: "hardware", label: t("Hardware"), icon: "cpu" }, { id: "kernel", label: t("Kernel"), icon: "terminal" }], tab, (id) => { tab = id; draw(); });
    draw();
    if (ctx.sub) requestAnimationFrame(() => setTimeout(() => document.getElementById(`sec-${ctx.sub}`)?.scrollIntoView({ block: "start", behavior: "smooth" }), 250));

    return {
      el: h("div", {}, h("div", { class: "page-hero" }, h("div", {}, h("h2", {}, t("Hardware e Kernel")), h("p", {}, t("Inventário e kernel detectados pelo vegad"))), tabs), body),
      actions: [h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: draw }, icon("refresh", 17))],
    };
  },
};

