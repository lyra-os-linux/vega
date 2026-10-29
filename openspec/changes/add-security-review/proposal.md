# add-security-review: parte do Vega

Proposta-mãe, requisitos e design:
[lyra-ecosystem/openspec/changes/add-security-review](https://github.com/lyra-os-linux/lyra-ecosystem/tree/main/openspec/changes/add-security-review),
validada pelo mantenedor em 29/10/2026.

O vega-gtk entrega:

- **Portas abertas**, na página Rede e Firewall: o programa, a porta, os selos
  de alcance (Só esta máquina, Rede local virtual, Exposta na rede, Bloqueada
  pelo firewall) e **Fechar** para cada exposição. Fechar remove a regra de
  porta e/ou para o serviço, sempre com confirmação.
- O teste **Segurança** no diagnóstico, com os remédios novos no Resolver e
  as orientações e atalhos dos achados sem conserto automático.
