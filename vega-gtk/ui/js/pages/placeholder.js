// Página provisória enquanto a migração para Tauri não chega a esta tela.
import { t } from "../i18n.js";
import { empty } from "../ui.js";

export function placeholder(page) {
  return { ...page, render: () => ({ el: empty("info", t(page.title), t("Em migração para a nova interface.")) }) };
}
