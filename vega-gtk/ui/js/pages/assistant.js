// Assistente de IA: chat com ferramentas e configurações/credenciais.
//
// A conversa vive no escopo do módulo, não da página: trocar de página no
// meio de uma resposta não perde nada, e ao voltar a página reflete o
// estado atual. Chaves ficam só no Secret Service (backend); ferramentas de
// leitura rodam no backend e voltam já embrulhadas como dado não confiável;
// as de mutação sempre passam por um diálogo obrigatório.

import { assistant, pick } from "../api.js";
import { N_, t } from "../i18n.js";
import { runSoftware } from "../tasks.js";
import { h, icon, clear, button, segmented, confirm, row, list } from "../ui.js";

const MUTATING = new Set(["install_package", "remove_package", "clear_package_cache"]);
const TOOL_PREFIX = "<dado_nao_confiavel";
const TEXT_EXTENSIONS = ["txt", "log", "md", "json", "yaml", "yml", "toml", "ini", "conf", "cfg", "xml", "csv", "sh", "py", "rs", "js", "ts", "html", "css", "service", "desktop", "repo", "spec"];
const IMAGE_EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];

/** `hidden` perde para o `display` das classes; esconde pelo estilo. */
function show(el, visible) {
  el.style.display = visible ? "" : "none";
}

// ---------- Estado da conversa (módulo) ----------

const state = {
  loaded: false,
  loading: null,
  busy: false,
  history: [],
  staged: [],
  draft: "",
  status: "",
  keyring: true,
  providers: [],
  saved: null, // configurações como estão no disco
  settings: null, // configurações em edição
  models: {}, // provedor -> lista
  typing: null, // índice da mensagem sendo "digitada"
};

const listeners = new Set();
const emit = (what = "all") => listeners.forEach((fn) => fn(what));
function setStatus(text) {
  state.status = text;
  emit("status");
}

function modelKey(provider) {
  return provider === "openai" ? "openai_model" : provider === "gemini" ? "gemini_model" : "anthropic_model";
}
const providerLabel = (id) => state.providers.find((p) => p.id === id)?.label ?? id;

async function load() {
  if (state.loaded) return;
  state.loading ??= (async () => {
    try {
      const data = await assistant("state");
      state.history = data.history ?? [];
      state.keyring = data.keyring;
      state.providers = data.providers;
      state.saved = data.settings;
      state.settings = structuredClone(data.settings);
      state.status = data.keyring
        ? t("Pronto • credenciais protegidas pelo Secret Service")
        : t("Secret Service indisponível: não será possível armazenar chaves");
      state.loaded = true;
      emit();
      refreshModels();
    } catch (error) {
      state.loading = null;
      setStatus(error.message);
    }
  })();
  return state.loading;
}

async function refreshModels() {
  const provider = state.settings.provider;
  state.refreshingModels = true;
  setStatus(t("Consultando modelos da {provider}…", { provider: providerLabel(provider) }));
  emit("settings");
  try {
    const models = await assistant("list_models", { provider });
    state.models[provider] = models;
    setStatus(t("{count} modelo(s) compatível(is) disponível(is)", { count: modelOptions(provider).length }));
  } catch (error) {
    setStatus(error.message);
  } finally {
    state.refreshingModels = false;
    emit("settings");
  }
}

/** Lista exibida: a do provedor, sempre incluindo o modelo escolhido. */
function modelOptions(provider) {
  const selected = state.settings?.[modelKey(provider)];
  const models = [...(state.models[provider] ?? [])];
  if (selected && !models.includes(selected)) models.unshift(selected);
  return [...new Set(models)].sort((a, b) => (a === selected ? -1 : b === selected ? 1 : a.localeCompare(b)));
}

// ---------- Conversa ----------

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Revela a resposta aos poucos (8 palavras a cada 18 ms), como no vega-gtk. */
async function appendProgressively(text) {
  const words = text.split(/(\s+)/);
  state.history.push({ role: "assistant", content: "", attachments: [] });
  const index = state.history.length - 1;
  state.typing = index;
  for (let i = 0; i < words.length; i += 16) {
    state.history[index].content = words.slice(0, i + 16).join("");
    emit("transcript");
    await sleep(18);
  }
  state.history[index].content = text;
  state.typing = null;
  emit("transcript");
}

