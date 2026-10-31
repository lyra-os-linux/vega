// Flatpak opcional (controle parental do Lyra OS 16.1): o flatpak não vem
// instalado. Este painel aparece enquanto ele estiver ausente e o habilita
// instalando o pacote; o Flathub vem de /etc/flatpak/remotes.d. O aviso sobre
// contas supervisionadas foi aprovado pelo mantenedor em 26/10/2026.

import { call } from "../../api.js";
import { t } from "../../i18n.js";
import { runSoftware } from "../../tasks.js";
import { requireRestart } from "../../restart.js";
import { h, clear, button, banner, confirm, toast, fail } from "../../ui.js";

const SUPERVISED_GROUP = "lyra-supervised";

async function hasSupervisedAccounts() {
  try {
    const users = await call("users_list");
    return users.some((user) => user.groups.includes(SUPERVISED_GROUP));
  } catch {
    // Na dúvida, mostra o aviso.
    return true;
  }
}

/** Texto do diálogo; o parágrafo de atenção só aparece com conta supervisionada. */
export function enableDialog(supervised) {
  return {
    title: t("Habilitar Flatpak e Flathub?"),
    text: t("Isso instala o Flatpak e ativa a loja Flathub, com milhares de aplicativos."),
    ic: "package",
    tone: supervised ? "warn" : "",
    confirmLabel: t("Habilitar"),
    extra: supervised
      ? h("p", { class: "small" },
          h("strong", {}, t("Atenção: este computador tem contas supervisionadas.")), " ",
          t("Com o Flatpak ativo, uma criança com conhecimentos técnicos consegue instalar aplicativos Flatpak na própria conta e abrir aplicativos Flatpak que você bloqueou no Controle Parental. As demais restrições continuam valendo."))
      : undefined,
  };
}

/** Painel "Habilitar Flatpak"; `onEnabled` recarrega a página depois. */
export function flatpakPanel(onEnabled) {
  const host = h("div", { class: "sw-flatpak" });
  let running = false;

  async function enable() {
    const ok = await confirm(enableDialog(await hasSupervisedAccounts()));
    if (!ok) return;
    running = true;
    paint();
    try {
      const { done } = await runSoftware({ kind: "install", origin: "official", id: "flatpak" }, t("Habilitando o Flatpak…"));
      const result = await done;
      running = false;
      if (result.success) {
        clear(host);
        // O GNOME Shell só lê /var/lib/flatpak/exports/share a partir do
        // próximo login (/etc/profile.d/flatpak.sh): até lá, apps do Flathub
        // instalados não aparecem no menu.
        toast(t("Flatpak habilitado."));
        onEnabled?.();
        await requireRestart("flatpak");
        return;
      }
      fail(new Error(result.message || t("Não foi possível habilitar o Flatpak.")));
    } catch (error) {
      running = false;
      fail(error);
    }
    paint();
  }

  function paint() {
    clear(host, banner("info", "package", t("Flatpak desabilitado"),
      t("Os aplicativos do Flathub ficam disponíveis depois de habilitar o Flatpak."),
      button(running ? t("Habilitando…") : t("Habilitar Flatpak"), { kind: "primary", disabled: running, onClick: enable })));
  }

  call("software_details", { origin: "official", id: "flatpak" })
    .then((details) => { if (!details?.installed) paint(); })
    .catch(() => {});
  return host;
}
