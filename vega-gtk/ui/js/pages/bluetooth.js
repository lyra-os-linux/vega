import { call, pick } from "../api.js";
import { registerIcons } from "../icons.js";
import { N_, t } from "../i18n.js";
import { h, icon, clear, badge, row, list, section, banner, empty, skeletonRows, confirm, toast, fail, busy } from "../ui.js";

registerIcons({
  headphones: '<path d="M3 14h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a9 9 0 0 1 18 0v7a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3"/>',
  mouse: '<rect x="5" y="2" width="14" height="20" rx="7"/><path d="M12 6v4"/>',
  phone: '<rect width="14" height="20" x="5" y="2" rx="2"/><path d="M12 18h.01"/>',
  send: '<path d="M14.54 21.69a.5.5 0 0 0 .94-.03l6.5-19a.5.5 0 0 0-.64-.64l-19 6.5a.5.5 0 0 0-.03.94l7.93 3.18a2 2 0 0 1 1.11 1.11z"/><path d="m21.85 2.15-10.94 10.94"/>',
});

// Mesmo mapeamento do vega-gtk (device_icon).
function deviceIcon(kind) {
  if (["audio-card", "audio-headphones", "audio-headset"].includes(kind)) return "headphones";
  if (kind === "input-keyboard") return "keyboard";
  if (kind === "input-mouse") return "mouse";
  if (kind === "phone") return "phone";
  return "bluetooth";
}

const displayName = (d) => d.alias || d.name || d.address;
const adapterName = (s) => s.controller_name || s.controller || t("Adaptador");
const flag = (label, on) => `${label} ${on ? t("ativa") : t("inativa")}`;

// Durante a busca, a lista de dispositivos é relida enquanto a página estiver aberta.
const SCAN_POLL_MS = 4000;