function push(role, content, attachments = []) {
  state.history.push({ role, content, attachments });
  emit("transcript");
}

const audit = (kind, detail) => assistant("audit", { kind, detail }).catch(() => {});
const saveHistory = () => assistant("save_history", { messages: state.history }).catch((error) => setStatus(error.message));

async function handleTool(call) {
  if (MUTATING.has(call.name)) return handleMutation(call);
  try {
    const message = await assistant("tool", { name: call.name, input: call.input });
    push(message.role, message.content);
  } catch (error) {
    setStatus(error.message);
  }
}

async function handleMutation(call) {
  let plan;
  try {
    plan = await assistant("mutation_plan", { name: call.name, input: call.input });
  } catch (error) {
    setStatus(error.message);
    return;
  }
  // Propostas da IA sempre pedem confirmação, mesmo com confirmações desligadas.
  const ok = await confirm({
    title: plan.title,
    text: plan.description,
    ic: plan.destructive ? "trash" : "download",
    tone: plan.destructive ? "danger" : "",
    confirmLabel: plan.confirm,
    confirmKind: plan.destructive ? "danger" : "primary",
    facts: [{ icon: "sparkles", text: t("Proposta do Assistente de IA") }, { icon: "lock", text: t("A operação será autorizada pelo polkit e acompanhada até a conclusão.") }],
    required: true,
  });
  if (!ok) {
    push("assistant", t("A proposta foi rejeitada. Nenhuma alteração foi realizada."));
    assistant("mutation_result", { description: plan.description, outcome: "rejected" }).catch(() => {});
    return;
  }
  try {
    const { task } = await runSoftware(plan.action, plan.description);
    const transaction = task.key.split(":")[1];
    push("assistant", t("Ação aprovada e enviada ao vegad (transação #{transaction}).", { transaction }));
    assistant("mutation_result", { description: plan.description, outcome: "approved" }).catch(() => {});
  } catch (error) {
    setStatus(error.message);
    assistant("mutation_result", { description: plan.description, outcome: "error", detail: error.message }).catch(() => {});
  }
}

async function sendMessage() {
  if (state.busy || !state.loaded) return;
  const prompt = state.draft.trim();
  if (!prompt && !state.staged.length) return;
  state.busy = true;
  state.draft = "";
  const attachments = state.staged.splice(0);
  emit();
  for (const attachment of attachments) audit("user_attachment", attachment.name);
  push("user", prompt, attachments);
  audit("user_message", prompt);
  saveHistory();

  const settings = structuredClone(state.settings);
  try {
    await assistant("save_settings", { settings });
    state.saved = structuredClone(settings);
  } catch (error) {
    setStatus(error.message);
    state.busy = false;
    emit();
    return;
  }

  setStatus(t("Consultando o provedor…"));
  let input = 0;
  let output = 0;
  let cost = 0;
  let hasCost = false;
  const maxRounds = Math.min(20, Math.max(1, settings.max_rounds_per_message));
  for (let round = 0; round < maxRounds; round++) {
    setStatus(t("Pensando… etapa {step} de {total}", { step: round + 1, total: maxRounds }));
    let reply;
    try {
      reply = await assistant("round", { settings, history: state.history, first: round === 0 });
    } catch (error) {
      setStatus(error.message);
      break;
    }
    input += reply.input_tokens;
    output += reply.output_tokens;
    if (reply.estimated_cost_usd != null) {
      cost += reply.estimated_cost_usd;
      hasCost = true;
    }
    if (reply.text) {
      await appendProgressively(reply.text);
      audit("assistant_message", reply.text);
    }
    for (const call of reply.tool_calls) await handleTool(call);
    saveHistory();
    setStatus(t("{input} tokens de entrada • {output} de saída{cost}", {
      input, output, cost: hasCost ? t(" • estimativa US$ {value}", { value: cost.toFixed(6) }) : "",
    }));
    if (!reply.tool_calls.length) break;
  }
  state.busy = false;
  emit();
}

