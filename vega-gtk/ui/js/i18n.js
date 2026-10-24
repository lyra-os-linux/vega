// Tradução: os textos da interface são os mesmos msgids em português do
// catálogo gettext `vega-gtk` (po/*.po). O backend entrega o catálogo do
// locale ativo uma vez; `t()` é síncrono.

let catalog = {};
export let locale = "pt-BR";

export async function loadCatalog(invoke, activeLocale) {
  locale = (activeLocale || "pt_BR").replace("_", "-");
  document.documentElement.lang = locale;
  try {
    catalog = (await invoke("i18n_catalog")) ?? {};
  } catch {
    catalog = {};
  }
}

/** Traduz e substitui `{nome}` pelos valores de `vars`. */
export function t(msgid, vars) {
  let text = catalog[msgid] || msgid;
  if (vars) for (const [key, value] of Object.entries(vars)) text = text.replaceAll(`{${key}}`, String(value));
  return text;
}

/** Marca um texto para extração sem traduzir agora (traduzido depois com `t`). */
export const N_ = (msgid) => msgid;
