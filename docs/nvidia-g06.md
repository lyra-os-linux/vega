# NVIDIA no Vega

A instalação genérica de drivers, a troca de drivers (`Hardware.SwitchNvidiaDriver`)
e a instalação de firmware não livre (`Software.InstallNonFreeFirmware`) foram
removidas. Esses dois métodos D-Bus v1 permanecem apenas para compatibilidade e
retornam `org.freedesktop.DBus.Error.NotSupported` antes de autenticação,
execução de comandos ou criação de transação. As ações Polkit correspondentes
foram removidas. A pilha G06 não é mais instalada por nenhum fluxo do Vega.

A instalação **opcional e qualificada** do driver NVIDIA oficial (G07, módulo
assinado pela SUSE) voltou com outra forma: o `vegad` anuncia a capability
`nvidia-official-v1` em `Metadata.Capabilities` e reativa
`Software.InstallNvidia(confirmed)` para esse fluxo restrito. O cartão NVIDIA da
página Hardware do `vega-gtk` (`vega-gtk/src/nvidia.rs`) só mostra o botão de
instalação quando a capability está presente e `Software.NvidiaStatus` responde
`available` ou `unmanaged`; nos outros estados exibe o diagnóstico traduzido.
A revisão de repositórios, confiança e recuperação é obrigatória mesmo com as
confirmações opcionais desativadas nas preferências.

Estados, pacotes qualificados, política de kernel, recuperação e limites do
fluxo estão documentados em `docs/nvidia.md` do repositório
[vegad](https://github.com/lyra-os-linux/vegad). Em particular, uma instalação
nova exige que o kernel em uso seja o kernel padrão de boot (`-default`) e que o
Leap publique o KMP assinado para esse kernel; caso contrário o estado é
`kernel-missing` e o botão fica oculto.

A proteção de suspensão para NVIDIA 580.159.03 em notebooks híbridos continua
sendo reconciliada na inicialização do daemon e independe desse fluxo.

Frontend e daemon devem ser atualizados de forma coordenada: um frontend novo
com daemon antigo mostra "Atualize o vegad para usar a integração NVIDIA atual";
um frontend antigo com daemon novo continua recebendo a recusa explícita dos
métodos descontinuados.
