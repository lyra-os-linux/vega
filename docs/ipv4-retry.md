# Nova tentativa de configuração IPv4 — Vega #134

O botão Configurar IPv4 fica indisponível durante a aplicação e a atualização
da lista de interfaces. Ao terminar, inclusive em erro, sua disponibilidade é
recalculada pela seleção atual. Uma recusa do backend permite preencher o
formulário e tentar novamente sem trocar de página ou conexão. Trocar a seleção
durante a chamada não libera uma segunda aplicação concorrente.

O teste `application::dialog_tests::native_ipv4_retry` usa widgets GTK reais,
um barramento D-Bus privado e configurações temporárias. O serviço simulado
retém as chamadas até o teste escolher sucesso ou erro; nenhuma configuração
alcança a rede do host. Com confirmações ligadas e desligadas, verifica recusa
e nova tentativa com dados corrigidos, sucesso seguido de falha na recarga,
recarga bem-sucedida, troca/remoção da seleção durante a chamada e conexão que
desaparece da lista. Os argumentos de todas as aplicações são conferidos.

```sh
xvfb-run -a cargo test --workspace --locked native_ipv4_retry -- --ignored --test-threads=1
```

Em 11/09/2026, a base `8df26eda43a92161455760ea3b9e8acfe4568507`, compilada
com o mesmo teste, falhou com `denial left IPv4 action disabled`. A correção
passou localmente em Mutter/Wayland isolado, junto de 32 testes Rust padrão,
dez testes Python, fmt, Clippy estrito e verificações de contrato D-Bus/spec RPM.
O ensaio valida o comportamento da interface e o transporte simulado; não
qualifica reconexão real, NetworkManager/Polkit, aparência completa ou RPM/ISO.
