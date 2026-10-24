# Migração do vega-gtk para Tauri: lista de paridade

A interface do Vega passa de GTK4/libadwaita para Tauri 2 (WebKitGTK) na 16.1,
por exceção ao congelamento aprovada pelo mantenedor em 24/10/2026. Requisito:
**todas** as funções do vega-gtk continuam existindo. O pacote, o binário
`vega-gtk`, o `vega-update-notifier`, o app id `org.lyraos.Vega` e o contrato
`vega-gtk --desktop-profile get|set <id>` (usado pelo lyra-welcome) não mudam.

Marque cada item ao portar. A referência é o vega-gtk antes da migração
(commit aec835d).

## Global
- [x] App id `org.lyraos.Vega`, instância única; segunda ativação reapresenta a janela.
- [x] `--virtual-machines` abre Máquinas virtuais; `VEGA_START_PAGE=software` abre Atualizações; ação `app.open-updates`.
- [x] `--desktop-profile get|set` antes de qualquer janela (códigos de saída 2 uso, 1 falha/readback diferente; `gio::Settings::sync()` após set).
- [x] `VEGA_BENCHMARK_MARKER` → `VEGA_WINDOW_READY_MS=` no stderr.
- [x] Uma conexão `VegaDbus`; falha marca o painel como indisponível e tenta de novo no próximo refresh.
- [x] Refresh do painel a cada `refresh_interval_minutes` (1–60), coalescido (1 em voo + 1 pendente).
- [x] Navegação: dashboard, software, backup (Backups | Pontos de restauração), virtualization, assistant, hardware (Hardware | Kernel), datetime, screen (Personalização), monitor, storage, network, desktop (Bluetooth), services, users, logs. Busca filtra a navegação.
- [x] Página inicial configurável (qualquer id).
- [x] Menu: Configurações; Sobre (versão `VEGA_VERSION`, site, issues, GPL-3.0, "Rodrigo Brito").
- [x] Política de confirmação: `required` sempre; `confirm` pulado se `confirm_actions=false`, exceto quando há formulário/revisão; ações de pacote, NVIDIA, rollback, chave de repositório e preparação sempre perguntam.
- [x] i18n gettext domínio `vega-gtk` (pt-BR, en-US, es-ES), msgids em português, resolução de locale de `i18n.rs`, idioma da preferência.

## Preferências (`$XDG_CONFIG_HOME/vega-gtk/preferences.json`)
- [x] language, start_page, confirm_actions, refresh_interval_minutes, notify_updates, notify_service_failures, notify_backups, redact_ai_data, save_ai_history (desligar apaga o histórico).

## Painel
- [x] Backend (`system().status()`), Sistema, Atualizações (`list_updates`, reage a `UpdatesAvailable`), Backup (`list_configs`), Pontos de restauração (`available` + `list`; corrigir o link para backup/snapshots), Serviços (`services().list()`, "habilitado mas parado"; notificação "Falha em serviços"), Disco (`system().disk_usage()`).
- [x] Inventário de hardware e firmware carregados junto.

## Software
- [x] Painel de preparação: `preparation().status()` a cada 5 s com a página visível; estados/fases/erros; "Tentar novamente" (`retry`); "Revisar chave…" → `approve_key` + transação.
- [x] Abas Buscar / Instalados / Atualizações / Repositórios; visão lista/cartões.
- [x] Buscar (≥2 caracteres, `search`), agrupamento por nome, origem oficial < flathub < outra, fila de instalação (sequencial, Flathub primeiro, resumo de falhas).
- [x] Detalhes (`package_details`) com instalar/remover.
- [x] Instalados (`list_installed`).
- [x] Atualizações (`list_updates`) agrupadas por repositório; Atualizar uma (`update_package`); Atualizar tudo (`update_all`); Limpar cache (`clear_cache`).
- [x] Repositórios (`list_repos`, `set_repo_enabled`, `add_repo` + `KeyPending` → `trust_repo_key`, inclusive sem key_id).
- [x] Painel de transação: progresso, progresso por pacote, console (`ConsoleLine`, limite 10 000 linhas/4 MiB), falha com alerta; recarregar a aba e o contador do painel ao terminar.

