// Aviso "Reinicie o computador para concluir" (backend/power.rs).
//
// Algumas mudanças só valem depois de reiniciar: habilitar o Flatpak (o GNOME
// só lê a pasta dos apps do Flathub no login) e o suporte a programas de 32
// bits (ia32_emulation=1 no boot). Quem faz a mudança chama requireRestart():
// o Vega pergunta se reinicia agora e, se não, deixa uma faixa no topo de
// todas as telas. O pedido fica guardado com o boot_id em que foi feito;
// depois de reiniciar o boot_id muda e o aviso some sozinho.

import { call } from "./api.js";
import { N_, t } from "./i18n.js";
import { local, saveLocal } from "./store.js";
import { h, banner, button, confirm, fail } from "./ui.js";

/** Motivos conhecidos, com o texto mostrado na faixa e na pergunta. */
const REASONS = {
  flatpak: N_("os apps do Flathub aparecerem no menu"),
  ia32: N_("os programas de 32 bits do Windows funcionarem"),
};

let bootId = null;

/** Lê o boot_id atual e descarta um pedido feito antes do último reinício. */
export async function initRestart() {
  bootId = await call("system_boot_id").catch(() => null);
  const pending = local.restartPending;
  if (pending && (!bootId || pending.bootId !== bootId)) {
    delete local.restartPending;
    saveLocal();
  }
}

const pendingReasons = () => (bootId && local.restartPending?.bootId === bootId ? local.restartPending.reasons : []);

function reasonsText(reasons) {
  return reasons.map((reason) => t(REASONS[reason] ?? reason)).join(t(" e "));
}

async function reboot() {
  try {
    await call("system_reboot");
  } catch (error) {
    fail(error, t("Não foi possível reiniciar o computador."));
  }
}

/** Faixa do topo das telas, ou null quando não há reinício pendente. */
export function restartBanner() {
  const reasons = pendingReasons();
  if (!reasons.length) return null;
  return h("div", { class: "restart-banner", id: "restart-banner" },
    banner("warn", "restart", t("Reinicie o computador para concluir"),
      t("É preciso reiniciar para {reasons}.", { reasons: reasonsText(reasons) }),
      button(t("Reiniciar agora"), { kind: "primary", icon: "restart", onClick: reboot })));
}

function paintBanner() {
  document.getElementById("restart-banner")?.remove();
  const el = restartBanner();
  if (el) document.getElementById("page")?.prepend(el);
}

/**
 * Registra que `reason` precisa de reinício e pergunta se reinicia agora.
 * Sem boot_id legível, só pergunta (não há como saber quando o aviso vence).
 */
export async function requireRestart(reason) {
  if (bootId) {
    const reasons = new Set(pendingReasons());
    reasons.add(reason);
    local.restartPending = { bootId, reasons: [...reasons] };
    saveLocal();
    paintBanner();
  }
  const now = await confirm({
    title: t("Reiniciar o computador agora?"),
    text: t("É preciso reiniciar para {reasons}. Salve o que estiver aberto antes de continuar.", { reasons: reasonsText([reason]) }),
    ic: "restart",
    tone: "warn",
    confirmLabel: t("Reiniciar agora"),
    required: true,
  });
  if (now) await reboot();
}
