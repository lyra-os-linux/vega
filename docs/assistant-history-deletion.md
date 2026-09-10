# Exclusão do histórico — Vega #124

A opção de persistência controla leituras e novas gravações. Uma solicitação
explícita de apagar a conversa remove `ai-history.json` independentemente dessa
opção. O arquivo inclui as cópias dos anexos; os arquivos originais permanecem.

A limpeza também retira os registros legados `user_message`, `assistant_message`
e `user_attachment` de `ai-audit.jsonl`. Esses tipos deixam de ser gravados.
Eventos operacionais têm retenção separada, descrita em
[Privacidade do assistente](ai-privacidade.md#limpar-conversa-e-retenção-local).
A reescrita usa um arquivo temporário privado e substituição por rename; as duas
exclusões não constituem uma transação atômica conjunta. Uma falha pode ocorrer
depois de remover parte das cópias. A interface informa o erro e permite repetir.

A operação de disco roda fora da thread GTK. Conversa, rascunho e anexos pendentes
só são removidos da interface após sucesso. Envio, importação e limpeza não
podem se sobrepor na mesma página. Não há sincronização com escritores externos.

## Verificação reproduzível

```sh
cargo test --workspace --locked
xvfb-run -a dbus-run-session -- cargo test --workspace --locked native_history_ui -- --ignored --test-threads=1
python3 -m unittest discover -s tests -v
```

Os testes usam subprocessos com diretórios XDG temporários definidos antes da
inicialização do GLib. A suíte de armazenamento cobre persistência real,
reativação sem reaparecimento, anexos, retenção operacional, exclusão repetida,
dados ausentes, log inválido, erros de remoção e links simbólicos. O teste GTK
exercita widgets reais, preservação da apresentação em erro e limpeza completa
após corrigir o erro. Não usa provedor de IA nem transações de pacotes.

Em 10/09/2026, o cenário salvar → desativar → limpar → reativar foi executado
contra a base `cbb8d99a7c6244ced5dc5cfcc7b1443eb54c505d`: a função retornou
sucesso, mas o teste falhou porque o arquivo continuava presente. Com a correção,
passaram os 32 testes Rust padrão (sete cenários de armazenamento em
subprocessos), o teste GTK executado separadamente e os dez testes Python.
Gettext real foi verificado em pt-BR, en-US e es-ES. Fmt e Clippy também passaram.
Isso não substitui a qualificação do próximo RPM ou da ISO.
