// Modo estúdio da seção Música (vega#217). Liga os perfis do sistema do
// vegad (system-profiles-v1): "audio-lowlatency" instala o pipewire-jack,
// dá tempo real ao grupo audio e põe o usuário nele; "audio-lowlatency-kernel"
// é opcional e acrescenta preempt=full threadirqs ao boot. O vegad tira um
// snapshot antes e desfaz tudo se algo falhar.

import { call, on } from "../../api.js";
import { t } from "../../i18n.js";
import { hasCapability } from "../../store.js";
import { h, banner, button, confirm, toast, fail } from "../../ui.js";

const AUDIO = "audio-lowlatency";
const KERNEL = "audio-lowlatency-kernel";

let profiles = null;
let loading = null;
let running = null;

function load(repaint) {
  if (loading) return;
  loading = call("profiles_list")
    .then((rows) => { profiles = Object.fromEntries(rows.map((row) => [row.id, row])); })
    .catch(() => { profiles = {}; })
    .finally(() => { loading = null; repaint(); });
}

/** Pede a mudança ao vegad e espera o fim do job. */
function setProfile(id, enabled) {
  return new Promise((resolve, reject) => {
    let job = null;
    let early = null;
    const stop = on("profile-event", (event) => {
      if (event.type !== "finished") return;
      if (job === null) { early = event; return; }
      if (event.job !== job) return;
      stop();
      resolve(event);
    });
    call("profiles_set", { id, enabled })
      .then((id_) => {
        job = id_;
        if (early?.job === job) { stop(); resolve(early); }
      })
      .catch((error) => { stop(); reject(error); });
  });
}

async function change(changes, repaint, done) {
  running = true;
  repaint();
  try {
    for (const [id, enabled] of changes) {
      const event = await setProfile(id, enabled);
      if (event.result !== "ok") throw new Error(event.detail || t("O vegad desfez as mudanças"));
    }
    toast(done);
  } catch (error) {
    fail(error, t("Não foi possível mudar o modo estúdio"));
  } finally {
    running = null;
    profiles = null;
    repaint();
  }
}

async function turnOn(repaint) {
  const ok = await confirm({
    title: t("Ligar o modo estúdio?"),
    text: t("O Vega instala o JACK do PipeWire, dá prioridade de tempo real ao grupo audio e coloca você nele. Antes, cria um snapshot. Vale a partir do próximo login."),
    confirmLabel: t("Ligar"),
  });
  if (!ok) return;
  await change([[AUDIO, true]], repaint, t("Modo estúdio ligado. Saia e entre de novo na sessão para valer."));
}

async function turnOff(repaint) {
  const changes = [[AUDIO, false]];
  if (profiles?.[KERNEL]?.enabled) changes.push([KERNEL, false]);
  await change(changes, repaint, t("Modo estúdio desligado."));
}

async function toggleKernel(repaint) {
  const enabled = profiles?.[KERNEL]?.enabled;
  if (!enabled) {
    const ok = await confirm({
      title: t("Ajustar o kernel para áudio?"),
      text: t("Acrescenta preempt=full e threadirqs ao boot do sistema inteiro, como no Ubuntu Studio. Reduz a latência em gravações exigentes. O Vega cria um snapshot antes, e desligar restaura a linha de boot anterior. Vale depois de reiniciar."),
      confirmLabel: t("Ajustar o kernel"),
    });
    if (!ok) return;
  }
  await change([[KERNEL, !enabled]], repaint,
    enabled ? t("Ajuste do kernel desfeito. Reinicie para valer.") : t("Kernel ajustado. Reinicie para valer."));
}

/** Cartão do modo estúdio no topo da seção Música. */
export function studioBanner(repaint, busy) {
  if (!hasCapability("system-profiles-v1")) {
    return banner("info", "volume", t("Modo estúdio"), t("Atualize o Lyra OS para ligar o áudio de baixa latência pelo Vega."));
  }
  if (!profiles) {
    load(repaint);
    return banner("info", "volume", t("Modo estúdio"), t("Verificando…"));
  }
  const audio = profiles[AUDIO];
  const kernel = profiles[KERNEL];
  const disabled = busy || Boolean(running);
  if (!audio?.enabled) {
    return banner("info", "volume", t("Modo estúdio desligado"),
      t("Para gravar com pouca latência: JACK pelo PipeWire (o Ardour e o Qtractor abrem sem configurar nada) e prioridade de tempo real para o grupo audio, como no Ubuntu Studio."),
      button(running ? t("Ligando…") : t("Ligar o modo estúdio"), { kind: "primary", disabled, onClick: () => turnOn(repaint) }));
  }
  const notes = [];
  if (audio.restart_pending) notes.push(t("Saia e entre de novo na sessão para valer."));
  if (kernel?.restart_pending) notes.push(t("Reinicie para o ajuste do kernel valer."));
  return banner(notes.length ? "warn" : "success", "volume", t("Modo estúdio ligado"),
    [t("JACK pelo PipeWire e tempo real para o grupo audio."),
      kernel?.enabled ? t("Kernel ajustado para áudio (preempt=full, threadirqs).") : t("Opcional: ajuste o kernel para latência ainda menor."),
      ...notes].join(" "),
    h("div", { class: "row-flex" },
      button(kernel?.enabled ? t("Desfazer ajuste do kernel") : t("Ajustar o kernel"), { size: "sm", disabled, onClick: () => toggleKernel(repaint) }),
      button(running ? t("Aplicando…") : t("Desligar"), { size: "sm", disabled, onClick: () => turnOff(repaint) })));
}
