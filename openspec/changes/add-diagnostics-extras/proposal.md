# add-diagnostics-extras: parte do Vega

Proposta-mãe:
[lyra-ecosystem/openspec/changes/add-diagnostics-extras](https://github.com/lyra-os-linux/lyra-ecosystem/tree/main/openspec/changes/add-diagnostics-extras),
aprovada em 29/10/2026.

- `backup.stale` e `backup.none` com orientação e o atalho **Abrir Backup**.
- **Atualizar** no achado `platform.firmware-updates`: o `fwupd_update.rs` roda
  `fwupdmgr update -y --no-reboot-check` na sessão do usuário (ação
  `update-firmware` do `software_run`, que nunca chega ao vegad), com a tarefa
  e o console de sempre. O polkit do fwupd decide se pede senha. O código 2
  ("nada a atualizar") conta como sucesso, e uma linha pedindo reinício muda a
  mensagem final para "Reinicie para concluir".
