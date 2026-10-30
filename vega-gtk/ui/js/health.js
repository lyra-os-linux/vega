// Estado de saúde do sistema, atualizado em segundo plano como o refresh do
// vega-gtk: ao abrir o Painel, a cada `refresh_interval_minutes` e quando o
// vegad anuncia atualizações. Também envia a notificação de falha em
// serviços e mantém os contadores da barra lateral.

import { call, on } from "./api.js";
import { t } from "./i18n.js";
import { prefs, setBadge } from "./store.js";

export const health = {
  loaded: false,
  status: null, // { version, distro }
  updates: null, // lista de pacotes
  backups: null, // configurações
  snapshots: null, // { available, count }
  services: null, // serviços habilitados mas parados
  disk: null, // { used, total, percent }
  errors: {},
};

const listeners = new Set();
export function onHealth(fn) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}
const emit = () => listeners.forEach((fn) => fn(health));

const settle = async (key, promise) => {
  try {
    health[key] = await promise;
    delete health.errors[key];
  } catch (error) {
    health[key] = null;
    health.errors[key] = error.message;
  }
};

let lastStruggling = null;

async function refreshServices() {
  await settle("services", call("services_list", { all: false }).then((list) => list.filter((s) => s.available && s.enabled && !s.active)));
  const count = health.services?.length ?? 0;
  setBadge("services", { value: count, tone: "warn" });
  if (health.services && count !== lastStruggling) {
    if (count > 0 && prefs.notify_service_failures) {
      call("notify", { id: "service-failures", summary: t("Falha em serviços"), body: t("{count} serviço(s) requer(em) atenção", { count }) });
    }
    lastStruggling = count;
  }
}

export async function refreshUpdates() {
  await settle("updates", call("software_updates"));
  setBadge("software", { value: health.updates?.length ?? 0 });
  emit();
}

async function refreshSnapshots() {
  const available = await call("snapshots_available");
  if (!available) return { available, count: 0 };
  const list = await call("snapshots_list");
  return { available, count: list.filter((s) => s.id !== 0).length };
}

/** Depois de liberar espaço: o uso do disco e a contagem de snapshots mudam. */
export async function refreshDisk() {
  await Promise.all([settle("disk", call("system_disk_usage")), settle("snapshots", refreshSnapshots())]);
  emit();
}

// Uma atualização em voo e no máximo uma pendente (refresh.rs do vega-gtk).
let inFlight = null;
let pending = false;
export function refresh() {
  if (inFlight) {
    pending = true;
    return inFlight;
  }
  inFlight = (async () => {
    await Promise.all([
      settle("status", call("system_status")),
      refreshUpdates(),
      settle("backups", call("backup_configs")),
      settle("snapshots", refreshSnapshots()),
      refreshServices(),
      settle("disk", call("system_disk_usage")),
    ]);
    health.loaded = true;
    emit();
  })().finally(() => {
    inFlight = null;
    if (pending) {
      pending = false;
      refresh();
    }
  });
  return inFlight;
}

export function startHealth() {
  refresh();
  // Como o vega-gtk: um relógio de 60 s dispara a cada N minutos.
  let minutes = 0;
  setInterval(() => {
    minutes += 1;
    if (minutes >= Math.min(60, Math.max(1, prefs.refresh_interval_minutes))) {
      minutes = 0;
      refresh();
    }
  }, 60_000);
  on("updates-available", () => refreshUpdates());
}
