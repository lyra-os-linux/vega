# Contexto das configurações — Vega 5.1.42

Dock, Painel, Menus, Busca e Animações condicionam seus controles ao estado
dos respectivos componentes. Falha ao consultar o estado mantém controles
dependentes indisponíveis. O contexto é atualizado ao entrar na página e após
alterar perfil ou componentes.

Margens, posição dos aplicativos em execução e atraso para ocultar dependem
das opções correspondentes. A margem do painel também fica indisponível com
dock estendido ou perfil Clássico/Central. A maximização pode suspender essa
margem temporariamente na suíte, conforme a explicação exibida.

Lyra Flutuante mostra menus fixos à esquerda, preservando a preferência de
posição usada pelos outros perfis. Minimizar tem um grupo de Lyra Animações,
separado dos efeitos do dock. O atraso do bloqueio requer bloqueio ativo.
Nenhuma dessas transições apaga os valores salvos.

O teste GTK `ui::settings_context_tests::native_settings_context` verifica
sensibilidade, retorno entre contextos e ausência de gravação provocada pela
atualização visual. Usa GSETTINGS_BACKEND=memory, sem alterar a sessão do usuário.
Os cenários de integração com GNOME e a ISO exata permanecem na qualificação
da candidata, posterior à auditoria final.