export default {
  id: "desktop",
  title: N_("Bluetooth"),
  icon: "bluetooth",
  group: "system",
  keywords: ["bluetooth", "fone", "headset", "mouse", "teclado", "parear", "obex", "transferencia"],
  sections: [
    { id: "devices", label: N_("Dispositivos"), icon: "bluetooth", keywords: ["parear", "conectar"] },
    { id: "transfer", label: N_("Transferência de arquivos"), icon: "folder", keywords: ["enviar", "receber", "obex"] },
  ],

  render(ctx) {
    const body = h("div", {}, skeletonRows(3));
    let status = null;
    let devices = [];
    let error = "";

    async function load() {
      try {
        const [s, d] = await Promise.all([call("bluetooth_status"), call("bluetooth_devices")]);
        status = s;
        devices = d;
        error = "";
      } catch (e) {
        error = e.message;
      }
      draw();
    }

    async function togglePower(btn) {
      const enable = !status.powered;
      const verb = enable ? t("Ligar") : t("Desligar");
      const ok = await confirm({
        title: t("{verb} Bluetooth?", { verb }),
        text: enable ? t("O adaptador Bluetooth será ligado.") : t("Dispositivos Bluetooth conectados serão desconectados."),
        ic: "bluetooth",
        confirmLabel: verb,
        required: false,
      });
      if (!ok) return;
      await busy(btn, async () => {
        try {
          await call("bluetooth_set_powered", { on: enable });
        } catch (e) {
          fail(e);
        }
        await load();
      });
    }

    async function toggleScan(btn) {
      const scanning = !status.scanning;
      const close = toast(scanning ? t("Iniciando busca Bluetooth…") : t("Parando busca Bluetooth…"), { ic: "bluetooth", timeout: 30000 });
      await busy(btn, async () => {
        try {
          await call("bluetooth_set_scanning", { on: scanning });
        } catch (e) {
          fail(e);
        }
        close();
        await load();
      });
    }

    async function deviceAction(device, btn) {
      const [action, op] = !device.paired ? [t("Parear"), "pair"] : device.connected ? [t("Desconectar"), "disconnect"] : [t("Conectar"), "connect"];
      const ok = await confirm({
        title: t("{action} dispositivo?", { action }),
        text: `${displayName(device)} • ${device.address}`,
        ic: deviceIcon(device.icon),
        confirmLabel: action,
        required: false,
      });
      if (!ok) return;
      const close = toast(t("{action} dispositivo…", { action }), { ic: "bluetooth", timeout: 60000 });
      await busy(btn, async () => {
        try {
          await call("bluetooth_device_action", { address: device.address, action: op });
        } catch (e) {
          fail(e);
        }
        close();
        await load();
      });
    }

    async function sendFile(device, btn) {
      let path;
      try {
        path = await pick({ title: t("Enviar arquivo por Bluetooth") });
      } catch (e) {
        return fail(e);
      }
      if (!path) return;
      if (typeof path !== "string" || !path.startsWith("/")) return toast(t("Selecione um arquivo local."), { ic: "alert-circle" });
      const ok = await confirm({
        title: t("Enviar arquivo por Bluetooth?"),
        text: t("Enviar {path} para {device} ({address})?", { path, device: displayName(device), address: device.address }),
        ic: "send",
        confirmLabel: t("Enviar"),
        required: false,
      });
      if (!ok) return;
      const close = toast(t("Enviando arquivo por Bluetooth…"), { ic: "send", timeout: 600000 });
      await busy(btn, async () => {
        try {
          await call("bluetooth_send_file", { address: device.address, path });
          close();
          toast(t("Arquivo enviado com sucesso."));
        } catch (e) {
          close();
          fail(e);
        }
      });
    }

    async function receiveFiles(btn) {
      let directory;
      try {
        directory = await pick({ directory: true, title: t("Pasta para arquivos recebidos") });
      } catch (e) {
        return fail(e);
      }
      if (!directory) return;
      if (typeof directory !== "string" || !directory.startsWith("/")) return toast(t("Selecione uma pasta local."), { ic: "alert-circle" });
      const ok = await confirm({
        title: t("Ativar recebimento Bluetooth?"),
        text: t("Arquivos recebidos serão gravados em {directory}. Aceite somente transferências esperadas.", { directory }),
        ic: "download",
        confirmLabel: t("Ativar"),
        required: false,
      });
      if (!ok) return;
      const close = toast(t("Ativando recebimento Bluetooth…"), { ic: "bluetooth", timeout: 30000 });
      await busy(btn, async () => {
        try {
          await call("bluetooth_start_receiver", { directory });
        } catch (e) {
          fail(e);
        }
        close();
        await load();
      });
    }

    function draw() {
      if (error && !status) return clear(body, banner("danger", "alert-circle", t("Bluetooth"), error));
      if (!status) return;
      if (!status.available) {
        return clear(body, empty("bluetooth", t("Bluetooth não está disponível nesta máquina."), t("bluetoothctl não detectado")));
      }

      const powerBtn = h("button", { class: `btn ${status.powered ? "" : "primary"}`.trim(), onclick: () => togglePower(powerBtn) },
        icon("power", 16), status.powered ? t("Desligar Bluetooth") : t("Ligar Bluetooth"));
      const scanBtn = h("button", { class: "btn", disabled: !status.powered, onclick: () => toggleScan(scanBtn) },
        status.scanning ? h("span", { class: "spinner" }) : icon("search", 16), status.scanning ? t("Parar busca") : t("Buscar dispositivos"));

      const hero = h("div", { class: "card pad net-hero" },
        h("span", { class: `net-hero-ic ${status.powered ? "accent" : ""}`.trim() }, icon("bluetooth", 26)),
        h("div", { class: "spacer" },
          h("div", { class: "small faint" }, t("Adaptador")),
          h("h3", {}, adapterName(status)),
          h("div", { class: "small muted" }, status.powered ? t("Bluetooth ligado") : t("Bluetooth desligado")),
          h("div", { class: "bt-flags" },
            badge(flag(t("Descoberta"), status.discoverable), status.discoverable ? "accent" : ""),
            badge(flag(t("Pareamento"), status.pairable), status.pairable ? "accent" : ""),
            badge(status.scanning ? t("Buscando dispositivos") : t("Busca parada"), status.scanning ? "green" : ""))),
        h("div", { class: "bt-hero-actions" }, scanBtn, powerBtn));

      const transferText = status.transfer_available ? (status.receiver_active ? t("Recebimento ativo") : t("Disponível")) : t("bt-obex não instalado");
      const receiveBtn = h("button", { class: "btn sm", disabled: !(status.transfer_available && !status.receiver_active), onclick: () => receiveFiles(receiveBtn) }, icon("download", 15), t("Receber arquivos"));
      const transfer = list(row({
        icon: "folder",
        tint: status.receiver_active ? "green" : "",
        title: t("Transferência de arquivos"),
        subtitle: [transferText, status.receiver_active && status.receive_path].filter(Boolean).join(" • "),
        suffix: receiveBtn,
      }));

      const deviceRows = devices.length === 0
        ? h("div", { class: "card pad muted small" }, t("Nenhum dispositivo conhecido ou encontrado"))
        : list(devices.map((device) => {
            const label = !device.paired ? t("Parear") : device.connected ? t("Desconectar") : t("Conectar");
            const actionBtn = h("button", { class: `btn sm ${!device.connected ? "primary" : ""}`.trim(), disabled: !status.powered, onclick: () => deviceAction(device, actionBtn) }, label);
            const sendBtn = h("button", { class: "icon-btn sm", "aria-label": t("Enviar arquivo"), title: t("Enviar arquivo"), disabled: !(device.paired && status.transfer_available), onclick: () => sendFile(device, sendBtn) }, icon("send", 15));
            const connection = device.connected ? t("Conectado") : device.paired ? t("Pareado") : t("Disponível");
            return row({
              icon: deviceIcon(device.icon),
              tint: device.connected ? "green" : device.paired ? "tint" : "",
              title: h("span", { class: "row-flex" }, displayName(device), device.connected && badge(t("Conectado"), "green")),
              subtitle: t("{address} • {connection} • {trust} • sinal {rssi} dBm", {
                address: device.address, connection, trust: device.trusted ? t("Confiável") : t("Não confiável"), rssi: device.rssi,
              }),
              suffix: [sendBtn, actionBtn],
            });
          }));

      clear(body,
        error && banner("danger", "alert-circle", t("Bluetooth"), error),
        hero,
        section(t("Dispositivos"), { id: "sec-devices" }, deviceRows),
        section(t("Transferência de arquivos"), { id: "sec-transfer" }, transfer));
    }

    load();
    const timer = setInterval(() => {
      if (ctx.visible() && status?.scanning && !document.querySelector(".overlay")) load();
    }, SCAN_POLL_MS);
    ctx.cleanup(() => clearInterval(timer));

    const refreshBtn = h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: () => busy(refreshBtn, load) }, icon("refresh", 17));
    return {
      el: h("div", {},
        h("div", { class: "page-hero" }, h("div", {}, h("h2", {}, t("Bluetooth")), h("p", {}, t("Adaptador, dispositivos e transferência de arquivos")))),
        body),
      actions: [refreshBtn],
    };
  },
};
