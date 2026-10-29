# add-space-cleanup: parte do Vega

Proposta-mãe, requisitos e design:
[lyra-ecosystem/openspec/changes/add-space-cleanup](https://github.com/lyra-os-linux/lyra-ecosystem/tree/main/openspec/changes/add-space-cleanup),
validada pelo mantenedor em 29/10/2026.

Este repositório entrega:

- a seção **Liberar espaço** em Armazenamento (`ui/js/pages/storage-cleanup.js`),
  com as categorias do sistema (pelo `Cleanup` do vegad, quando a capability
  `space-cleanup-v1` existe) e as do usuário;
- as categorias do usuário no backend (`src/backend/cleanup.rs`): lixeira,
  miniaturas, `~/.cache` por aplicativo e runtimes Flatpak do usuário, limpos na
  sessão, sem senha e sem seguir links simbólicos (`remove_dir_all` do Rust não
  atravessa links; uma raiz de categoria que seja link é recusada);
- o atalho "Abrir Liberar espaço" para o achado `storage.low-space`.

"Limpar tudo o que for seguro" cobre `coredumps`, `journal`, `package-cache`,
`flatpak-unused`, `trash`, `thumbnails` e `user-flatpak-unused`. `old-kernels`,
`snapshots` e `user-cache` só saem por escolha explícita.
