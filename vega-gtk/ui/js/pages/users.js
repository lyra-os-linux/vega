import { call, pick } from "../api.js";
import { N_, t } from "../i18n.js";
import { h, icon, clear, badge, button, toggle, row, list, empty, skeletonRows, confirm, sheet, fail, toast, hueFor } from "../ui.js";

/** Mesma regra do vega-gtk (e do vegad) para contas comuns. */
export function validUsername(value) {
  return /^[a-z_][a-z0-9_-]*$/.test(value.trim());
}

function avatar(name, size = 56) {
  const initials = name.trim().split(/\s+/).map((part) => part[0]).slice(0, 2).join("").toUpperCase() || "?";
  const el = h("span", { class: "user-avatar", "aria-hidden": "true" }, initials);
  el.style.width = el.style.height = `${size}px`;
  el.style.fontSize = `${Math.round(size * 0.36)}px`;
  el.style.background = `linear-gradient(150deg, ${hueFor(name)}, ${hueFor(`${name}!`)})`;
  return el;
}

export default {
  id: "users",
  title: N_("Usuários"),
  icon: "users",
  group: "system",
  keywords: ["conta", "senha", "administrador", "wheel", "grupo", "login"],

  render(ctx) {
    let users = null;
    let busy = false;
    const status = h("span", { class: "small faint selectable" }, t("Carregando usuários…"));
    const body = h("div", {}, skeletonRows(3));

    async function load() {
      status.textContent = t("Carregando usuários…");
      try {
        users = await call("users_list");
        if (!ctx.visible()) return;
        status.textContent = t("{count} usuário(s)", { count: users.length });
      } catch (error) {
        users = users ?? [];
        status.textContent = error.message;
      }
      paint();
    }

    async function run(label, fn) {
      busy = true;
      status.textContent = label;
      paint();
      try {
        await fn();
        busy = false;
        await load();
        return true;
      } catch (error) {
        busy = false;
        status.textContent = error.message;
        fail(error);
        paint();
        return false;
      }
    }

    async function toggleAdmin(user) {
      const ok = await confirm(user.is_admin
        ? { title: t("Remover privilégios administrativos?"), text: t("{user} deixará de administrar o sistema.", { user: user.username }), ic: "key", tone: "warn", confirmLabel: t("Remover admin"), required: false }
        : { title: t("Conceder privilégios administrativos?"), text: t("{user} poderá administrar o sistema.", { user: user.username }), ic: "key", confirmLabel: t("Tornar admin"), required: false });
      if (ok) run(t("Processando {user}…", { user: user.username }), () => call("users_set_admin", { username: user.username, admin: !user.is_admin }));
    }

    async function remove(user) {
      const ok = await confirm({
        title: t("Remover usuário?"),
        text: t("A conta {user} e seu diretório pessoal serão removidos.", { user: user.username }),
        ic: "trash",
        tone: "danger",
        confirmLabel: t("Remover"),
        confirmKind: "danger",
        required: false,
      });
      if (ok) run(t("Processando {user}…", { user: user.username }), () => call("users_remove", { username: user.username }));
    }

    // Formulário de criação/edição (o mesmo diálogo do vega-gtk).
    function editor(user) {
      const editing = Boolean(user);
      const input = (placeholder, attrs = {}) => {
        const el = h("input", { class: "input", placeholder, "aria-label": placeholder, ...attrs });
        el.style.width = "100%";
        return el;
      };
      const fullName = input(t("Nome completo"), { value: user?.full_name ?? "" });
      const username = input(t("nome de usuário"), { value: user?.username ?? "", class: "input mono", disabled: editing, autocapitalize: "off", spellcheck: "false" });
      const password = input(t("Senha (mínimo de 8 caracteres)"), { type: "password", autocomplete: "new-password" });
      const passwordConfirm = input(t("Confirmar senha"), { type: "password", autocomplete: "new-password" });
      const groups = input(t("Grupos adicionais, separados por vírgula"), { value: user?.groups.join(", ") ?? "" });
      let admin = user ? user.is_admin : true;
      let photoPath = "";
      const photoLabel = h("span", { class: "spacer" }, editing ? t("Manter foto atual (ou selecionar outra)…") : t("Selecionar foto…"));
      const photoButton = h("button", { class: "btn", onclick: async () => {
        const path = await pick({ title: t("Selecionar foto"), filters: [{ name: t("Imagens PNG ou JPEG"), extensions: ["png", "jpg", "jpeg", "PNG", "JPG", "JPEG"] }] }).catch(fail);
        if (typeof path === "string" && path) {
          photoPath = path;
          photoLabel.textContent = path.split("/").pop() || t("Foto selecionada");
        }
      } }, icon("camera", 16), photoLabel);
      photoButton.style.width = "100%";
      photoButton.style.justifyContent = "flex-start";

      const submit = button(editing ? t("Salvar alterações") : t("Criar usuário"), { kind: "primary" });
      const valid = () => {
        const keepPassword = editing && password.value === "";
        const passwordOk = keepPassword || ([...password.value].length >= 8 && password.value === passwordConfirm.value);
        return validUsername(username.value) && fullName.value.trim() !== "" && passwordOk;
      };
      const mark = (el, bad) => el.classList.toggle("invalid", bad);
      const refreshValidity = () => {
        mark(username, username.value !== "" && !validUsername(username.value));
        mark(password, password.value !== "" && [...password.value].length < 8);
        mark(passwordConfirm, passwordConfirm.value !== "" && password.value !== passwordConfirm.value);
        submit.disabled = busy || !valid();
      };
      for (const el of [fullName, username, password, passwordConfirm]) el.addEventListener("input", refreshValidity);
      // Sugere o login a partir do nome, como o GNOME, até a pessoa editar.
      let typedLogin = editing;
      username.addEventListener("input", () => (typedLogin = true));
      fullName.addEventListener("input", () => {
        if (typedLogin) return;
        username.value = fullName.value.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase().split(/\s+/)[0]?.replace(/[^a-z0-9_-]/g, "") ?? "";
        refreshValidity();
      });
      refreshValidity();

      const close = sheet(editing ? t("Editar usuário") : t("Novo usuário"), [
        h("div", { class: "user-editor-head" }, avatar(user?.full_name || user?.username || "?", 64), h("div", { class: "user-editor-fields" }, fullName, username)),
        h("div", { class: "user-editor-grid" }, password, passwordConfirm),
        groups,
        h("p", { class: "small faint" }, t("Informe grupos existentes, por exemplo: audio, video, docker. O grupo wheel é incluído automaticamente para administradores.")),
        photoButton,
        list(row({ icon: "key", title: t("Administrador"), suffix: toggle(admin, (on) => (admin = on), t("Administrador")) })),
        h("div", { class: "row-flex", style: { "justify-content": "flex-end" } }, button(t("Cancelar"), { onClick: () => close() }), submit),
      ], { wide: false });

      submit.addEventListener("click", async () => {
        if (!valid()) return;
        const login = username.value.trim();
        const role = admin ? t("administrador") : t("usuário comum");
        const ok = await confirm({
          title: editing ? t("Salvar alterações?") : t("Criar usuário?"),
          text: editing ? t("Os dados da conta {username} serão atualizados.", { username: login }) : t("A conta {username} será criada como {role}.", { username: login, role }),
          ic: "user",
          confirmLabel: editing ? t("Salvar alterações") : t("Criar"),
          required: false,
        });
        if (!ok) return;
        const form = {
          username: login,
          full_name: fullName.value.trim(),
          password: password.value,
          groups: groups.value.split(",").map((g) => g.trim()).filter(Boolean),
          photo_path: photoPath,
          is_admin: admin,
        };
        submit.disabled = true;
        const done = await run(t("Criando {username}…", { username: login }), () => call("users_save", { form, create: !editing }));
        if (done) {
          close();
          toast(editing ? t("Salvar alterações") : t("Criar usuário"));
        } else {
          refreshValidity();
        }
      });
      requestAnimationFrame(() => fullName.focus());
    }

    function card(user) {
      const mutable = user.username !== "root" && !busy;
      return h("div", { class: "card user-card" },
        avatar(user.full_name || user.username),
        h("div", { class: "user-card-name" },
          h("strong", {}, user.full_name || user.username),
          h("div", { class: "small muted mono" }, user.username)),
        h("div", { class: "row-flex user-card-badges" },
          badge(user.is_admin ? t("Administrador") : t("Usuário comum"), user.is_admin ? "accent" : "outline", user.is_admin ? "key" : null)),
        user.groups.length > 0 && h("div", { class: "small faint user-card-groups", title: user.groups.join(", ") }, user.groups.join(", ")),
        h("div", { class: "user-card-actions" },
          button(t("Editar"), { size: "sm", disabled: !mutable, onClick: () => editor(user) }),
          button(user.is_admin ? t("Remover admin") : t("Tornar admin"), { size: "sm", disabled: !mutable, onClick: () => toggleAdmin(user) }),
          h("button", { class: "icon-btn sm danger-icon", "aria-label": t("Remover"), title: t("Remover"), disabled: !mutable, onclick: () => remove(user) }, icon("trash", 15))));
    }

    function paint() {
      if (users == null) return;
      clear(body, users.length === 0
        ? empty("users", t("Nenhum usuário listado"), t("Ainda não há contas cadastradas."))
        : h("div", { class: "grid auto users-grid" }, users.map(card)));
    }

    load();
    return {
      el: h("div", {},
        h("div", { class: "page-hero" },
          h("div", {}, h("h2", {}, t("Contas e Usuários")), h("p", {}, t("Criação, remoção e controle de administração"))),
          h("div", { class: "row-flex" }, status, button(t("Novo usuário"), { kind: "primary", icon: "plus", onClick: () => editor(null) }))),
        body),
      actions: [h("button", { class: "icon-btn", "aria-label": t("Atualizar"), title: t("Atualizar"), onclick: () => load() }, icon("refresh", 17))],
    };
  },
};
