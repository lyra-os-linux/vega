// Estado compartilhado: preferências (as mesmas do vega-gtk, em
// ~/.config/vega-gtk/preferences.json), metadados do vegad e contadores da
// barra lateral.

import { call } from "./api.js";

export const prefs = {
  language: "system",
  start_page: "dashboard",
  confirm_actions: true,
  refresh_interval_minutes: 5,
  notify_updates: true,
  notify_service_failures: true,
  notify_backups: true,
  redact_ai_data: true,
  save_ai_history: true,
};

export async function loadPrefs() {
  Object.assign(prefs, await call("prefs_get"));
}

export async function savePrefs(changes) {
  Object.assign(prefs, changes);
  await call("prefs_set", { settings: { ...prefs } });
}

/** Metadados do vegad (perfil, versão, capacidades); null se indisponível. */
export const daemon = { metadata: null };

export async function loadMetadata() {
  try {
    daemon.metadata = await call("system_metadata");
  } catch {
    daemon.metadata = null;
  }
  return daemon.metadata;
}

export const hasCapability = (name) => Boolean(daemon.metadata?.capabilities?.includes(name));

const badgeListeners = new Set();
export const badges = {};

export function setBadge(page, value) {
  badges[page] = value;
  badgeListeners.forEach((fn) => fn(badges));
}

export function onBadges(fn) {
  badgeListeners.add(fn);
  return () => badgeListeners.delete(fn);
}

// Preferências só desta interface (não existem no vega-gtk).
const LOCAL_KEY = "vega-ui";
export const local = { density: "comfortable" };
try {
  Object.assign(local, JSON.parse(localStorage.getItem(LOCAL_KEY) ?? "{}"));
} catch { /* armazenamento indisponível */ }

export function saveLocal() {
  try {
    localStorage.setItem(LOCAL_KEY, JSON.stringify(local));
  } catch { /* armazenamento indisponível */ }
}
