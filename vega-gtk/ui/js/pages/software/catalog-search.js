// Busca no catálogo curado (vega#207). A aba Buscar procura primeiro nos
// cartões de todas as vitrines, sem rede, e só depois no zypper e no Flatpak.
//
// Cada cartão pode levar `tags`: sinônimos e nomes pelos quais as pessoas
// procuram o app ("WhatsApp", "alternativa ao Photoshop", "Redis"), marcados
// com N_ e traduzidos como os outros textos. A busca casa cada palavra digitada
// com o começo de uma palavra do nome, do id, do texto, das tags, da seção ou
// da aba, sem diferenciar maiúsculas nem acentos, no português e no idioma
// ativo. `tasks` (na seção ou no cartão) liga o cartão aos filtros de TASKS.

import { N_, t } from "../../i18n.js";

/** Filtros rápidos por tarefa da aba Buscar. */
export const TASKS = [
  { id: "record", label: N_("Gravar"), icon: "camera" },
  { id: "edit", label: N_("Editar"), icon: "palette" },
  { id: "code", label: N_("Programar"), icon: "terminal" },
  { id: "study", label: N_("Estudar"), icon: "logs" },
  { id: "play", label: N_("Jogar"), icon: "zap" },
];

// Mesma normalização de ui.js (sem importá-lo: o módulo fica testável no node).
const normalize = (text) => String(text).normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
const words = (text) => normalize(text).split(/[^\p{L}\p{N}+#]+/u).filter(Boolean);
const matches = (list, token) => list.some((word) => word.startsWith(token));

/**
 * Achata as vitrines em entradas { app, tab, label, section, tasks }.
 * `showcases`: [{ tab, label, catalog }], na ordem das abas.
 */
export function curatedEntries(showcases) {
  return showcases.flatMap(({ tab, label, catalog }) => catalog.filter((section) => section.apps).flatMap((section) =>
    section.apps.map((app) => ({ app, tab, label, section, tasks: new Set([...(section.tasks ?? []), ...(app.tasks ?? [])]) }))));
}

/**
 * Cartões que casam com `query` e com a tarefa `task` (ambos opcionais), sem
 * repetir o app que aparece em mais de uma aba. Os que casam pelo nome vêm
 * antes, depois os que casam pelas tags; o resto segue a ordem das abas.
 */
export function searchCatalog(entries, query, task) {
  const tokens = words(query ?? "");
  const seen = new Set();
  const found = [];
  for (const entry of entries) {
    const { app, section, label } = entry;
    if (task && !entry.tasks.has(task)) continue;
    const key = `${app.origin}\u0000${app.id}`;
    if (seen.has(key)) continue;
    let score = 0;
    if (tokens.length) {
      const name = words(`${app.name} ${app.id}`);
      const tags = words((app.tags ?? []).flatMap((tag) => [tag, t(tag)]).join(" "));
      const rest = words([app.text, section.title, label].flatMap((text) => [text, t(text)]).join(" "));
      if (!tokens.every((token) => matches(name, token) || matches(tags, token) || matches(rest, token))) continue;
      score = tokens.reduce((sum, token) => sum + (matches(name, token) ? 2 : matches(tags, token) ? 1 : 0), 0);
    }
    seen.add(key);
    found.push({ entry, score });
  }
  return found.sort((a, b) => b.score - a.score).map(({ entry }) => entry);
}
