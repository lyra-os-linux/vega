# Driver NVIDIA G06 pelo Vega

O módulo **Software** do Vega oferece uma instalação opcional e pós-instalação
para GPUs NVIDIA Turing ou mais recentes no Lyra OS/openSUSE Leap 16.0. O
driver não faz parte da ISO padrão e nunca é instalado sem confirmação.

## Fluxo seguro

Antes de habilitar a ação, o `vegad` confirma uma GPU de vídeo NVIDIA G06,
consulta o Secure Boot e rejeita instalações parciais ou versões
desalinhadas. A transação privilegiada então:

1. repete o preflight no serviço privilegiado;
2. cria um snapshot Snapper somente leitura e registra seu número;
3. valida ou adiciona `repo-nvidia`, apontando exclusivamente para o
   repositório NVIDIA oficial do Leap 16.0;
4. instala `nvidia-open-driver-G06-signed-kmp-meta` e
   `nvidia-userspace-meta-G06` na mesma chamada do Zypper;
5. confirma que os dois metapacotes têm exatamente a mesma versão;
6. audita a versão upstream de todos os RPMs G06 efetivamente instalados;
7. aplica a política de qualificação de energia para a combinação de driver e
   topologia gráfica detectada;
8. regenera o initramfs e orienta a reinicialização.

O módulo aberto assinado é obrigatório com Secure Boot ativo. O fluxo não
oferece pacotes legados a GPUs anteriores a Turing e não tenta corrigir
automaticamente uma instalação parcial, pois remover pacotes gráficos sem
revisão pode inutilizar a sessão gráfica.

## Verificação após reiniciar

Abra **Vega → Software** e selecione **Verificar driver**. A verificação exige
uma resposta válida do `nvidia-smi`, vínculo do driver e pelo menos um
conector publicado em `/sys/class/drm`.

O `vegad` também reconcilia a política de energia ao iniciar. Combinações com
regressão conhecida permanecem com o driver gráfico funcional, mas recebem o
estado `quarantined` e um drop-in gerenciado em
`/etc/systemd/sleep.conf.d/90-lyra-nvidia-quarantine.conf`. Esse arquivo
bloqueia suspensão e hibernação para evitar travamento, desligamento forçado e
recuperação do filesystem. Quando uma versão qualificada substitui o driver,
o Vega remove automaticamente somente o drop-in que contém seu marcador de
propriedade; arquivos administrativos nunca são sobrescritos ou removidos.

A combinação inicial em quarentena é NVIDIA `580.159.03` em notebooks
híbridos Intel/AMD + NVIDIA. Ela reproduziu falhas no gerenciamento de VRAM do
módulo durante a entrada em S3 e s2idle. Desktops com a mesma versão não são
bloqueados por essa regra.

O resultado "Driver NVIDIA ativo" só é saudável quando os metapacotes e todos
os componentes G06 possuem a mesma versão upstream. Misturas entre módulo,
userspace ou firmware são tratadas como instalação desalinhada, mesmo quando
`nvidia-smi` ainda responde.

## Recuperação

O número do snapshot aparece no progresso e no resultado da transação. Se o
driver impedir o boot gráfico, escolha no GRUB o snapshot “antes do driver
NVIDIA G06”. Depois de iniciar o snapshot e confirmar que o sistema está
funcional, execute o rollback pelo módulo **Snapshots** do Vega. Não apague o
snapshot antes da validação pós-reinício.

Uma falha anterior à criação do snapshot não altera pacotes. Uma falha após
o snapshot sempre inclui seu número no diagnóstico e não tenta ocultar nem
ignorar erros do Zypper ou do `dracut`.

## Firmware não livre de outros dispositivos

A mesma aba **Hardware** oferece separadamente firmware do `repo-non-oss`.
Essa função usa uma lista revisada de IDs PCI/USB e associações inequívocas
com pacotes; não instala firmware apenas por fabricante ou por busca textual.
Na implementação inicial são reconhecidas placas Hauppauge/Conexant IVTV e
dispositivos bladeRF. O botão só é habilitado se o hardware estiver presente,
o pacote correspondente estiver disponível no `repo-non-oss` e ainda não
estiver instalado. A transação exige confirmação, cria snapshot somente
leitura, fixa a origem no `repo-non-oss` e regenera o initramfs.
