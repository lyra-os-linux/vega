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
pacotes, serviços nem preferências da sessão do usuário. A validação na candidata Alpha 8 permanece pendente.


## Publicação 5.1.40 — 20/09/2026

[OBS1379318](https://build.opensuse.org/request/show/1379318) aceito: staging22 e
release110, fontes Git `99b039bc6d6f9bc62c697b9af1668434e1410fee`,
[CI35533662810](https://github.com/lyra-os-linux/vega/actions/runs/35533662810) aprovado.
Leap16.1 e Tumbleweed publicados. RPM público Leap
`vega-gtk-5.1.40-lp161.1.1.x86_64.rpm`, SHA256
`1564ec93722bfd35230570679c08ba585899d38876fcf4475ca8323f2ac321ff`.
Assinatura7edca82e válida; download público idêntico ao RPM da API de release.

O teste AT-SPI no RPM5.1.39 reproduziu a ausência de refresh ao clicar na aba
ativa. O RPM5.1.40 passou sete verificações: abertura, clique ativo, retorno de
Software, 30 cliques agrupados em uma repetição, erro exibido, Disco preservado
quando Status falha e recuperação. Backend e configurações isolados. Passaram
52 verificações do comando de perfis com os schemas da suíte. O binário da
release é idêntico ao testado em staging. Gates completos dos canais staging e
release passaram; rpmlint: zero erros, seis avisos de empacotamento existentes.

[Comprovantes portáveis](dashboard-obs-evidence.json). O DesktopPR96 exige
Vega>=5.1.40 na imagem. Nenhuma instalação do RPM na estação nem geração de ISO
nesta entrega; VEGA-01 permanece pendente de qualificação da candidata exata.
Rollback: revisão109 da release, pelo fluxo de staging e nova qualificação.
