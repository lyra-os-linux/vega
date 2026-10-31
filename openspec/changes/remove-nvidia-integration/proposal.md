# remove-nvidia-integration: parte do Vega

Proposta-mãe, requisitos e design:
[lyra-ecosystem/openspec/changes/remove-nvidia-integration](https://github.com/lyra-os-linux/lyra-ecosystem/tree/main/openspec/changes/remove-nvidia-integration),
aprovada pelo mantenedor em 31/10/2026.

Este repositório entrega:

- o `vega-gtk` sem o comando `software_nvidia_status`, sem a ação
  `install-nvidia` e sem o cartão e a seção NVIDIA da página Hardware; com um
  `vegad` antigo, que ainda anuncia `nvidia-official-v1`, nada NVIDIA aparece;
- o achado `gpu.bench-failed` do diagnóstico sem o atalho "Ver NVIDIA";
- o CSS, os textos dos catálogos e a documentação que só existiam para isso
  (`docs/nvidia-g06.md` sai; o README aponta a
  [documentação do Lyra OS](https://lyraos.com.br/docs/));
- `lyra-vega-dbus` 5.2.0, que não tem mais os métodos NVIDIA.

Continuam: o teste de desempenho gráfico (`src/backend/gpu_bench.rs`, com os
achados de RTD3 e das cenas do glmark2), a detecção da GPU no inventário e
`docs/webkit-dmabuf.md`, que não instalam nada.
