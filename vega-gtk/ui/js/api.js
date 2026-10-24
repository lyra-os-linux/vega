// Ponte com o backend Rust (comandos Tauri e eventos).

const tauri = window.__TAURI__;

/** Chama um comando; erros chegam como texto já traduzido. */
export async function call(cmd, args = {}) {
  try {
    return await tauri.core.invoke(cmd, args);
  } catch (error) {
    throw new Error(typeof error === "string" ? error : error?.message ?? String(error));
  }
}

/** Escuta um evento do backend; devolve a função que cancela a escuta. */
export function on(event, handler) {
  const pending = tauri.event.listen(event, (e) => handler(e.payload));
  return () => pending.then((unlisten) => unlisten());
}

/** Domínios com entrada única (`op` + `args`). */
export const personalization = (op, args = {}) => call("personalization_call", { op, args });
export const assistant = (op, args = {}) => call("assistant_call", { op, args });
export const vms = (op, args = {}) => call("vms_call", { op, args });

/**
 * Seletor de arquivo/pasta nativo. `filters`: [{ name, extensions }].
 * Devolve o caminho escolhido (ou lista, com `multiple`) ou null.
 */
export async function pick({ directory = false, multiple = false, filters, title } = {}) {
  return tauri.core.invoke("plugin:dialog|open", { options: { directory, multiple, filters, title } });
}

export const win = tauri.window.getCurrentWindow();
