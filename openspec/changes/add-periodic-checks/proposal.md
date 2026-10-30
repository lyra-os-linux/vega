# add-periodic-checks: parte do Vega

Proposta-mãe: [lyra-ecosystem/openspec/changes/add-periodic-checks](https://github.com/lyra-os-linux/lyra-ecosystem/tree/main/openspec/changes/add-periodic-checks), aprovada em 29/10/2026 (frequência semanal).

- O `vega-update-notifier` escuta o `Novelties` e, ao entrar na sessão, chama o `LastPeriodic`: uma notificação por execução com novidades ("O Vega encontrou N novidade(s) na saúde do sistema"), que abre o Vega com `--open-health` (Hardware → Diagnóstico, com o relatório da verificação). Respeita a preferência nova `notify_health`.
- Preferências → **Saúde do sistema**: ligar e desligar a verificação e escolher semanal ou diária, para o sistema todo (pede senha).
- A lista do Diagnóstico usa o resultado da verificação periódica quando ele é mais novo e mostra quando ela rodou e quantas novidades achou.