async function clearConversation() {
  if (state.busy) {
    setStatus(t("Aguarde a operação em andamento antes de limpar a conversa."));
    return;
  }
  state.busy = true;
  setStatus(t("Limpando conversa…"));
  emit();
  try {
    await assistant("clear_history");
    state.history = [];
    state.staged = [];
    state.draft = "";
    state.status = t("Conversa e anexos locais apagados. O registro de ações do sistema foi mantido.");
  } catch (error) {
    state.status = error.message;
  } finally {
    state.busy = false;
    emit();
  }
}

async function attachFiles() {
  if (state.busy) return;
  const chosen = await pick({
    multiple: true,
    title: t("Anexar arquivo ou imagem"),
    filters: [{ name: t("Imagens e arquivos de texto"), extensions: [...IMAGE_EXTENSIONS, ...TEXT_EXTENSIONS] }],
  });
  const paths = chosen == null ? [] : Array.isArray(chosen) ? chosen : [chosen];
  for (const path of paths) {
    try {
      state.staged.push(await assistant("read_attachment", { path }));
    } catch (error) {
      setStatus(error.message);
    }
  }
  emit("composer");
}

// ---------- Renderização segura do texto ----------

/** Negrito e código inline, sempre como nós de texto. */
function inline(text) {
  const nodes = [];
  const pattern = /(`[^`\n]+`|\*\*[^*\n]+\*\*)/g;
  let last = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > last) nodes.push(text.slice(last, match.index));
    const token = match[0];
    nodes.push(token.startsWith("`") ? h("code", {}, token.slice(1, -1)) : h("strong", {}, token.slice(2, -2)));
    last = match.index + token.length;
  }
  if (last < text.length) nodes.push(text.slice(last));
  return nodes;
}

/** Markdown simples (parágrafos, listas, títulos, blocos de código) em DOM. */
function richText(text) {
  const root = h("div", { class: "ai-rich selectable" });
  const lines = text.split("\n");
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (line.trim().startsWith("```")) {
      const code = [];
      i++;
      while (i < lines.length && !lines[i].trim().startsWith("```")) code.push(lines[i++]);
      i++;
      root.append(h("pre", { class: "ai-code" }, code.join("\n")));
      continue;
    }
    const bullet = /^\s*[-*•]\s+(.*)$/.exec(line);
    const numbered = /^\s*\d+[.)]\s+(.*)$/.exec(line);
    if (bullet || numbered) {
      const ordered = Boolean(numbered);
      const listEl = h(ordered ? "ol" : "ul");
      while (i < lines.length) {
        const match = ordered ? /^\s*\d+[.)]\s+(.*)$/.exec(lines[i]) : /^\s*[-*•]\s+(.*)$/.exec(lines[i]);
        if (!match) break;
        listEl.append(h("li", {}, inline(match[1])));
        i++;
      }
      root.append(listEl);
      continue;
    }
    const heading = /^\s*#{1,6}\s+(.*)$/.exec(line);
    if (heading) {
      root.append(h("p", { class: "ai-heading" }, inline(heading[1])));
      i++;
      continue;
    }
    if (!line.trim()) {
      i++;
      continue;
    }
    const paragraph = [];
    while (i < lines.length && lines[i].trim() && !/^\s*([-*•]|\d+[.)]|#{1,6})\s+/.test(lines[i]) && !lines[i].trim().startsWith("```")) paragraph.push(lines[i++]);
    const p = h("p");
    paragraph.forEach((part, index) => {
      if (index) p.append(h("br"));
      p.append(...inline(part));
    });
    root.append(p);
  }
  return root;
}

function attachmentChip(attachment, onRemove) {
  const image = attachment.mime?.startsWith("image/");
  const thumb = image
    ? h("img", { class: "ai-thumb", src: `data:${attachment.mime};base64,${attachment.data}`, alt: "" })
    : h("span", { class: "ai-thumb file" }, icon("logs", 15));
  return h("span", { class: "ai-chip", title: attachment.name },
    thumb,
    h("span", { class: "ai-chip-name" }, attachment.name),
    onRemove && h("button", { class: "icon-btn sm", "aria-label": t("Remover anexo"), title: t("Remover anexo"), onclick: onRemove }, icon("x", 13)));
}

