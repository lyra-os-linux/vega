# Edição de máquinas — Vega GTK 5.1.43

A ação **Editar máquina** permite renomear, aplicar CPU/RAM, aumentar a capacidade
de discos e ejetar a ISO. Cada botão aplica apenas sua operação, para não misturar
uma ampliação irreversível com alterações de configuração. As operações exigem
máquina persistente desligada, sem estado salvo; o backend revalida esse estado.
O UUID e os caminhos de armazenamento permanecem estáveis ao renomear.

A ampliação admite volumes de arquivo RAW/QCOW2 independentes, sem snapshots,
criptografia ou compartilhamento. Bases de clones registrados nos pools ativos
também são preservadas, mesmo sem uma VM associada ao clone. A capacidade deve aumentar e não exceder
2048 GiB; a chamada libvirt não habilita a opção de redução. O convidado pode
precisar expandir sua partição/sistema de arquivos depois. Ejetar a ISO altera
somente a fonte do leitor virtual, preservando a mídia no disco.

A remoção preserva arquivos por padrão. A opção explícita de apagar apresenta
um plano de caminhos que é conferido novamente antes de executar: volumes
pessoais do pool Lyra com nome vinculado ao UUID (incluindo ISO local já ejetada),
NVRAM exclusiva existente e atalho próprio. Discos externos/compartilhados,
arquivos com links e diretórios não são apagados. Na conexão do sistema, não se
infere propriedade de discos externos ao pool pessoal. Falhas de exclusão após
remover a definição são reportadas com os arquivos preservados; não há promessa
de transação ou rollback de arquivos apagados.

Testes: `edit_probe` exige a marca `lyra.virtualization-test=1` da VM descartável.
Verifica ampliação com conteúdo conhecido, recusa de redução e uso compartilhado,
ejetar sem apagar, mudança de recursos/UUID, remoção padrão, plano obsoleto e
exclusão explícita de disco/ISO/NVRAM/atalho. A interface nativa usa dados fictícios.
Esses ensaios não qualificam a ISO Alpha 8 nem executam alterações nas VMs pessoais.

API de armazenamento: https://libvirt.org/html/libvirt-libvirt-storage.html#virStorageVolResize
