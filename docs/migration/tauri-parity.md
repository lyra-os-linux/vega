# Migração do vega-gtk para Tauri: lista de paridade

A interface do Vega passa de GTK4/libadwaita para Tauri 2 (WebKitGTK) na 16.1,
por exceção ao congelamento aprovada pelo mantenedor em 24/10/2026. Requisito:
**todas** as funções do vega-gtk continuam existindo. O pacote, o binário
`vega-gtk`, o `vega-update-notifier`, o app id `org.lyraos.Vega` e o contrato
`vega-gtk --desktop-profile get|set <id>` (usado pelo lyra-welcome) não mudam.

Marque cada item ao portar. A referência é o vega-gtk antes da migração
(commit aec835d).

## Global
- [ ] App id `org.lyraos.Vega`, instância única; segunda ativação reapresenta a janela.
- [ ] `--virtual-machines` abre Máquinas virtuais; `VEGA_START_PAGE=software` abre Atualizações; ação `app.open-updates`.
- [ ] `--desktop-profile get|set` antes de qualquer janela (códigos de saída 2 uso, 1 falha/readback diferente; `gio::Settings::sync()` após set).
- [ ] `VEGA_BENCHMARK_MARKER` → `VEGA_WINDOW_READY_MS=` no stderr.
- [ ] Uma conexão `VegaDbus`; falha marca o painel como indisponível e tenta de novo no próximo refresh.
- [ ] Refresh do painel a cada `refresh_interval_minutes` (1–60), coalescido (1 em voo + 1 pendente).
- [ ] Navegação: dashboard, software, backup (Backups | Pontos de restauração), virtualization, assistant, hardware (Hardware | Kernel), datetime, screen (Personalização), monitor, storage, network, desktop (Bluetooth), services, users, logs. Busca filtra a navegação.
- [ ] Página inicial configurável (qualquer id).
- [ ] Menu: Configurações; Sobre (versão `VEGA_VERSION`, site, issues, GPL-3.0, "Rodrigo Brito").
- [ ] Política de confirmação: `required` sempre; `confirm` pulado se `confirm_actions=false`, exceto quando há formulário/revisão; ações de pacote, NVIDIA, rollback, chave de repositório e preparação sempre perguntam.
- [ ] i18n gettext domínio `vega-gtk` (pt-BR, en-US, es-ES), msgids em português, resolução de locale de `i18n.rs`, idioma da preferência.

## Preferências (`$XDG_CONFIG_HOME/vega-gtk/preferences.json`)
- [ ] language, start_page, confirm_actions, refresh_interval_minutes, notify_updates, notify_service_failures, notify_backups, redact_ai_data, save_ai_history (desligar apaga o histórico).

## Painel
- [ ] Backend (`system().status()`), Sistema, Atualizações (`list_updates`, reage a `UpdatesAvailable`), Backup (`list_configs`), Pontos de restauração (`available` + `list`; corrigir o link para backup/snapshots), Serviços (`services().list()`, "habilitado mas parado"; notificação "Falha em serviços"), Disco (`system().disk_usage()`).
- [ ] Inventário de hardware e firmware carregados junto.

## Software
- [ ] Painel de preparação: `preparation().status()` a cada 5 s com a página visível; estados/fases/erros; "Tentar novamente" (`retry`); "Revisar chave…" → `approve_key` + transação.
- [ ] Abas Buscar / Instalados / Atualizações / Repositórios; visão lista/cartões.
- [ ] Buscar (≥2 caracteres, `search`), agrupamento por nome, origem oficial < flathub < outra, fila de instalação (sequencial, Flathub primeiro, resumo de falhas).
- [ ] Detalhes (`package_details`) com instalar/remover.
- [ ] Instalados (`list_installed`).
- [ ] Atualizações (`list_updates`) agrupadas por repositório; Atualizar uma (`update_package`); Atualizar tudo (`update_all`); Limpar cache (`clear_cache`).
- [ ] Repositórios (`list_repos`, `set_repo_enabled`, `add_repo` + `KeyPending` → `trust_repo_key`, inclusive sem key_id).
- [ ] Painel de transação: progresso, progresso por pacote, console (`ConsoleLine`, limite 10 000 linhas/4 MiB), falha com alerta; recarregar a aba e o contador do painel ao terminar.