/** Resultado de ferramenta: dado não confiável, mostrado recolhido. */
function toolMessage(content) {
  const name = /origem="tool:([^"]+)"/.exec(content)?.[1] ?? "tool";
  const body = content.replace(/^<dado_nao_confiavel[^>]*>\n?/, "").replace(/\n?<\/dado_nao_confiavel>[\s\S]*$/, "");
  const details = h("details", { class: "ai-tool" },
    h("summary", {}, icon("terminal", 14), h("span", {}, t("Resultado de ferramenta")), h("code", {}, name)),
    h("pre", { class: "ai-code selectable" }, body));
  return h("div", { class: "ai-msg tool" }, details);
}

function message(msg, index) {
  if (msg.role === "user" && msg.content.startsWith(TOOL_PREFIX)) return toolMessage(msg.content);
  const user = msg.role === "user";
  const typing = state.typing === index;
  return h("div", { class: `ai-msg ${user ? "user" : "assistant"}` },
    !user && h("span", { class: "ai-avatar", "aria-hidden": "true" }, icon("sparkles", 16)),
    h("div", { class: "ai-bubble" },
      h("div", { class: "ai-author" }, user ? t("Você") : "Lyra Vega"),
      msg.content && (user ? h("p", { class: "ai-plain selectable" }, msg.content) : richText(msg.content)),
      typing && h("span", { class: "ai-caret", "aria-hidden": "true" }),
      msg.attachments?.length > 0 && h("div", { class: "ai-chips" }, msg.attachments.map((a) => attachmentChip(a)))));
}

// ---------- Página ----------

