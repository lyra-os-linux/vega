# add-help-report: parte do Vega

Proposta-mãe: [lyra-ecosystem/openspec/changes/add-help-report](https://github.com/lyra-os-linux/lyra-ecosystem/tree/main/openspec/changes/add-help-report), aprovada em 29/10/2026 (destino: Discussions do `lyraos-desktop`, ligadas em 30/10/2026, categoria Q&A).

- Botão **Pedir ajuda** no relatório do diagnóstico, que busca o relatório **redigido** e abre o painel com a descrição, a prévia editável e Copiar, Abrir no fórum (Discussions) e Abrir uma issue.
- `ui/js/pages/help-text.js`: o texto, sem dependência da interface, testado com node em `tests/test_help_text.py` (40 achados com 40.000 caracteres brutos cabem em 6.000, nenhum achado sai).
- Se a URL passar de ~7,8 KB (acentos e Markdown crescem na codificação), a página recebe o texto encurtado e o completo vai para a área de transferência.
