// Painel de preparação dos repositórios (ui/preparation.rs do vega-gtk):
// consulta o vegad a cada 5 s enquanto a página está visível, mostra fase,
// erro e horários, e oferece "Tentar novamente" e "Revisar chave…".

import { call } from "../../api.js";
import { t } from "../../i18n.js";
import { runSoftware } from "../../tasks.js";
import { h, icon, clear, button, confirm, dateTime } from "../../ui.js";

const POLL_MS = 5000;

function localTime(value) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : dateTime(date);
}

/** Título e linhas de detalhe; null esconde o painel. */
export function presentation(status) {
  const titles = {
    pending: t("Preparação dos repositórios pendente"),
    running: t("Preparando os repositórios"),
    "waiting-retry": t("Aguardando nova tentativa"),
    "awaiting-approval": t("Uma chave de assinatura precisa de aprovação"),
    failed: t("A preparação dos repositórios não terminou"),
    completed: t("Repositórios preparados"),
  };
  if (status.state === "unavailable" || status.state === "skipped") return null;
  const title = titles[status.state] ?? t("Estado da preparação indisponível");
  const parts = [];
  if (status.state === "running") {
    parts.push({
      "importing-keys": t("Conferindo as chaves de assinatura autorizadas…"),
      refreshing: t("Atualizando os metadados dos repositórios…"),
      "listing-updates": t("Consultando as atualizações disponíveis…"),
      publishing: t("Registrando o resultado da preparação…"),
    }[status.phase] ?? t("Iniciando a preparação…"));
  } else if (status.state === "completed") {
    parts.push(t("Os repositórios estão prontos. Você pode escolher quais atualizações instalar."));
  } else if (status.state === "pending") {
    parts.push(t("A preparação ainda não começou."));
  }
  if (status.state !== "running" && status.state !== "completed") {
    const errors = {
      network: t("Verifique a conexão de rede. Não foi possível acessar os repositórios."),
      "untrusted-key": t("É necessário revisar e aprovar a chave do repositório. Apenas aguardar ou tentar novamente não autoriza a chave."),
      interrupted: t("A preparação foi interrompida. Você pode tentar novamente."),
      "operation-failed": t("O serviço não concluiu a preparação. Tente novamente ou consulte o registro do serviço."),
      "service-failed": t("O serviço não concluiu a preparação. Tente novamente ou consulte o registro do serviço."),
    };
    if (errors[status.error_kind]) parts.push(errors[status.error_kind]);
    else if (status.last_error) parts.push(status.last_error);
  }
  if (status.updated_at) parts.push(t("Última atualização: {time}", { time: localTime(status.updated_at) }));
  if (status.next_retry_at) parts.push(t("Próxima tentativa automática, aproximadamente: {time}", { time: localTime(status.next_retry_at) }));
  return { title, parts };
}

const TONE = {
  running: ["", "refresh"],
  completed: ["success", "check-circle"],
  pending: ["", "clock"],
  "waiting-retry": ["warn", "clock"],
  "awaiting-approval": ["warn", "key"],
  failed: ["danger", "alert"],
};

/** Monta o painel; devolve o elemento (vazio quando não há o que mostrar). */
export function preparationPanel(ctx) {
  const root = h("div", { class: "sw-prep" });
  const state = { status: undefined, key: null, keyError: false, error: false, busy: false, feedback: "" };

  const paint = () => {
    if (state.error) {
      return clear(root, card("danger", "alert-circle", t("Estado da preparação indisponível"), [t("Não foi possível consultar o serviço. A conexão será verificada novamente.")]));
    }
    const shown = state.status && presentation(state.status);
    if (!shown) return clear(root);
    const [tone, ic] = TONE[state.status.state] ?? ["", "info"];
    const actions = [];
    if (state.status.can_retry) actions.push(button(t("Tentar novamente"), { icon: "refresh", size: "sm", disabled: state.busy, onClick: retry }));
    if (state.key) actions.push(button(t("Revisar chave…"), { icon: "key", size: "sm", kind: "primary", disabled: state.busy, onClick: review }));
    const feedback = [];
    if (state.keyError) feedback.push(t("Não foi possível carregar a chave pendente. A consulta será repetida."));
    if (state.feedback && !["running", "completed"].includes(state.status.state)) feedback.push(state.feedback);
    clear(root, card(tone, ic, shown.title, shown.parts, actions, feedback, state.status.state === "running"));
  };

  const poll = async () => {
    if (!ctx.visible() || state.busy) return;
    state.busy = true;
    try {
      const status = await call("preparation_status");
      state.error = false;
      state.status = status;
      if (status?.state === "awaiting-approval") {
        try {
          state.key = (await call("preparation_pending_keys"))[0] ?? null;
          state.keyError = false;
        } catch {
          state.key = null;
          state.keyError = true;
        }
      } else {
        state.key = null;
        state.keyError = false;
      }
    } catch {
      state.error = true;
    } finally {
      state.busy = false;
    }
    paint();
  };

  async function retry() {
    if (state.busy) return;
    state.busy = true;
    state.feedback = t("Solicitando autorização…");
    paint();
    try {
      await call("preparation_retry");
      state.feedback = t("Nova tentativa solicitada. Acompanhando o serviço…");
    } catch (error) {
      state.feedback = t("Não foi possível solicitar uma nova tentativa: {detail}", { detail: error.message });
    }
    state.busy = false;
    poll();
  }

  async function review() {
    const key = state.key;
    if (!key || state.busy) return;
    const ok = await confirm({
      title: t("Confiar na chave do repositório?"),
      ic: "key",
      tone: "warn",
      extra: h("p", { class: "sw-pre selectable" }, t("Repositório: {repo}\nAssinante: {user}\nImpressão digital completa: {fingerprint}\n\nConfira a impressão digital com o responsável pelo repositório antes de confiar. A aprovação retomará a preparação.", { repo: key.repo, user: key.user_id, fingerprint: key.fingerprint })),
      confirmLabel: t("Confiar"),
      required: true,
    });
    if (!ok) return;
    state.busy = true;
    state.feedback = t("Solicitando autorização…");
    paint();
    try {
      const { done } = await runSoftware({ kind: "approve-preparation-key", repo: key.repo, fingerprint: key.fingerprint, user_id: key.user_id, token: key.token }, t("Confiar na chave do repositório?"));
      const result = await done;
      state.feedback = result.success ? t("Chave aprovada. Acompanhando a preparação…") : t("Não foi possível aprovar a chave: {detail}", { detail: result.message });
    } catch (error) {
      state.feedback = t("Não foi possível aprovar a chave: {detail}", { detail: error.message });
    }
    state.busy = false;
    poll();
  }

  poll();
  const timer = setInterval(poll, POLL_MS);
  ctx.cleanup(() => clearInterval(timer));
  return root;
}

function card(tone, ic, title, parts, actions = [], feedback = [], spinning = false) {
  return h("div", { class: `banner sw-prep-card ${tone}`.trim() },
    spinning ? h("span", { class: "ic" }, h("span", { class: "spinner" })) : icon(ic, 22),
    h("div", { class: "body" },
      h("strong", {}, title),
      parts.map((line) => h("span", { class: "sw-prep-line selectable" }, line)),
      feedback.map((line) => h("span", { class: "sw-prep-line sw-prep-feedback" }, line))),
    actions.length > 0 && h("div", { class: "row-flex sw-prep-actions" }, actions));
}
