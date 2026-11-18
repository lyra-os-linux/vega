// Seção "Certificado digital" da vitrine Populares (vega#211, fase 1).
//
// Assistente para o certificado ICP-Brasil: A1 (arquivo .pfx/.p12) e A3
// (token ou cartão) que funciona com o OpenSC do Leap. Os passos aparecem
// conforme o estado lido por `certificate_status` (backend/certificates.rs):
// instalar o suporte a leitoras, ver a leitora e o token, registrar o token
// nos navegadores, ver titular e validade, importar um A1. Os pacotes usam a
// transação oficial da loja; o resto roda na conta do usuário, sem senha.
//
// Tokens que precisam de middleware proprietário (SafeNet, Watchdata, GD
// Burti) ficam para a fase 2 (vega#263): aqui só aparecem como "não
// reconhecido", com o diagnóstico copiável para o relato (.github#5).

import { call, pick } from "../../api.js";
import { t } from "../../i18n.js";
import { h, icon, button, banner, badge, section, confirm, toast, fail } from "../../ui.js";

const PACKAGES = ["pcsc-lite", "pcsc-ccid", "opensc"];
const SOCKET = "pcscd.socket";
/** A partir de quantos dias o certificado aparece como "vence em breve". */
const EXPIRING_DAYS = 30;
const REPORT_URL = "https://github.com/lyra-os-linux/.github/issues/new?template=banco-governo.yml";

// Estado que sobrevive à troca de aba, como em banks.js.
const state = { status: null, error: null, loading: false, acting: false };

export function resetCertificateSection() {
  if (state.loading || state.acting) return;
  state.status = null;
  state.error = null;
}

const dateLabel = (iso) => new Date(`${iso}T12:00:00`).toLocaleDateString();

function validity(cert) {
  if (cert.days_left < 0) return badge(t("Vencido em {date}", { date: dateLabel(cert.not_after) }), "red", "alert-circle");
  if (cert.days_left <= EXPIRING_DAYS) return badge(t("Vence em {days} dias", { days: cert.days_left }), "yellow", "alert");
  return badge(t("Válido até {date}", { date: dateLabel(cert.not_after) }), "green", "check");
}

/** Pede a senha do A1 num diálogo; devolve null se cancelar. */
async function askPassword(name) {
  const input = h("input", { class: "input", type: "password", autocomplete: "off", "aria-label": t("Senha do certificado") });
  input.style.width = "100%";
  const ok = await confirm({
    title: t("Importar {name}?", { name }),
    text: t("Digite a senha do certificado. O Vega importa o certificado no Firefox, no Chromium, no Edge e nos web apps, só na sua conta. A senha não fica guardada."),
    ic: "key",
    extra: input,
    confirmLabel: t("Importar"),
  });
  return ok ? input.value : null;
}

/**
 * Seção "Certificado digital". `transact` é o da página Software (painel de
 * transação e polkit); `repaint` redesenha a vitrine; `busy` diz se há
 * transação em andamento.
 */
