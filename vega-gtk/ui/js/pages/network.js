import { call, pick } from "../api.js";
import { registerIcons } from "../icons.js";
import { N_, t } from "../i18n.js";
import { h, icon, clear, button, badge, toggle, row, list, section, banner, empty, skeletonRows, confirm, toast, fail, busy, segmented } from "../ui.js";

registerIcons({
  vpn: '<path d="M12 2 4 5v6c0 5 3.4 9.3 8 11 4.6-1.7 8-6 8-11V5z"/><rect width="7" height="6" x="8.5" y="11" rx="1"/><path d="M10 11V9a2 2 0 0 1 4 0v2"/>',
});

const dash = (value) => (value ? value : "—");

const KIND_ICON = { wifi: "wifi", ethernet: "plug", vpn: "vpn", wireguard: "vpn", tun: "vpn", "wifi-p2p": "wifi", loopback: "server", bridge: "layers" };
const kindIcon = (kind) => KIND_ICON[kind] ?? "globe";
const isVpn = (kind) => ["vpn", "wireguard", "tun"].includes(kind);

/** Mesma regra do vega-gtk (valid_ipv4_cidr). */
function validIpv4(address) {
  const parts = address.split(".");
  return parts.length === 4 && parts.every((p) => /^\d{1,3}$/.test(p) && Number(p) <= 255 && !(p.length > 1 && p.startsWith("0")));
}
function validIpv4Cidr(value) {
  const at = value.indexOf("/");
  if (at < 0) return false;
  const prefix = value.slice(at + 1);
  return validIpv4(value.slice(0, at)) && /^\d{1,3}$/.test(prefix) && Number(prefix) <= 32;
}

/** Mesma regra do vega-gtk (wifi_requires_password). */
const wifiRequiresPassword = (security) => !["", "--", "open", "none"].includes(security.trim().toLowerCase());

export function signalBars(strength) {
  const bars = strength > 75 ? 4 : strength > 50 ? 3 : strength > 25 ? 2 : 1;
  return h("span", { class: "signal", title: `${strength}%`, "aria-label": `${strength}%` },
    [1, 2, 3, 4].map((i) => h("i", { class: `b${i} ${i <= bars ? "on" : ""}`.trim() })));
}

/** Foco no primeiro campo e Enter confirma, em diálogos com formulário. */
function formDialog(first, inputs, open) {
  for (const input of inputs) {
    input.addEventListener("keydown", (e) => {
      if (e.key !== "Enter") return;
      e.preventDefault();
      input.closest(".dialog")?.querySelector(".dialog-actions .btn:last-child")?.click();
    });
  }
  const pending = open();
  if (first) requestAnimationFrame(() => requestAnimationFrame(() => first.focus()));
  return pending;
}

function heroCard(ic, tone, eyebrow, title, subtitle, extra) {
  return h("div", { class: "card pad net-hero" },
    h("span", { class: `net-hero-ic ${tone}` }, icon(ic, 26)),
    h("div", { class: "spacer" }, h("div", { class: "small faint" }, eyebrow), h("h3", {}, title), subtitle && h("div", { class: "small muted mono selectable" }, subtitle)),
    extra);
}

const TABS = [
  { id: "interfaces", label: N_("Interfaces"), icon: "plug" },
  { id: "wifi", label: N_("Wi‑Fi"), icon: "wifi" },
  { id: "proxy", label: N_("Proxy"), icon: "globe" },
  { id: "vpn", label: "VPN", icon: "vpn" },
  { id: "firewall", label: N_("Firewall"), icon: "shield" },
];

