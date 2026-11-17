// Seção "Bancos" da vitrine Populares (vega#210): os webapps dos bancos.
//
// O módulo de segurança dos bancos (Warsaw) não tem cartão próprio: os
// webapps o gerenciam. Instalar o primeiro webapp instala o módulo; abrir um
// webapp liga o `warsaw.service` e fechá-lo desliga (vega-bank-webapp). A
// seção só fala do módulo quando há algo a resolver: módulo faltando com
// webapps instalados, instalação incoerente ou serviço preso ao ligar.
//
// O estado vem de `bank_module_status` (backend/banks.rs), que só lê. Instalar
// e remover usam a transação de pacote oficial da loja. O warsaw fica num
// repositório só dele (home:rodrigosbrito:warsaw), fora dos repositórios do
// Lyra: o Vega adiciona esse repositório, com a chave aprovada pelo usuário,
// como no cartão do Edge.
//
// "Reparar" (warsaw#12, rollback do Snapper com /usr/local fora do snapshot)
// também usa só a transação da loja, sem mudar o contrato: com restos sem
// pacote, instala o pacote por cima; com o pacote incompleto, remove e instala
// de novo.

import { call } from "../../api.js";
import { t } from "../../i18n.js";
import { h, button, banner, section, confirm, fail } from "../../ui.js";

const PACKAGE = "warsaw";
/** Os webapps rodam no Chromium em modo app (vega-bank-webapp e vega-webapp), do repo-oss. */
const BROWSER = "chromium";
const UNIT = "warsaw.service";
const REPO = { name: "lyra-warsaw", url: "https://download.opensuse.org/repositories/home:/rodrigosbrito:/warsaw/openSUSE_Leap_16.1/" };

// Estado que sobrevive à troca de aba, como `memory` em software.js.
const state = { module: null, error: null, loading: false, acting: false };

const packageStep = (transact, kind) => transact({ kind, origin: "official", id: PACKAGE },
  t("{verb} {name}", { verb: kind === "remove" ? t("Remover") : t("Instalar"), name: t("módulo de segurança dos bancos") }), { reload: false });

/** Instala ou repara o módulo pelo repositório dele. */
async function installModule({ transact, ensureRepo }, health) {
  if (!(await ensureRepo(REPO))) return false;
  if (health === "broken" && !(await packageStep(transact, "remove"))?.success) return false;
  return Boolean((await packageStep(transact, "install"))?.success);
}

const installBrowser = (deps) => deps.transact({ kind: "install", origin: "official", id: BROWSER }, t("Instalando {name}…", { name: "Chromium" }), { reload: false });

/**
 * Antes de instalar um web app de site (vega#203): ele só precisa do
 * Chromium. Se faltar, pede confirmação e instala dos repositórios do
 * openSUSE. Devolve true se o Chromium ficou instalado.
 */
export async function ensureChromium(deps) {
  let browser;
  try {
    browser = await call("software_details", { origin: "official", id: BROWSER });
  } catch (error) {
    fail(error, t("Não foi possível ler o estado do Chromium"));
    return false;
  }
  if (browser?.installed) return true;
  const ok = await confirm({
    title: t("Instalar o Chromium?"),
    text: t("Os web apps abrem o site numa janela própria do Chromium. O Vega instala o Chromium dos repositórios do openSUSE."),
    ic: "download",
    confirmLabel: t("Instalar"),
    required: true,
  });
  return ok && Boolean((await installBrowser(deps))?.success);
}

/**
 * Antes de instalar um webapp de banco: todo webapp liga o módulo ao abrir,
 * então o módulo precisa estar instalado e íntegro. Se faltar (ou estiver
 * incompleto), pede confirmação e instala pelo repositório do módulo.
 * Devolve true se o módulo ficou pronto para uso.
 */