export default {
  id: "assistant",
  title: N_("Assistente de IA"),
  icon: "sparkles",
  group: "main",
  keywords: ["ia", "ai", "chat", "assistente", "claude", "openai", "gemini", "llm"],
  sections: [
    { id: "chat", label: N_("Chat"), icon: "sparkles", keywords: ["conversa", "perguntar"] },
    { id: "settings", label: N_("Configurações e Credenciais"), icon: "key", keywords: ["chave", "api", "modelo", "provedor", "keyring"] },
  ],

  render(ctx) {
    let tab = ctx.sub === "settings" ? "settings" : "chat";
    const status = h("div", { class: "ai-status", role: "status", "aria-live": "polite" });
    const body = h("div");

    // Chat
    const transcript = h("div", { class: "ai-transcript", role: "log", "aria-live": "polite" });
    const chips = h("div", { class: "ai-chips staged" });
    const prompt = h("textarea", { class: "ai-input", rows: "1", placeholder: t("Pergunte ao assistente…"), "aria-label": t("Mensagem") });
    const attach = h("button", { class: "icon-btn", "aria-label": t("Anexar arquivo ou imagem"), title: t("Anexar arquivo ou imagem"), onclick: attachFiles }, icon("plus", 18));
    const send = h("button", { class: "btn primary ai-send", onclick: sendMessage });
    const composer = h("div", { class: "ai-composer" }, chips, h("div", { class: "ai-compose-row" }, attach, prompt, send));
    const hint = h("div", { class: "ai-hint small faint" }, h("kbd", {}, "Enter"), " ", t("Enviar"), " · ", h("kbd", {}, "Shift"), "+", h("kbd", {}, "Enter"), " ", t("nova linha"));
    const chat = h("div", { class: "card ai-chat" }, transcript, composer, hint);

    const autosize = () => {
      prompt.style.height = "auto";
      prompt.style.height = `${Math.min(220, prompt.scrollHeight)}px`;
    };
    prompt.addEventListener("input", () => {
      state.draft = prompt.value;
      autosize();
    });
    prompt.addEventListener("keydown", (e) => {
      if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
        e.preventDefault();
        sendMessage();
      }
    });

    const greeting = () => h("div", { class: "ai-greeting" },
      h("span", { class: "ai-greeting-art" }, icon("sparkles", 34)),
      h("p", {}, t("Olá! Posso explicar o estado do sistema e ajudar com tarefas no Lyra Vega. Configure um provedor e sua chave abaixo para começar.")),
      h("div", { class: "ai-suggestions" },
        [t("Há atualizações disponíveis?"), t("Como está o uso de disco?"), t("Mostre erros recentes do sistema")].map((text) =>
          h("button", { class: "ai-suggestion", onclick: () => { state.draft = text; prompt.value = text; autosize(); prompt.focus(); } }, text))),
      button(t("Configurações e Credenciais"), { kind: "flat", size: "sm", icon: "key", onClick: () => { tab = "settings"; tabs.select("settings"); draw(); } }));

    let stick = true;
    transcript.addEventListener("scroll", () => {
      stick = transcript.scrollHeight - transcript.scrollTop - transcript.clientHeight < 40;
    }, { passive: true });

    const paintTranscript = () => {
      clear(transcript, state.loaded && state.history.length === 0 ? greeting() : state.history.map(message),
        state.busy && state.typing == null && h("div", { class: "ai-msg assistant" }, h("span", { class: "ai-avatar" }, icon("sparkles", 16)), h("div", { class: "ai-bubble ai-thinking" }, h("i"), h("i"), h("i"))));
      if (stick) transcript.scrollTop = transcript.scrollHeight;
    };
    const paintComposer = () => {
      clear(chips, state.staged.map((attachment, index) => attachmentChip(attachment, () => { state.staged.splice(index, 1); paintComposer(); })));
      show(chips, state.staged.length > 0);
      if (prompt.value !== state.draft) {
        prompt.value = state.draft;
        autosize();
      }
      prompt.disabled = state.busy || !state.loaded;
      attach.disabled = state.busy || !state.loaded;
      send.disabled = state.busy || !state.loaded;
      clear(send, state.busy ? [h("span", { class: "spinner" }), t("Pensando…")] : [icon("arrow-up", 16), t("Enviar")]);
      clearButton.disabled = state.busy;
    };

    // Configurações e credenciais
    const settingsHost = h("div");
    const paintSettings = () => {
      if (!state.settings) return clear(settingsHost);
      const s = state.settings;
      const providerSelect = h("select", { class: "input", "aria-label": t("Provedor") },
        state.providers.map((p) => h("option", { value: p.id, selected: p.id === s.provider }, p.label)));
      providerSelect.addEventListener("change", () => {
        s.provider = providerSelect.value;
        // Como no vega-gtk: o modelo vem do que está salvo para esse provedor.
        s[modelKey(s.provider)] = state.saved[modelKey(s.provider)];
        keyInput.value = "";
        paintSettings();
        refreshModels();
      });
      const models = modelOptions(s.provider);
      const modelSelect = h("select", { class: "input ai-model", "aria-label": t("Modelo") },
        models.map((m) => h("option", { value: m, selected: m === s[modelKey(s.provider)] }, m)));
      modelSelect.addEventListener("change", () => (s[modelKey(s.provider)] = modelSelect.value));
      const refresh = h("button", { class: "icon-btn", "aria-label": t("Atualizar lista de modelos"), title: t("Atualizar lista de modelos"), disabled: state.refreshingModels, onclick: refreshModels },
        state.refreshingModels ? h("span", { class: "spinner" }) : icon("refresh", 16));
      const number = (key, min, max, label) => {
        const input = h("input", { class: "input ai-number", type: "number", min: String(min), max: String(max), step: "1", value: String(s[key]), "aria-label": label });
        input.addEventListener("change", () => {
          const value = Math.min(max, Math.max(min, Math.round(Number(input.value) || min)));
          input.value = String(value);
          s[key] = value;
        });
        return input;
      };
      const keyInput = h("input", { class: "input ai-key", type: "password", autocomplete: "off", spellcheck: "false", placeholder: t("Chave armazenada somente no keyring"), "aria-label": t("Chave de API") });
      const saveKey = button(t("Salvar chave"), { icon: "key", disabled: !state.keyring, onClick: async () => {
        const provider = s.provider;
        setStatus(t("Salvando chave no keyring…"));
        try {
          await assistant("save_key", { provider, key: keyInput.value });
          keyInput.value = "";
          setStatus(t("Chave salva com segurança no keyring"));
          refreshModels();
        } catch (error) {
          setStatus(error.message);
        }
      } });
      const removeKey = button(t("Remover chave"), { kind: "danger-soft", icon: "trash", disabled: !state.keyring, onClick: async () => {
        const provider = s.provider;
        const ok = await confirm({
          title: t("Remover chave de API?"),
          text: t("A chave de {provider} será apagada do keyring.", { provider: providerLabel(provider) }),
          ic: "key", tone: "danger", confirmLabel: t("Remover"), confirmKind: "danger", required: false,
        });
        if (!ok) return;
        try {
          await assistant("clear_key", { provider });
          setStatus(t("Chave removida do keyring"));
        } catch (error) {
          setStatus(error.message);
        }
      } });
      keyInput.addEventListener("keydown", (e) => { if (e.key === "Enter") saveKey.click(); });

      clear(settingsHost,
        h("div", { class: "ai-settings-head" },
          h("span", { class: "ai-settings-art" }, icon("shield-check", 22)),
          h("div", {}, h("h3", {}, t("Configurações e credenciais")), h("p", { class: "small muted" }, t("As chaves ficam no Secret Service e nunca em arquivos do Lyra Vega")))),
        list(
          row({ icon: "sparkles", title: t("Provedor"), suffix: providerSelect }),
          row({ icon: "layers", title: t("Modelo"), suffix: h("div", { class: "row-flex ai-model-row" }, modelSelect, refresh) }),
          row({ icon: "gauge", title: t("Limite diário"), suffix: number("max_messages_per_day", 1, 5000, t("Limite diário")) }),
          row({ icon: "refresh", title: t("Máximo de etapas"), suffix: number("max_rounds_per_message", 1, 20, t("Máximo de etapas")) }),
        ),
        h("div", { class: "row-flex ai-actions" },
          h("span", { class: "spacer" }),
          button(t("Salvar configurações"), { kind: "primary", icon: "check", onClick: async () => {
            try {
              await assistant("save_settings", { settings: s });
              state.saved = structuredClone(s);
              setStatus(t("Configurações salvas"));
            } catch (error) {
              setStatus(error.message);
            }
          } })),
        h("div", { class: "list ai-key-card" },
          row({ icon: "key", tint: state.keyring ? "tint" : "yellow", title: t("Chave de API"), subtitle: state.keyring ? t("Pronto • credenciais protegidas pelo Secret Service") : t("Secret Service indisponível: não será possível armazenar chaves"), wrap: true }),
          h("div", { class: "ai-key-row" }, keyInput, saveKey, removeKey)),
      );
    };

    const clearButton = button(t("Limpar conversa"), { icon: "trash", size: "sm", onClick: clearConversation });

    const paintStatus = () => {
      clear(status, state.status && [state.busy ? h("span", { class: "spinner" }) : icon("info", 14), h("span", { class: "selectable" }, state.status)]);
      show(status, Boolean(state.status));
    };

    const draw = () => {
      clear(body, tab === "chat" ? chat : h("div", { class: "ai-settings" }, settingsHost));
      show(clearButton, tab === "chat");
    };
    const tabs = segmented(
      [{ id: "chat", label: t("Chat"), icon: "sparkles" }, { id: "settings", label: t("Configurações e Credenciais"), icon: "key" }],
      tab,
      (id) => { tab = id; draw(); if (id === "chat") requestAnimationFrame(() => prompt.focus()); },
    );

    const onChange = (what) => {
      if (what === "status") return paintStatus();
      if (what === "transcript") return paintTranscript();
      if (what === "composer") return paintComposer();
      if (what === "settings") return paintSettings();
      paintStatus();
      paintTranscript();
      paintComposer();
      paintSettings();
    };
    listeners.add(onChange);
    ctx.cleanup(() => listeners.delete(onChange));
    onChange("all");
    draw();
    load();
    if (tab === "chat") requestAnimationFrame(() => prompt.focus());

    const el = h("div", { class: "ai-page" },
      h("div", { class: "page-hero" },
        h("div", {}, h("h2", {}, t("Assistente de IA")), h("p", {}, t("Orientação contextual com privacidade, limites e aprovação explícita para alterações"))),
        tabs),
      status,
      body);
    return { el, actions: [clearButton] };
  },
};