export default {
  id: "network",
  title: N_("Rede e Firewall"),
  icon: "globe",
  group: "system",
  keywords: ["rede", "network", "internet", "ip", "ipv4", "dns", "wifi", "wi-fi", "vpn", "openvpn", "proxy", "firewall", "firewalld", "porta"],
  sections: [
    { id: "interfaces", label: N_("Interfaces"), icon: "plug", keywords: ["ethernet", "ipv4", "dns", "gateway"] },
    { id: "wifi", label: N_("Wi‑Fi"), icon: "wifi", keywords: ["wifi", "wireless", "senha"] },
    { id: "proxy", label: N_("Proxy"), icon: "globe", keywords: ["http", "socks", "environment"] },
    { id: "vpn", label: "VPN", icon: "key", keywords: ["openvpn", "ovpn"] },
    { id: "firewall", label: N_("Firewall"), icon: "shield", keywords: ["firewalld", "porta", "zona", "ssh"] },
  ],

  render(ctx) {
    let tab = TABS.some((x) => x.id === ctx.sub) ? ctx.sub : "interfaces";
    const data = { interfaces: null, wifi: null, proxy: null, firewall: null, services: null, ports: null };
    const errors = {};
    const heroHost = h("div");
    const body = h("div", {}, skeletonRows(4));
    let interfaceBusy = false;
    const expanded = new Set();

    const settle = async (key, promise) => {
      try {
        data[key] = await promise;
        delete errors[key];
      } catch (error) {
        data[key] = null;
        errors[key] = error.message;
      }
    };

    const loadInterfaces = () => settle("interfaces", call("network_interfaces"));
    const loadWifi = () => settle("wifi", call("network_wifi"));
    const loadProxy = () => settle("proxy", call("network_proxy"));
    const loadFirewall = () => Promise.all([
      settle("firewall", call("firewall_status")),
      settle("services", call("firewall_services")),
      settle("ports", call("firewall_ports")),
    ]);

    async function loadAll() {
      await Promise.all([loadInterfaces(), loadWifi(), loadProxy(), loadFirewall()]);
      draw();
    }

    const connected = () => (data.interfaces ?? []).find((i) => i.state === "connected" && !isVpn(i.kind) && i.kind !== "loopback");
    // O vegad pode não marcar a rede atual como ativa; a interface conectada
    // com o mesmo nome confirma.
    const wifiActive = (w) => w.active || (data.interfaces ?? []).some((i) => i.kind === "wifi" && i.state === "connected" && i.device === w.device && (i.ssid || i.name) === w.ssid);

    // ---------- Interfaces ----------
    async function configureIpv4(item) {
      if (interfaceBusy) return;
      const field = (value, placeholder, readonly) => {
        const input = h("input", { class: "input", value: value ?? "", placeholder, readonly: readonly || null });
        input.value = value ?? "";
        return input;
      };
      const connection = field(item.name, "", true);
      const address = field(item.ipv4, "192.168.1.20/24");
      const gateway = field(item.gateway, "192.168.1.1");
      const dns = field(item.dns, "1.1.1.1, 8.8.8.8");
      const form = h("div", { class: "net-form" },
        [[t("Conexão"), connection], [t("Endereço/CIDR"), address], ["Gateway", gateway], ["DNS", dns]]
          .map(([label, input]) => h("label", {}, h("span", {}, label), input)));
      const ok = await formDialog(address, [address, gateway, dns], () => confirm({
        title: t("Configurar IPv4 estático?"),
        text: t("O NetworkManager substituirá a configuração automática e reconectará esta conexão."),
        ic: "plug",
        extra: form,
        confirmLabel: t("Aplicar"),
      }));
      if (!ok) return;
      const values = [connection, address, gateway, dns].map((input) => input.value.trim());
      const [name, cidr, gw, servers] = values;
      if (!name || !validIpv4Cidr(cidr)) return toast(t("Informe uma conexão e um endereço IPv4 com CIDR válido."), { ic: "alert-circle" });
      if (gw && !validIpv4(gw)) return toast(t("O gateway IPv4 informado é inválido."), { ic: "alert-circle" });
      // Outro formulário pode ter começado uma alteração enquanto este esperava.
      if (interfaceBusy) return;
      interfaceBusy = true;
      draw();
      const close = toast(t("Aplicando IPv4 estático…"), { ic: "refresh", timeout: 60000 });
      try {
        await call("network_set_static_ipv4", { connection: name, address: cidr, gateway: gw, dns: servers });
        close();
        await loadInterfaces();
        toast(t("Interfaces de rede atualizadas"));
      } catch (error) {
        close();
        fail(error);
      } finally {
        interfaceBusy = false;
        draw();
      }
    }

    function interfaceSheet(item) {
      const kv = [
        [t("Conexão"), item.name], ["Interface", item.device], ["IPv4", item.ipv4], ["IPv6", item.ipv6],
        ["Gateway", item.gateway], ["DNS", item.dns], ["MAC", item.mac],
      ].filter(([, v]) => v);
      return kv;
    }

    function drawInterfaces() {
      if (errors.interfaces) return banner("danger", "alert-circle", t("Interfaces"), errors.interfaces);
      const items = data.interfaces ?? [];
      if (!items.length) return empty("plug", t("Nenhuma interface detectada"));
      return list(items.flatMap((item) => {
        const open = expanded.has(item.device + item.name);
        const up = item.state === "connected";
        const r = h("div", { class: "list-row clickable", role: "button", tabindex: "0", "aria-expanded": String(open),
          onclick: () => { open ? expanded.delete(item.device + item.name) : expanded.add(item.device + item.name); draw(); },
          onkeydown: (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); e.currentTarget.click(); } } },
          h("span", { class: `lead ${up ? "green" : ""}`.trim() }, icon(kindIcon(item.kind), 18)),
          h("div", { class: "body" },
            h("div", { class: "title" }, `${item.name} • ${item.device}`),
            h("div", { class: "subtitle selectable" }, `${item.kind} • ${item.state} • IPv4 ${dash(item.ipv4)} • ${dash(item.speed)} • ${dash(item.mac)}`)),
          h("div", { class: "suffix" },
            h("span", { class: "hide-narrow" }, badge(item.state, up ? "green" : "")),
            icon("chevron-right", 16, "ic chev")));
        const detail = open && h("div", { class: "list-detail" },
          h("dl", { class: "kv net-kv" }, interfaceSheet(item).flatMap(([k, v]) => [h("dt", {}, k), h("dd", { class: "mono selectable", title: v }, v)])),
          h("div", { class: "row-flex", style: { "margin-top": "12px" } },
            button(t("Configurar IPv4"), { icon: "cog", size: "sm", disabled: interfaceBusy, onClick: (e) => { e.stopPropagation(); configureIpv4(item); } })));
        return [r, detail];
      }));
    }

    // ---------- Wi-Fi ----------
    async function wifiAction(network, btn) {
      const disconnect = wifiActive(network);
      const needsPassword = !disconnect && wifiRequiresPassword(network.security);
      const password = needsPassword && h("input", { class: "input", type: "password", placeholder: t("Senha da rede"), autocomplete: "off" });
      if (password) password.style.width = "100%";
      const ok = await formDialog(password, password ? [password] : [], () => confirm({
        title: disconnect ? t("Desconectar do Wi‑Fi?") : t("Conectar ao Wi‑Fi?"),
        text: (disconnect ? t("{ssid} será desconectada deste dispositivo.") : t("{ssid} será conectada pelo NetworkManager.")).replace("{ssid}", network.ssid),
        ic: "wifi",
        extra: password || undefined,
        confirmLabel: disconnect ? t("Desconectar") : t("Conectar"),
        required: false,
      }));
      if (!ok) return;
      const secret = password ? password.value : "";
      if (needsPassword && !secret) return toast(t("Informe a senha da rede Wi‑Fi."), { ic: "alert-circle" });
      await busy(btn, async () => {
        try {
          if (disconnect) await call("network_disconnect", { device: network.device });
          else await call("network_connect_wifi", { ssid: network.ssid, password: secret });
          await Promise.all([loadWifi(), loadInterfaces()]);
          toast(t("Redes Wi‑Fi atualizadas"));
        } catch (error) {
          fail(error);
        }
      });
      draw();
    }

    function drawWifi() {
      if (errors.wifi) return banner("danger", "alert-circle", t("Wi‑Fi"), errors.wifi);
      const items = [...(data.wifi ?? [])].sort((a, b) => wifiActive(b) - wifiActive(a) || b.signal - a.signal);
      if (!items.length) return empty("wifi", t("Nenhuma rede Wi‑Fi detectada"));
      return list(items.map((network) => {
        const active = wifiActive(network);
        const action = h("button", { class: `btn sm ${active ? "" : "primary"}`.trim(), onclick: () => wifiAction(network, action) }, active ? t("Desconectar") : t("Conectar"));
        return row({
          lead: h("span", { class: `lead ${active ? "green" : ""}`.trim() }, signalBars(network.signal)),
          title: h("span", { class: "row-flex" }, network.ssid || "—", active && badge(t("Conectada"), "green", "check"), !wifiRequiresPassword(network.security) && icon("alert", 14, "ic faint")),
          subtitle: t("{security} • sinal {signal}% • {state}", { security: dash(network.security), signal: network.signal, state: active ? t("Conectada") : t("Disponível") }),
          suffix: action,
        });
      }));
    }

    // ---------- Proxy ----------
    function drawProxy() {
      if (errors.proxy) return banner("danger", "alert-circle", t("Proxy"), errors.proxy);
      const p = data.proxy ?? { http: "", https: "", socks: "", no_proxy: "" };
      const input = (value, placeholder) => {
        const el = h("input", { class: "input mono", placeholder, spellcheck: "false" });
        el.value = value ?? "";
        return el;
      };
      const fields = {
        http: input(p.http, t("http://servidor:porta")),
        https: input(p.https, t("http://servidor:porta")),
        socks: input(p.socks, t("socks://servidor:porta")),
        no_proxy: input(p.no_proxy, t("localhost, 127.0.0.1, domínio.local")),
      };
      const configured = Object.values(p).some(Boolean);
      const apply = h("button", { class: "btn primary", onclick: async () => {
        const config = Object.fromEntries(Object.entries(fields).map(([k, el]) => [k, el.value.trim()]));
        const clearing = Object.values(config).every((v) => !v);
        const ok = await confirm({
          title: clearing ? t("Remover configuração de proxy?") : t("Aplicar proxy global?"),
          text: clearing ? t("As variáveis de proxy gerenciadas pelo Vega serão removidas de /etc/environment.") : t("A configuração será gravada em /etc/environment e poderá exigir uma nova sessão para alcançar todos os aplicativos."),
          ic: "globe",
          tone: clearing ? "danger" : "",
          confirmLabel: clearing ? t("Remover") : t("Aplicar"),
          confirmKind: clearing ? "danger" : "primary",
          required: false,
        });
        if (!ok) return;
        await busy(apply, async () => {
          try {
            await call("network_set_proxy", { proxy: config });
            await loadProxy();
            toast(errors.proxy ?? t("Configuração carregada de /etc/environment"), { ic: errors.proxy ? "alert-circle" : "check-circle" });
          } catch (error) {
            fail(error);
          }
        });
        draw();
      } }, t("Aplicar proxy"));
      return h("div", {},
        heroCard("globe", configured ? "accent" : "", t("Proxy"), configured ? t("Configuração carregada de /etc/environment") : t("Proxy não configurado"), null),
        h("div", { class: "card pad net-form" },
          [["HTTP", fields.http], ["HTTPS", fields.https], ["SOCKS", fields.socks], [t("Exceções"), fields.no_proxy]].map(([label, el]) => h("label", {}, h("span", {}, label), el)),
          h("div", { class: "row-flex net-form-actions" }, apply)));
    }

    // ---------- VPN ----------
    let vpnStatus = "";
    async function importVpn(btn) {
      let path;
      try {
        path = await pick({ title: t("Importar perfil OpenVPN"), filters: [{ name: t("Perfis OpenVPN (*.ovpn)"), extensions: ["ovpn", "OVPN"] }] });
      } catch (error) {
        return fail(error);
      }
      if (!path) return;
      if (typeof path !== "string" || !path.startsWith("/")) {
        vpnStatus = t("Selecione um arquivo local de perfil OpenVPN.");
        return draw();
      }
      if (!/\.ovpn$/i.test(path)) {
        vpnStatus = t("O perfil deve possuir a extensão .ovpn.");
        return draw();
      }
      const ok = await confirm({
        title: t("Importar perfil OpenVPN?"),
        text: t("O NetworkManager importará o perfil:\n{path}\n\nRevise a origem e o conteúdo do arquivo antes de continuar.", { path }),
        ic: "vpn",
        confirmLabel: t("Importar"),
        required: false,
      });
      if (!ok) return;
      await busy(btn, async () => {
        vpnStatus = t("Importando perfil OpenVPN…");
        try {
          await call("network_import_vpn", { path });
          vpnStatus = t("Perfil importado no NetworkManager.");
          await loadInterfaces();
        } catch (error) {
          vpnStatus = error.message;
        }
      });
      draw();
    }

    function drawVpn() {
      const vpns = (data.interfaces ?? []).filter((i) => isVpn(i.kind));
      const btn = h("button", { class: "btn primary", onclick: () => importVpn(btn) }, icon("download", 16), t("Importar perfil .ovpn"));
      return h("div", {},
        h("div", { class: "card pad net-hero" },
          h("span", { class: "net-hero-ic accent" }, icon("vpn", 26)),
          h("div", { class: "spacer" }, h("h3", {}, "OpenVPN"), h("div", { class: "small muted" }, t("Importe um perfil OpenVPN fornecido pelo seu serviço de VPN.")),
            vpnStatus && h("div", { class: "small selectable", style: { "margin-top": "6px" } }, vpnStatus)),
          btn),
        vpns.length > 0 && section("VPN", {}, list(vpns.map((item) => row({
          icon: "vpn", tint: item.state === "connected" ? "green" : "",
          title: item.name,
          subtitle: `${item.kind} • ${item.state} • IPv4 ${dash(item.ipv4)}`,
          suffix: badge(item.state, item.state === "connected" ? "green" : ""),
        })))));
    }

    // ---------- Firewall ----------
    async function toggleService(service, input) {
      const enable = !service.enabled;
      const verb = enable ? t("Permitir") : t("Bloquear");
      const ok = await confirm({
        title: t("{verb} serviço no firewall?", { verb }),
        text: t("{label} ({name}) será {state} nas conexões de entrada.", { label: service.label, name: service.name, state: enable ? t("permitido") : t("bloqueado") }),
        ic: "shield",
        confirmLabel: verb,
        required: false,
      });
      if (!ok) {
        input.checked = service.enabled;
        return;
      }
      input.disabled = true;
      const close = toast(t("{verb} {label}…", { verb, label: service.label }), { ic: "shield", timeout: 60000 });
      try {
        await call("firewall_set_service", { name: service.name, enabled: enable });
      } catch (error) {
        fail(error);
      }
      close();
      await loadFirewall();
      draw();
    }

    async function addPort(portInput, protocolSelect, btn) {
      const port = portInput.value.trim();
      const protocol = protocolSelect.value;
      if (!port) return toast(t("Informe a porta ou intervalo antes de adicionar."), { ic: "alert-circle" });
      const ok = await confirm({
        title: t("Adicionar regra de firewall?"),
        text: t("A porta {port}/{protocol} passará a aceitar conexões de entrada.", { port, protocol }),
        ic: "shield",
        confirmLabel: t("Adicionar"),
        required: false,
      });
      if (!ok) return;
      await busy(btn, async () => {
        try {
          await call("firewall_add_port", { port, protocol });
          portInput.value = "";
          await loadFirewall();
        } catch (error) {
          fail(error);
        }
      });
      draw();
    }

    async function removePort(rule, btn) {
      const ok = await confirm({
        title: t("Remover regra de firewall?"),
        text: t("A porta {port}/{protocol} deixará de aceitar conexões de entrada.", { port: rule.port, protocol: rule.protocol }),
        ic: "trash",
        tone: "danger",
        confirmLabel: t("Remover"),
        confirmKind: "danger",
        required: false,
      });
      if (!ok) return;
      await busy(btn, async () => {
        try {
          await call("firewall_remove_port", { port: rule.port, protocol: rule.protocol });
          await loadFirewall();
        } catch (error) {
          fail(error);
        }
      });
      draw();
    }

    function drawFirewall() {
      const status = data.firewall;
      const state = status ? (status.enabled ? t("Ativo") : t("Inativo")) : "";
      const top = errors.firewall
        ? banner("danger", "alert-circle", t("Firewall"), errors.firewall)
        : heroCard(status?.enabled ? "shield-check" : "shield", status?.enabled ? "green" : "red", "firewalld", t("Firewall"),
            t("{state} • zona/perfil: {zone}", { state, zone: dash(status?.active_zone) }), badge(state, status?.enabled ? "green" : "red"));

      const services = errors.services
        ? banner("danger", "alert-circle", t("Firewall"), errors.services)
        : (data.services ?? []).length === 0
          ? h("div", { class: "card pad muted small" }, t("Nenhum serviço publicado"))
          : list(data.services.map((service) => {
              const label = service.enabled ? t("Bloquear serviço") : t("Permitir serviço");
              const sw = toggle(service.enabled, (_on, input) => toggleService(service, input), label);
              sw.title = label;
              return row({
                icon: service.enabled ? "check-circle" : "lock",
                tint: service.enabled ? "green" : "",
                title: service.label || service.name,
                subtitle: h("span", { class: "mono" }, service.name),
                suffix: [h("span", { class: "hide-narrow" }, badge(service.enabled ? t("Permitido") : t("Bloqueado"), service.enabled ? "green" : "")), sw],
              });
            }));

      const portInput = h("input", { class: "input mono", placeholder: t("porta ou intervalo, ex.: 8080 ou 9000-9010"), spellcheck: "false" });
      const protocol = h("select", { class: "input", "aria-label": "Protocol" }, h("option", { value: "tcp" }, "tcp"), h("option", { value: "udp" }, "udp"));
      const addBtn = h("button", { class: "btn primary", onclick: () => addPort(portInput, protocol, addBtn) }, icon("plus", 16), t("Adicionar regra"));
      portInput.addEventListener("keydown", (e) => { if (e.key === "Enter") addBtn.click(); });

      const ports = errors.ports
        ? banner("danger", "alert-circle", t("Regras de porta personalizadas"), errors.ports)
        : h("div", { class: "list" },
            (data.ports ?? []).length === 0
              ? h("div", { class: "list-row muted small" }, t("Nenhuma regra de porta personalizada"))
              : data.ports.map((rule) => {
                  const remove = h("button", { class: "btn sm danger-soft", onclick: () => removePort(rule, remove) }, t("Remover regra"));
                  return row({ icon: "plug", title: h("span", { class: "mono" }, `${rule.port}/${rule.protocol}`), suffix: remove });
                }),
            h("div", { class: "list-footer net-port-add" }, portInput, protocol, addBtn));

      return h("div", {}, top, services, section(t("Regras de porta personalizadas"), {}, ports));
    }

    // ---------- Montagem ----------
    function drawHero() {
      const item = connected();
      if (!item) return clear(heroHost);
      const wifi = item.kind === "wifi" ? (data.wifi ?? []).find((w) => w.ssid === (item.ssid || item.name)) : null;
      clear(heroHost, heroCard(kindIcon(item.kind), "green", t("Conectada"), item.ssid || item.name,
        [item.device, item.ipv4, item.speed].filter(Boolean).join(" • "),
        wifi && h("span", { class: "net-hero-signal" }, signalBars(wifi.signal), `${wifi.signal}%`)));
    }

    function draw() {
      if (data.interfaces === null && !errors.interfaces && data.wifi === null && !errors.wifi) return;
      drawHero();
      const view = { interfaces: drawInterfaces, wifi: drawWifi, proxy: drawProxy, vpn: drawVpn, firewall: drawFirewall }[tab]();
      clear(body, view);
    }

    const tabs = segmented(TABS.map((x) => ({ ...x, label: t(x.label) })), tab, (id) => { tab = id; draw(); });
    loadAll();

    const refreshBtn = h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: () => busy(refreshBtn, loadAll) }, icon("refresh", 17));

    return {
      el: h("div", { class: "net-page" },
        h("div", { class: "page-hero" }, h("div", {}, h("h2", {}, t("Rede e Firewall")), h("p", {}, t("Interfaces, Wi‑Fi, proxy e proteção da rede")))),
        heroHost,
        h("div", { class: "net-tabs" }, tabs),
        body),
      actions: [refreshBtn],
    };
  },
};
