# Paleta nativa Lyra — Vega 5.1.46

As janelas do Vega usam a paleta clara/escura do lyra-os-theme 1.10.0,
registrada em `vega-gtk/resources/lyra-palette.json`. Os arquivos
`palette-light.css` e `palette-dark.css` devem acompanhar esse contrato.

O StyleManager do libadwaita seleciona e recarrega a paleta durante a
alternância de modo. Em alto contraste, o Vega usa as cores semânticas do
sistema. A personalização global GTK do usuário não é necessária.

Validação nativa em Leap 16.1, GTK 4.18.6 / libadwaita 1.7.5:
Vega real em Mutter, D-Bus e portais isolados, sem acesso ao vegad do host;
mesmo PID durante escuro → claro → escuro → alto contraste claro → alto
contraste escuro → claro. Capturas e pixels do fundo verificam a troca real.
O backend indisponível nas capturas é intencional nesse ensaio visual.

Reversão: voltar ao RPM 5.1.45; nenhuma preferência GTK global é gravada
por esta alteração. Não qualifica uma ISO candidata nem todos os fluxos do Vega.
