# Formulários e confirmações

A preferência **Confirmar ações administrativas** controla apenas confirmações
opcionais de ações que o usuário já definiu. Desativá-la não preenche nem envia
formulários automaticamente.

- `required_dialog` sempre aguarda uma resposta: configuração de backup, IPv4,
  descrição de snapshot, aprovação de propostas da IA, revisão de rollback e
  confiança em chaves de repositório.
- `confirm_dialog` pode dispensar uma confirmação simples, mas sempre apresenta
  diálogos com `extra_child`. Essa proteção atende também ao diálogo de Wi-Fi:
  redes protegidas coletam senha; redes abertas e desconexão seguem a preferência.
- Cancelar ou fechar continua interrompendo a ação. Validações de campos e
  autorização do backend permanecem nos fluxos existentes.
- Adicionar repositório usa um formulário na própria página Software. Nome e
  URL continuam obrigatórios com a preferência ligada ou desligada.

A descrição da preferência informa a regra em português, inglês e espanhol.
Propostas da IA sempre exigem aprovação explícita, conforme a política de
[privacidade do Assistente](ai-privacidade.md).

## Validação da issue #125

`application::dialog_tests::native_dialog_flows` instancia os widgets GTK reais,
aciona os callbacks da aplicação e verifica os parâmetros recebidos por um
serviço D-Bus simulado. O teste cria um barramento privado sem ativação de
serviços e diretórios XDG temporários. Nenhum pedido alcança o vegad do host.
O serviço registra as mutações e devolve erro deliberadamente; não instala
pacotes, altera rede ou cria backups/snapshots.

Com a preferência ligada e desligada, cobre:

- backup: cancelar, fechar, recusar campos vazios e enviar campos preenchidos,
  caminhos separados por vírgula, UUID e frequência semanal;
- snapshot, IPv4 e Wi-Fi: aguardar entrada e enviar os dados preenchidos;
- repositório: campos vazios não enviam pedido; nome e URL preenchidos chegam
  ao backend sem espaços nas extremidades;
- IA: instalar, remover e limpar cache, cada ação aprovada e rejeitada;
- rollback: mostrar as diferenças retornadas pelo backend antes da decisão;
- repositório com chave verificável ou sem chave: exibir os detalhes e aguardar
  aprovação, com cancelamento sem pedido de confiança;
- confirmação simples: respeitar a preferência.

Na base `73c18a747715390e557e9ac76cab5f85dfad8cd7`, o mesmo teste passa pela
etapa de confirmações ligadas e falha quando deveria apresentar o formulário de
backup com confirmações desligadas. A correção passa nas duas etapas.

Execução em CI, com Xvfb:

```sh
xvfb-run -a cargo test --workspace --locked native_dialog_flows -- --ignored --test-threads=1
```

Validação local também executada em Mutter headless/Wayland. O GTK emitiu avisos
de largura mínima de rótulos nos diálogos tanto na base quanto na correção;
esta qualificação verifica interação e payloads, não a aparência completa dos
diálogos ou acessibilidade. Não substitui testes de operações reais em VM/RPM.

O ensaio também identificou que um erro em `SetStaticIpv4` deixa o botão da
interface desabilitado. Esse problema independente está registrado na
[issue #134](https://github.com/lyra-os-linux/vega/issues/134).
