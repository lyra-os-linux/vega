# Ciclo instalado de VM — 21/09/2026

A qualificação usou libvirt/QEMU/KVM dentro de uma VM descartável marcada,
com conta não privilegiada `installeduser`, conexão `qemu:///session`, Xvfb e
Openbox. Nenhuma VM pessoal nem pacote da estação foi alterado.

O Vega GTK 5.1.43 público criou a VM pela interface (BIOS, 2 CPUs, 768 MiB,
4 GiB) usando a ISO oficial Alpine virt 3.24.2. A instalação real em disco
foi conduzida pelo console Lyra VMs e `setup-disk`, com pacotes assinados do
repositório Alpine. O instalador automatizado recusava outro UUID/capacidade.
A mídia mínima não incluía syslinux; a primeira tentativa terminou antes do
particionamento. A segunda, com o repositório oficial, concluiu normalmente.

Após desligamento normal, o botão Ejetar ISO do Vega retirou a mídia do leitor.
O próximo boot montou `/dev/vda2` como raiz ext4 e preservou o marcador de dados.
Renomear manteve UUID e caminhos; ampliar alterou o disco de 4 para 5 GiB.
CPU/RAM foram aplicadas pelo teclado no editor corrigido: o segundo boot pelo
disco informou 1 CPU, cerca de 1 GiB de RAM e 10.485.760 setores de 512 bytes,
com o mesmo hash do marcador. A partição não foi expandida automaticamente.

A remoção padrão pela UI deixou disco e ISO local idênticos byte a byte.
A definição descartável foi então restaurada para testar a opção explícita:
ambos os arquivos listados foram apagados, inclusive a ISO já ejetada, e a ISO
original manteve seu checksum. Nenhuma definição de VM restou no ensaio.

## Correção encontrada e validada

O editor em 5.1.43 usava AlertDialog com uma área rolável estreita: nome e
controles de disco eram cortados. Em 5.1.44 o editor usa Dialog com cabeçalho,
rolagem vertical e ações fixas. A confirmação de remoção também passa a quebrar
o texto da opção e os caminhos longos, sem rolagem horizontal.

Capturas verificadas em 1600×1000 e 1366×768 a 100%, e 2560×1600 a 200%.
A navegação por Tab alcançou nome, renomear, CPU, RAM, capacidade, ampliar,
fechar e aplicar; digitação e aplicação pelo teclado foram exercitadas.
O teste GTK de regressão verifica largura útil do editor e que o botão Ampliar
permanece dentro dele, além dos controles e limite mínimo já existentes.

A automação AT-SPI precisou selecionar controles por papel e aguardar foco;
APIs de foco/valor não funcionaram para todos os elementos nesse ambiente.
O teste funcional usou também eventos reais de teclado/mouse. Isso não equivale
a uma qualificação com leitor de tela nem a toda combinação de resolução/escala.

## Limites e artefatos

A criação, instalação, ejeção, renomeação e crescimento iniciaram no RPM público
5.1.43. As edições finais por teclado e a remoção foram repetidas na compilação
com a correção de layout. A publicação de 5.1.44 deve verificar o RPM final.
A mudança é somente de UI GTK; não altera o backend de armazenamento.

Os scripts, logs, definições e capturas estão em
`analysis/2026-09-21/vm-full-cycle/` no workspace de qualificação. O resumo
portável está em [evidência do ciclo](evidence/vm-full-cycle-20260921.json).

Este ensaio qualifica um convidado Alpine em BIOS, não todas as distribuições,
nem instalação UEFI de convidado, leitor de tela ou a ISO Alpha 8. A auditoria
continua anterior à construção e aos testes do checksum exato da candidata.

Origem: https://dl-cdn.alpinelinux.org/alpine/v3.24/releases/x86_64/
Instalação: https://wiki.alpinelinux.org/wiki/System_Disk_Mode
