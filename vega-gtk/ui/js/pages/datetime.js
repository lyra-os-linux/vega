import { call } from "../api.js";
import { N_, t, locale } from "../i18n.js";
import { h, icon, clear, button, toggle, row, list, section, banner, skeletonRows, confirm, toast, normalize } from "../ui.js";

/** Valor atual primeiro quando ele não está na lista do vegad (set_choices do vega-gtk). */
function withCurrent(values, current) {
  return current && !values.includes(current) ? [current, ...values] : values;
}

function select(values, current, label) {
  return h("select", { class: "input dt-select", "aria-label": label }, values.map((value) => h("option", { value, selected: value === current }, value)));
}

/** Fuso horário: lista longa, então um campo com sugestões e validação. */
function timezonePicker(values, current, onChange) {
  const id = `tz-${Math.random().toString(36).slice(2)}`;
  const input = h("input", { class: "input dt-select", list: id, value: current, spellcheck: "false", "aria-label": t("Fuso horário") });
  const options = h("datalist", { id }, values.map((value) => h("option", { value })));
  input.addEventListener("change", () => {
    const typed = normalize(input.value.trim());
    const match = values.find((value) => normalize(value) === typed) ?? values.find((value) => normalize(value).includes(typed));
    input.value = match ?? current;
    onChange(input.value);
  });
  return { el: h("span", {}, input, options), get value() { return input.value; } };
}

export default {
  id: "datetime",
  title: N_("Data, Hora e Idioma"),
  icon: "clock",
  group: "system",
  keywords: ["fuso", "horario", "timezone", "ntp", "relogio", "idioma", "locale", "teclado", "keymap"],
  sections: [
    { id: "timezone", label: N_("Fuso horário"), icon: "clock", keywords: ["timezone", "fuso"] },
    { id: "keyboard", label: N_("Layout do teclado"), icon: "keyboard", keywords: ["teclado", "keymap"] },
  ],

  render(ctx) {
    const body = h("div", {}, skeletonRows(4));
    const status = h("p", { class: "small faint hw-status selectable" }, t("Carregando data, hora e idioma…"));
    const clockTime = h("div", { class: "dt-time" }, "—");
    const clockDate = h("div", { class: "dt-date" });
    const clockZone = h("div", { class: "small muted" });
    let zone = null;

    const tick = () => {
      const now = new Date();
      try {
        clockTime.textContent = now.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit", second: "2-digit", timeZone: zone ?? undefined });
        clockDate.textContent = now.toLocaleDateString(locale, { weekday: "long", day: "numeric", month: "long", year: "numeric", timeZone: zone ?? undefined });
      } catch {
        clockTime.textContent = now.toLocaleTimeString(locale);
      }
      clockZone.textContent = zone ?? "";
    };
    tick();
    const timer = setInterval(tick, 1000);
    ctx.cleanup(() => clearInterval(timer));

    async function load() {
      let data;
      try {
        data = await call("datetime_load");
      } catch (error) {
        status.textContent = error.message;
        clear(body, banner("danger", "alert-circle", t("Data, Hora e Idioma"), error.message));
        return;
      }
      const { status: current, timezones, locales, keymaps } = data;
      zone = current.timezone;
      tick();

      const timezone = timezonePicker(withCurrent(timezones, current.timezone), current.timezone, (value) => { zone = value; tick(); });
      let ntp = current.ntp;
      const localeSelect = select(withCurrent(locales, current.locale), current.locale, t("Idioma e formato regional"));
      const keymapSelect = select(withCurrent(keymaps, current.keymap), current.keymap, t("Layout do teclado"));
      const apply = button(t("Aplicar ao sistema"), { kind: "primary", icon: "check" });

      apply.addEventListener("click", async () => {
        const values = { timezone: timezone.value, ntp, locale: localeSelect.value, keymap: keymapSelect.value };
        const ok = await confirm({
          title: t("Alterar data, hora e idioma?"),
          text: t("Aplicar timezone {timezone}, locale {locale}, teclado {keymap} e NTP {ntp} para todo o sistema?", { ...values, ntp: ntp ? t("ativado") : t("desativado") }),
          ic: "clock",
          confirmLabel: t("Aplicar"),
          required: false,
        });
        if (!ok) return;
        apply.disabled = true;
        status.textContent = t("Aplicando configuração global…");
        try {
          await call("datetime_apply", values);
          toast(t("Configuração atual carregada"));
          await load();
        } catch (error) {
          status.textContent = error.message;
        }
        apply.disabled = false;
      });

      clear(body,
        h("div", { class: "card pad dt-clock" },
          h("span", { class: "dt-clock-ic" }, icon("clock", 26)),
          h("div", { class: "spacer" }, clockTime, clockDate, clockZone)),
        section(t("Configuração global"), { sub: t("As alterações afetam todos os usuários") },
          list(
            h("div", { class: "list-row", id: "sec-timezone" }, h("span", { class: "lead tint" }, icon("globe", 18)), h("div", { class: "body" }, h("div", { class: "title" }, t("Fuso horário"))), h("div", { class: "suffix" }, timezone.el)),
            row({ icon: "refresh", tint: "tint", title: t("Sincronização automática (NTP)"), suffix: toggle(ntp, (on) => (ntp = on), t("Sincronização automática (NTP)")) }),
            row({ icon: "globe", tint: "tint", title: t("Idioma e formato regional"), suffix: localeSelect }),
            h("div", { class: "list-row", id: "sec-keyboard" }, h("span", { class: "lead tint" }, icon("keyboard", 18)), h("div", { class: "body" }, h("div", { class: "title" }, t("Layout do teclado"))), h("div", { class: "suffix" }, keymapSelect))),
          h("div", { class: "row-flex dt-actions" }, apply)),
        status);
      status.textContent = t("Configuração atual carregada");
      if (ctx.sub) document.getElementById(`sec-${ctx.sub}`)?.scrollIntoView({ block: "center" });
    }

    load();
    return {
      el: h("div", {}, h("div", { class: "page-hero" }, h("div", {}, h("h2", {}, t("Data, Hora e Idioma")), h("p", {}, t("Timezone, NTP, locale e teclado")))), body),
      actions: [h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: () => { clear(body, skeletonRows(4)); load(); } }, icon("refresh", 17))],
    };
  },
};
