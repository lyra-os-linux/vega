# Atualização do Painel — Vega #146

Cada clique no botão Painel solicita uma atualização, inclusive quando a aba já
está selecionada. A abertura da janela e o temporizador usam o mesmo controle:
existe uma consulta do resumo em execução e, no máximo, uma repetição pendente.
As consultas dos cards continuam assíncronas e concorrentes entre si; os ciclos
do resumo são serializados. Pedidos durante a repetição podem solicitar outro
ciclo, sem criar uma fila proporcional ao número de cliques.

O temporizador continua usando a preferência existente: 5 minutos por padrão,
limitada a 1–60 minutos. O clique não depende do vencimento desse intervalo.

Todos os chamadores do card de atualizações (resumo, sinal do daemon e conclusão
de operações de software) compartilham um segundo controle. Uma consulta antiga
não pode concluir depois de uma consulta mais nova desse mesmo card. Falhas de
System.Status afetam apenas Backend/Sistema, preservando os resultados dos
outros cards. Se a conexão inicial ao D-Bus falhar, o próximo pedido tenta
conectar novamente, configurando as páginas uma única vez após conectar.

## Validação

- `cargo fmt --check`
- `cargo clippy --locked --all-targets -- -D warnings`
- `cargo test --locked --workspace`: 46 testes aprovados; testes gráficos
  marcados como ignorados são executados separadamente.
- `python3 -m unittest discover -s tests -q`: 12 testes aprovados.
- `cargo test --locked --workspace native_dashboard_ -- --ignored --test-threads=1`:
  dois testes com GTK real, D-Bus privado e diretórios/configurações isolados.
  Cobrem 50 cliques durante consulta retida, uma repetição, retorno de outra aba,
  clique na aba ativa, falha de status sem sobrescrever Disco/Backup e recuperação.
  O segundo teste retém ListUpdates, agrupa outros 50 pedidos e verifica sucesso
  após falha e apresentação de erro numa consulta posterior.
- `cargo test --locked --workspace native_dialog_flows -- --ignored --test-threads=1`:
  regressão dos formulários e callbacks de software que compartilham o card.

O CI executa os dois novos testes gráficos sob Xvfb. A fixture não modifica
pacotes, serviços nem preferências da sessão do usuário. Publicação RPM e
validação na candidata Alpha 8 ainda são etapas posteriores.