## Backup (Restic)
- [x] `list_configs`, `list_snapshots`, `list_snapshot_paths` (100 por vez), nova configuração (id, caminhos, destino, UUID, frequência manual/daily/weekly), excluir, executar agora (eventos + notificação "Backup concluído"), restaurar itens (pasta separada | substituir), alertas.

## Pontos de restauração (Snapper)
- [x] `available`, `list` (sem #0), comparar (`diff_packages`), criar, excluir, limpar todos com progresso, rollback com revisão obrigatória, retenção 1–100.

## Hardware e Kernel
- [x] Processador, vídeo, memória, firmware.
- [x] NVIDIA: capacidade `nvidia-official-v1`, `nvidia_status`, mensagens por estado, Secure Boot, snapshot de recuperação, "Atualizar diagnóstico", instalar/ativar integração com diálogo obrigatório (`install_nvidia(true)`).
- [x] Kernel: instalados, disponíveis não instalados, bootloader (entrada padrão, timeout 0–120, cmdline) com `apply_boot_config`.

## Armazenamento
- [x] `storage().list()`, montar/desmontar.

## Rede
- [x] Interfaces (IPv4 estático validado via `set_static_ipv4`), Wi-Fi (conectar com senha se protegida, desconectar), Proxy (`set_proxy`, remover), VPN (importar .ovpn), Firewall (status, serviços, portas tcp/udp).

## Bluetooth
- [x] Status/indisponível, ligar, buscar, parear/conectar/desconectar, enviar arquivo, receber arquivos.

## Serviços
- [x] Principais (`list`) | Todos (`list_all`); habilitar, iniciar/parar, reiniciar.

## Usuários
- [x] Listar; root imutável; criar/editar (nome, login validado, senha ≥8 com confirmação, grupos, foto PNG/JPEG ≤5 MB, admin padrão ligado); tornar/remover admin; remover.

## Logs
- [x] Carga preguiçosa (sem polkit na abertura); unidade, prioridade, período (padrão 1 h), busca, limite (padrão 500).

## Monitor
- [x] Polling 2 s só com a página visível; CPU total e por núcleo, GPU total e por dispositivo, memória, swap, disco e rede (taxas); processos em árvore; encerrar (SIGTERM).

## Data, hora e idioma
- [x] Fuso, NTP, locale, keymap → `datetime().apply`.

## Personalização
- [x] Visão geral: Tema (color-scheme prefer-light/prefer-dark), Cores, Ícones, Fontes, Cursores, Janelas, Papel de parede, Tela de bloqueio; Perfil, Menu, Dock; atalhos para Configurações/Ajustes/Extensões (filtrando `Exec=false`), Ajustes abrindo a página certa via AT-SPI.
- [x] Perfis lyra/ubuntu/vanilla/windows10/windows11/macos com toda a lógica de `dock.rs` (suite `shell-suite`, snapshots por perfil, migrações, escrita atômica).
- [x] Componentes da suíte (dock, painel, menus, busca, animações) e ícones da área de trabalho.
- [x] Dock (aplicação imediata, alinhamento por modo, gating por contexto) e Menu/Painel.
- [x] Tela de bloqueio (lock-enabled, lock-delay, idle-delay).

## Assistente de IA
- [x] Anthropic/OpenAI/Gemini, modelos, limites, chaves no Secret Service (`secret-tool`), anexos, ferramentas (leitura e com confirmação obrigatória), redação, histórico/uso/auditoria com permissões 0600.

## Máquinas virtuais
- [x] Conexões pessoal/sistema, busca, polling 5 s, abrir tela (`lyra-vms`), atalho, ciclo de vida, editar (renomear, CPU/memória, ampliar disco, ejetar ISO), criar (ISO/QCOW2/RAW, BIOS/UEFI) com progresso e cancelamento, remover com plano de arquivos, mensagens traduzidas.

## Notificador de atualizações
- [x] Binário `vega-update-notifier` sem mudança de comportamento (GApplication, contagem, notificação, abre `vega-gtk` com `VEGA_START_PAGE=software`).

## Empacotamento
- [x] Spec: BuildRequires de WebKitGTK 4.1, GTK 3 e libsoup 3; mesmos arquivos instalados.
- [ ] `vendor.tar.gz` regenerado no OBS (`_service` cargo_vendor) antes da submissão.
