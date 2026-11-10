# Abrir o Vega numa página (deep link)

O Welcome, a documentação e as notificações podem abrir o Vega direto numa
página, aba e seção (vega#209). Isso funciona com o Vega fechado ou já aberto:
a segunda execução repassa o destino à janela que já existe.

## Linha de comando

```bash
vega-gtk --page software
vega-gtk --page software --tab developers
vega-gtk --page software --tab developers --section databases
```

`--section` só vale junto com `--tab`.

## Link `vega://`

O `vega.desktop` registra o esquema `x-scheme-handler/vega`, para links em
páginas e notificações:

```text
vega://software/developers/databases
```

O link só navega: ele não instala nada, não muda configuração e não pede senha.

## Destinos

- **Página:** o `id` da página na barra lateral (`software`, `network`,
  `hardware`…). Página desconhecida abre a página inicial escolhida nas
  Preferências.
- **Aba:** o identificador que a página aceita. No Software: `popular`,
  `developers`, `network`, `windows`, `search`, `installed`, `updates` e
  `repos`. A página ignora uma aba que não conhece.
- **Seção:** o `id` da categoria no catálogo da aba, como `databases` em
  `vega-gtk/ui/js/pages/software/developer-catalog.js`. O Vega rola até ela quando o catálogo
  termina de carregar; uma seção que não existe é ignorada.

Cada trecho aceita só letras minúsculas, dígitos e hífen, com até 48
caracteres e no máximo três trechos. Um destino fora disso é descartado, e o
Vega abre normalmente.

Os atalhos antigos continuam valendo: `--virtual-machines`, `--open-updates` e
`--open-health`.