export function certificateSection({ transact, repaint, busy }) {
  async function refresh() {
    try {
      state.status = await call("certificate_status");
      state.error = null;
    } catch (error) {
      state.error = error.message ?? String(error);
    }
    repaint();
  }

  if (!state.status && !state.error && !state.loading) {
    state.loading = true;
    refresh().finally(() => { state.loading = false; });
  }

  async function act(work) {
    state.acting = true;
    repaint();
    try {
      await work();
    } catch (error) {
      fail(error, t("Não foi possível concluir a operação"));
    } finally {
      state.acting = false;
      await refresh();
    }
  }

  async function installSupport(missing) {
    for (const id of missing) {
      const result = await transact({ kind: "install", origin: "official", id }, t("Instalando {name}…", { name: id }), { reload: false });
      if (!result?.success) return;
    }
    // O pcscd sobe pelo socket quando o OpenSC pede a leitora.
    const now = await call("certificate_status");
    if (now.pcscd !== "enabled") await call("services_set_enabled", { name: SOCKET, enabled: true });
  }

  async function register() {
    const done = await call("certificate_register_browsers");
    if (done.length > 0) toast(t("Token registrado em: {list}. Reabra o Firefox se ele estiver aberto.", { list: done.join(", ") }));
  }

  async function importA1() {
    let path;
    try {
      path = await pick({ title: t("Escolha o certificado A1"), filters: [{ name: t("Certificado A1"), extensions: ["pfx", "p12", "PFX", "P12"] }] });
    } catch (error) {
      return fail(error);
    }
    if (!path || Array.isArray(path)) return;
    const password = await askPassword(String(path).split("/").pop());
    if (password == null) return;
    const cert = await call("certificate_import_a1", { path, password });
    if (cert.days_left < 0) toast(t("Certificado de {holder} importado, mas está vencido desde {date}.", { holder: cert.holder, date: dateLabel(cert.not_after) }), { ic: "alert" });
    else if (cert.days_left <= EXPIRING_DAYS) toast(t("Certificado de {holder} importado. Ele vence em {days} dias: providencie a renovação.", { holder: cert.holder, days: cert.days_left }), { ic: "alert" });
    else toast(t("Certificado de {holder} importado, válido até {date}. Reabra o navegador para usá-lo.", { holder: cert.holder, date: dateLabel(cert.not_after) }));
  }

  async function copyDiagnostics() {
    const text = await call("certificate_diagnostics");
    await navigator.clipboard?.writeText(text);
    toast(t("Diagnóstico copiado. Ele não tem seu nome, CPF nem número de série."));
  }

  const disabled = busy || state.acting;
  const status = state.status;
  const actions = h("div", { class: "row-flex wrap" },
    button(t("Importar certificado A1…"), { kind: "primary", size: "sm", icon: "key", disabled, onClick: () => act(importA1) }),
    button(t("Copiar diagnóstico"), { size: "sm", icon: "copy", disabled: disabled || !status, onClick: () => act(copyDiagnostics) }),
    button(t("Atualizar"), { size: "sm", icon: "refresh", disabled, onClick: () => act(async () => {}) }));

  let body;
  if (state.error) {
    body = banner("error", "alert-circle", t("Não foi possível ler o estado do certificado"), state.error);
  } else if (!status) {
    body = h("p", { class: "small muted" }, t("Procurando leitoras e certificados…"));
  } else {
    const a3 = [];
    if (status.missing.length > 0) {
      a3.push(banner("info", "download", t("Certificado A3: instale o suporte a leitoras"),
        t("Para usar token ou cartão, o Vega instala o pcsc-lite, o pcsc-ccid e o OpenSC dos repositórios do openSUSE. Para o A1 não precisa."),
        button(t("Instalar"), { kind: "primary", size: "sm", icon: "download", disabled, onClick: () => act(() => installSupport(status.missing)) })));
    } else {
      if (status.readers.length === 0) {
        a3.push(h("p", { class: "small muted" }, t("Nenhuma leitora ou token conectado. Conecte e clique em Atualizar.")));
      }
      for (const reader of status.readers) {
        a3.push(h("div", { class: "row-flex" }, icon("cpu", 16),
          h("span", { class: "spacer" }, reader.name),
          badge(reader.card ? t("com cartão") : t("sem cartão"), reader.card ? "green" : "")));
      }
      const unknown = status.tokens.filter((token) => !token.recognized);
      if (unknown.length > 0) {
        a3.push(banner("warn", "alert", t("Token não reconhecido pelo OpenSC"),
          t("Este token precisa do programa do fabricante (SafeNet, Watchdata, GD Burti…). O suporte a esses tokens vem numa próxima etapa. Copie o diagnóstico e relate o modelo para entrar na lista."),
          button(t("Reportar problema"), { size: "sm", icon: "external", onClick: () => call("open_uri", { uri: REPORT_URL }).catch((error) => fail(error, t("Não foi possível abrir o site"))) })));
      }
      const pending = status.databases.filter((db) => !db.registered);
      if (status.tokens.some((token) => token.recognized) || status.readers.length > 0) {
        a3.push(pending.length > 0
          ? banner("info", "globe", t("Registrar o token nos navegadores"),
            t("O Firefox, o Chromium, o Edge e os web apps do Vega passam a pedir o PIN do token nos sites que exigem certificado. Faltam: {list}.", { list: pending.map((db) => db.browser).join(", ") }),
            button(t("Registrar"), { kind: "primary", size: "sm", icon: "check", disabled, onClick: () => act(register) }))
          : h("p", { class: "small muted" }, icon("check", 14), " ", t("Token registrado no Firefox, no Chromium, no Edge e nos web apps.")));
      }
    }
    if (status.flatpak.length > 0) {
      a3.push(banner("warn", "alert", t("Navegadores Flatpak não usam o token"),
        t("{list} rodam isolados e não enxergam o token. Use o Firefox, o Chromium ou o Edge nos sites que pedem certificado.", { list: status.flatpak.join(", ") })));
    }
    const certs = status.certificates.length > 0
      ? h("div", { class: "list" }, status.certificates.map((cert) => h("div", { class: "row-flex" },
        icon(cert.kind === "token" ? "shield" : "key", 16),
        h("div", { class: "spacer" }, h("strong", {}, cert.holder),
          h("div", { class: "small muted" }, `${cert.kind === "token" ? t("A3 (token ou cartão)") : t("A1 (arquivo)")} · ${cert.issuer}`)),
        validity(cert))))
      : h("p", { class: "small muted" }, t("Nenhum certificado encontrado ainda."));
    body = h("div", { class: "sw-cert-body" }, certs, ...a3);
  }

  return section(t("Certificado digital"), { id: "section-certificate" },
    h("div", { class: "card pad sw-cert" },
      h("p", { class: "small muted" }, t("Certificado ICP-Brasil para gov.br, Receita, Justiça e emissores de nota: o A1 é um arquivo .pfx; o A3 fica num token USB ou cartão.")),
      body,
      actions));
}