export async function ensureBankModule(deps) {
  let module;
  let browser;
  try {
    [module, browser] = await Promise.all([
      call("bank_module_status"),
      call("software_details", { origin: "official", id: BROWSER }).catch(() => null),
    ]);
  } catch (error) {
    fail(error, t("Não foi possível ler o estado do módulo"));
    return false;
  }
  const needsModule = ["absent", "leftovers", "broken"].includes(module.health);
  const needsBrowser = !browser?.installed;
  if (!needsModule && !needsBrowser) return true;
  if (!needsModule) {
    const ok = await confirm({
      title: t("Instalar o Chromium?"),
      text: t("Os webapps de bancos abrem o site numa janela própria do Chromium. O Vega instala o Chromium dos repositórios do openSUSE."),
      ic: "download",
      confirmLabel: t("Instalar"),
      required: true,
    });
    return ok && Boolean((await installBrowser(deps))?.success);
  }
  const ok = await confirm({
    title: t("Habilitar a segurança de bancos?"),
    text: t("Os webapps de bancos ligam o módulo de segurança (Warsaw) antes de abrir o site. Ele ainda não está instalado, e o Vega instala agora.")
      + " " + t("O Vega adiciona o repositório do módulo, só com o Warsaw, e mostra a chave de assinatura para você aprovar.")
      + (needsBrowser ? " " + t("Também instala o Chromium, onde os webapps abrem numa janela própria.") : ""),
    ic: "shield",
    confirmLabel: t("Habilitar"),
    required: true,
  });
  if (!ok) return false;
  state.module = null;
  if (!(await installModule(deps, module.health))) return false;
  if (!needsBrowser) return true;
  return Boolean((await installBrowser(deps))?.success);
}

/** Botão Atualizar da página: lê o estado de novo no próximo desenho. */
export function resetBankSection() {
  if (state.loading || state.acting) return;
  state.module = null;
  state.error = null;
}

/**
 * Seção "Bancos" da vitrine. `transact` e `ensureRepo` são os da página
 * Software (painel de transação, polkit, aprovação da chave); `cards`, os
 * cartões dos webapps; `hasWebapps`, se algum webapp de banco está instalado;
 * `repaint` redesenha a vitrine; `busy` diz se há transação em andamento.
 */
export function bankSection({ transact, ensureRepo, repaint, busy, cards, hasWebapps }) {
  async function refresh() {
    try {
      state.module = await call("bank_module_status");
      state.error = null;
    } catch (error) {
      state.error = error.message;
    }
    repaint();
  }

  if (!state.module && !state.error && !state.loading) {
    state.loading = true;
    refresh().finally(() => { state.loading = false; });
  }

  async function act(work) {
    state.acting = true;
    repaint();
    try {
      await work();
    } catch (error) {
      fail(error, t("Não foi possível concluir a operação"));
    } finally {
      state.acting = false;
      await refresh();
    }
  }

  const disabled = busy || state.acting;
  const health = state.module?.health;
  const fix = (label) => button(label, { kind: "primary", size: "sm", icon: "refresh", disabled,
    onClick: () => act(() => installModule({ transact, ensureRepo }, health)) });
  let notice = null;
  if (health === "absent" && hasWebapps) {
    notice = banner("warn", "shield", t("Módulo de segurança dos bancos não instalado"),
      t("Os webapps de bancos ligam o módulo antes de abrir o site. Instale de novo para usá-los."),
      fix(t("Habilitar segurança de bancos")));
  } else if (health === "leftovers") {
    notice = banner("warn", "alert", t("Sobraram arquivos do Warsaw sem o pacote"),
      t("Isso costuma acontecer depois de voltar a um snapshot antigo. Reparar instala o pacote de novo e deixa tudo coerente."),
      fix(t("Reparar")));
  } else if (health === "broken") {
    notice = h("div", {},
      banner("warn", "alert", t("A instalação do Warsaw está incompleta"),
        t("Faltam arquivos ou eles não batem com o pacote. Isso costuma acontecer depois de voltar a um snapshot antigo."),
        fix(t("Reparar"))),
      state.module.problems.length > 0 && h("details", { class: "sw-banks-problems" },
        h("summary", { class: "small" }, t("Ver os arquivos com problema ({count})", { count: state.module.problems.length })),
        h("pre", { class: "small" }, state.module.problems.join("\n"))));
  } else if (health === "stuck") {
    notice = banner("error", "alert-circle", t("O módulo de segurança dos bancos não conseguiu ligar"),
      t("Feche os webapps de bancos e abra de novo. Se continuar, remova o webapp e instale outra vez."),
      button(t("Desligar"), { size: "sm", icon: "stop", disabled,
        onClick: () => act(() => call("services_set_running", { name: UNIT, running: false })) }));
  }

  return section(t("Bancos"), { id: "section-banks" },
    notice && h("div", { class: "sw-banks" }, notice),
    cards);
}
