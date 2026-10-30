import { call } from "../api.js";
import { N_, t } from "../i18n.js";
import { h, icon, clear, button, badge, section, banner, skeletonRows, empty, confirm, meter, ring } from "../ui.js";
import { cleanupSection } from "./storage-cleanup.js";

/** Partições e volumes agrupados sob o disco ao qual pertencem (`nvme0n1p2` → `nvme0n1`). */
function groupByDisk(volumes) {
  const disks = volumes.filter((v) => v.kind === "disk");
  const groups = disks.map((disk) => ({ disk, children: [] }));
  const loose = [];
  for (const volume of volumes) {
    if (volume.kind === "disk") continue;
    const parent = groups
      .filter((g) => volume.name.startsWith(g.disk.name))
      .sort((a, b) => b.disk.name.length - a.disk.name.length)[0];
    (parent ? parent.children : loose).push(volume);
  }
  return { groups, loose };
}

function usage(volume) {
  if (!volume.size || !volume.used) return null;
  return t("{used} usados de {size} • {percent}%", { used: volume.used, size: volume.size, percent: volume.use_percent });
}

function volumeIcon(volume) {
  if (volume.fs_type === "swap") return "layers";
  if (volume.removable) return "plug";
  if (volume.mountpoint === "/" || volume.mountpoint === "/boot/efi") return "star";
  return volume.kind === "disk" ? "disk" : "folder";
}

export default {
  id: "storage",
  title: N_("Armazenamento"),
  icon: "disk",
  group: "system",
  keywords: ["disco", "ssd", "particao", "montar", "desmontar", "pendrive", "volume", "espaco", "liberar", "limpar", "lixeira", "cache", "limpeza"],

  render(ctx) {
    const body = h("div", {}, skeletonRows(4));
    const status = h("p", { class: "small faint hw-status selectable" });

    async function act(volume) {
      const unmount = volume.can_unmount;
      const verb = unmount ? t("Desmontar") : t("Montar");
      const ok = await confirm({
        title: t("{verb} volume?", { verb }),
        text: `${volume.path} (${volume.mountpoint || t("não montado")})`,
        ic: "disk",
        confirmLabel: verb,
        confirmKind: unmount ? "" : "primary",
        required: false,
      });
      if (!ok) return;
      status.textContent = t("{verb} {path}…", { verb, path: volume.path });
      try {
        await call(unmount ? "storage_unmount" : "storage_mount", { path: volume.path });
        await load();
      } catch (error) {
        status.textContent = error.message;
      }
    }

    function volumeRow(volume) {
      const used = usage(volume);
      const details = [volume.kind, volume.fs_type || t("sem filesystem")].join(" • ");
      const mounted = volume.mountpoint ? t("Montado em {mountpoint}", { mountpoint: volume.mountpoint }) : t("Não montado");
      const action = (volume.can_mount || volume.can_unmount) &&
        button(volume.can_unmount ? t("Desmontar") : t("Montar"), { size: "sm", kind: volume.can_unmount ? "" : "primary", onClick: () => act(volume) });
      return h("div", { class: "list-row volume-row" },
        h("span", { class: `lead ${volume.mountpoint ? "tint" : ""}`.trim() }, icon(volumeIcon(volume), 18)),
        h("div", { class: "body" },
          h("div", { class: "title" }, volume.name, h("span", { class: "mono faint volume-path" }, volume.path)),
          h("div", { class: "subtitle selectable" }, `${details} • ${mounted}`),
          used && h("div", { class: "volume-usage" }, meter(volume.use_percent / 100), h("span", { class: "small muted" }, used))),
        h("div", { class: "suffix" }, volume.removable && badge(t("Removível"), "yellow"), volume.fs_type && badge(volume.fs_type, "outline"), action));
    }

    function summary(volumes) {
      const main = volumes.find((v) => v.mountpoint === "/") ??
        volumes.filter((v) => v.fs_type && v.fs_type !== "swap" && v.fs_type !== "vfat" && v.used).sort((a, b) => b.use_percent - a.use_percent || b.size.localeCompare(a.size))[0];
      if (!main) return null;
      return h("div", { class: "card pad storage-summary" },
        ring(main.use_percent / 100, { size: 92, stroke: 10 }),
        h("div", { class: "spacer" },
          h("div", { class: "small faint" }, `${main.name} • ${main.fs_type}`),
          h("h3", {}, t("{used} usados de {size} • {percent}%", { used: main.used, size: main.size, percent: main.use_percent })),
          h("p", { class: "small muted" }, main.mountpoint ? t("Montado em {mountpoint}", { mountpoint: main.mountpoint }) : t("Não montado"))));
    }

    async function load() {
      let volumes;
      try {
        volumes = await call("storage_list");
      } catch (error) {
        clear(body, banner("danger", "alert-circle", t("Armazenamento"), error.message));
        return;
      }
      if (!volumes.length) {
        clear(body, empty("disk", t("Nenhum volume detectado")));
        return;
      }
      const { groups, loose } = groupByDisk(volumes);
      clear(body,
        summary(volumes),
        groups.map(({ disk, children }) =>
          section(disk.model || disk.name, {
            sub: [disk.size, disk.path].filter(Boolean).join(" • "),
            // Sem partições, a própria linha do disco já traz a ação.
            actions: children.length ? [disk.removable && badge(t("Removível"), "yellow"), (disk.can_mount || disk.can_unmount) && button(disk.can_unmount ? t("Desmontar") : t("Montar"), { size: "sm", onClick: () => act(disk) })] : null,
          }, h("div", { class: "list" }, (children.length ? children : [disk]).map(volumeRow)))),
        loose.length > 0 && section(t("Armazenamento"), {}, h("div", { class: "list" }, loose.map(volumeRow))),
        status);
    }

    load();
    const cleanup = cleanupSection(ctx, load);
    return {
      el: h("div", {}, h("div", { class: "page-hero" }, h("div", {}, h("h2", {}, t("Armazenamento")), h("p", {}, t("Volumes e pontos de montagem detectados pelo vegad")))), body, cleanup),
      actions: [h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: () => { clear(body, skeletonRows(4)); load(); } }, icon("refresh", 17))],
    };
  },
};
