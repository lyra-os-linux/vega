import { call } from "../api.js";
import { N_, t } from "../i18n.js";
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
  keywords: ["placa de video", "gpu", "driver", "processador", "memoria", "firmware", "kernel", "boot", "grub", "diagnostico", "saude", "teste"],
  sections: [
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
      clear(body, section(t("Componentes"), {}, host), section(t("Diagnóstico"), {}, diagnosticsCard(ctx)));
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