## Backup (Restic)
- [ ] `list_configs`, `list_snapshots`, `list_snapshot_paths` (100 por vez), nova configuração (id, caminhos, destino, UUID, frequência manual/daily/weekly), excluir, executar agora (eventos + notificação "Backup concluído"), restaurar itens (pasta separada | substituir), alertas.

## Pontos de restauração (Snapper)
- [ ] `available`, `list` (sem #0), comparar (`diff_packages`), criar, excluir, limpar todos com progresso, rollback com revisão obrigatória, retenção 1–100.

## Hardware e Kernel
- [ ] Processador, vídeo, memória, firmware.
- [ ] NVIDIA: capacidade `nvidia-official-v1`, `nvidia_status`, mensagens por estado, Secure Boot, snapshot de recuperação, "Atualizar diagnóstico", instalar/ativar integração com diálogo obrigatório (`install_nvidia(true)`).
- [ ] Kernel: instalados, disponíveis não instalados, bootloader (entrada padrão, timeout 0–120, cmdline) com `apply_boot_config`.

## Armazenamento
- [ ] `storage().list()`, montar/desmontar.

## Rede
- [ ] Interfaces (IPv4 estático validado via `set_static_ipv4`), Wi-Fi (conectar com senha se protegida, desconectar), Proxy (`set_proxy`, remover), VPN (importar .ovpn), Firewall (status, serviços, portas tcp/udp).

## Bluetooth
- [ ] Status/indisponível, ligar, buscar, parear/conectar/desconectar, enviar arquivo, receber arquivos.

## Serviços
- [ ] Principais (`list`) | Todos (`list_all`); habilitar, iniciar/parar, reiniciar.

## Usuários
- [ ] Listar; root imutável; criar/editar (nome, login validado, senha ≥8 com confirmação, grupos, foto PNG/JPEG ≤5 MB, admin padrão ligado); tornar/remover admin; remover.

## Logs
- [ ] Carga preguiçosa (sem polkit na abertura); unidade, prioridade, período (padrão 1 h), busca, limite (padrão 500).

## Monitor
- [ ] Polling 2 s só com a página visível; CPU total e por núcleo, GPU total e por dispositivo, memória, swap, disco e rede (taxas); processos em árvore; encerrar (SIGTERM).

## Data, hora e idioma
- [ ] Fuso, NTP, locale, keymap → `datetime().apply`.

## Personalização
- [ ] Visão geral: Tema (color-scheme prefer-light/prefer-dark), Cores, Ícones, Fontes, Cursores, Janelas, Papel de parede, Tela de bloqueio; Perfil, Menu, Dock; atalhos para Configurações/Ajustes/Extensões (filtrando `Exec=false`), Ajustes abrindo a página certa via AT-SPI.
- [ ] Perfis lyra/ubuntu/vanilla/windows10/windows11/macos com toda a lógica de `dock.rs` (suite `shell-suite`, snapshots por perfil, migrações, escrita atômica).
- [ ] Componentes da suíte (dock, painel, menus, busca, animações) e ícones da área de trabalho.
- [ ] Dock (aplicação imediata, alinhamento por modo, gating por contexto) e Menu/Painel.
- [ ] Tela de bloqueio (lock-enabled, lock-delay, idle-delay).

## Assistente de IA
- [ ] Anthropic/OpenAI/Gemini, modelos, limites, chaves no Secret Service (`secret-tool`), anexos, ferramentas (leitura e com confirmação obrigatória), redação, histórico/uso/auditoria com permissões 0600.

## Máquinas virtuais
- [ ] Conexões pessoal/sistema, busca, polling 5 s, abrir tela (`lyra-vms`), atalho, ciclo de vida, editar (renomear, CPU/memória, ampliar disco, ejetar ISO), criar (ISO/QCOW2/RAW, BIOS/UEFI) com progresso e cancelamento, remover com plano de arquivos, mensagens traduzidas.

## Notificador de atualizações
- [ ] Binário `vega-update-notifier` sem mudança de comportamento (GApplication, contagem, notificação, abre `vega-gtk` com `VEGA_START_PAGE=software`).

## Empacotamento
- [ ] Spec: BuildRequires `webkit2gtk3-devel` (webkit2gtk-4.1), dependências de runtime, `vendor.tar.gz` regenerado, mesmos arquivos instalados.
