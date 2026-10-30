# add-network-troubleshooter: parte do Vega

Proposta-mãe:
[lyra-ecosystem/openspec/changes/add-network-troubleshooter](https://github.com/lyra-os-linux/lyra-ecosystem/tree/main/openspec/changes/add-network-troubleshooter),
aprovada em 29/10/2026.

- Botão **Minha internet não funciona** no topo da página Rede
  (`ui/js/pages/network-troubleshoot.js`), com o painel passo a passo: cabo ou
  Wi-Fi, endereço, roteador, DNS e internet, cada um com o status, e o achado
  principal com a orientação e o conserto (via `RepairFindings`).
- **Abrir página de login** para portal cativo: o comando `open_portal` só
  aceita `http://host/`, porque o portal intercepta HTTP sem TLS.
- O teste `network` também aparece no Diagnóstico (rápido e completo), com os
  consertos no Resolver.
