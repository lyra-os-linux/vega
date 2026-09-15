# Ecosystem #25 — GNOME, idiomas e RPM

Correções locais de 15/09/2026. Escopo: Vega GTK/vegad e integração GNOME.
O feedback sobre o antigo Vega Qt não foi tratado como reprodução no produto
atual. RPM permanece o formato de distribuição; não foi criada versão Flatpak.

## Diagnóstico e correções

| Relato / condição | Classificação e tratamento |
| --- | --- |
| Interface sempre em português | Não reproduzido no RPM instalado 5.1.38 com locales completos: abriu em PT-BR, EN-US e ES-ES. |
| Idioma do processo ignorado | A resolução anterior priorizava AccountsService e ignorava listas LANGUAGE. Agora segue o ambiente, com preferência explícita do aplicativo acima dele. |
| Ausência de seletor | Adicionado em Menu principal → Configurações → Idioma do Vega; aplica ao reabrir, sem root ou alteração do idioma do GNOME. |
| Instalação mínima sem locale regional | Reproduzido: gettext em C/C.UTF-8 exibe os textos-fonte em português. Fallback usa en_US.UTF-8 de glibc-locale-base, agora dependência explícita, e LANGUAGE escolhe o catálogo. |
| Checkout escondendo falha de empacotamento | Binários instalados deixam de procurar os MOs do checkout; compilação falha se não puder gerar os catálogos. Instalador manual também copia os três MOs. |
| Atalhos de personalização sem programas | Requer explicitamente gnome-control-center, gnome-tweaks e gnome-extensions, além dos provedores Python/AT-SPI já existentes. |
| Repositório inadequado | Script estava fixado em Leap 16.0 e só configurava Vega. Agora detecta a base, configura Lyra para a dependência Sheliak e recusa alias existente com outro endereço. CLI mantém instalação própria. |
| Candidato novo usando apenas repositórios publicados | Solver recusou corretamente sheliak >= 2.0.0 indisponível nesse conjunto. Publicar Vega + suíte + daemon compatíveis em lote; não remover Requires nem ignorar dependências. |

Prioridade: preferência do aplicativo; lista GNU LANGUAGE; LC_ALL, LC_MESSAGES,
LANG. Valores vazios/C/POSIX nas variáveis de locale são ignorados para manter
o comportamento dos launchers do desktop. Regiões en/pt/es usam o catálogo
disponível; outros idiomas usam inglês. A ordem de LANGUAGE é descrita pelo
[GNU gettext](https://www.gnu.org/software/gettext/manual/html_node/The-LANGUAGE-variable.html).
O notificador usa a mesma resolução quando inicia; uma instância já aberta
adota a nova preferência no próximo início, normalmente no próximo login.

## Evidências e limites

Artefatos locais: `analysis/2026-09-15/gnome-feedback-25/` no workspace LyraOS.
RPM de teste: 5.1.38-0.local3, perfil Cargo dev sem símbolos de depuração,
com fontes locais sem commit registradas por hash; não é build release do OBS.
Somente Vega GTK foi atualizado no host, por Zypper, sem ignorar dependências.

Testes: Rust/Clippy, catálogos completos e placeholders nos três idiomas,
regressões do script de repositórios; matriz de UI com preferências temporárias,
idioma automático/forçado/fallback/lista e abertura pelo arquivo desktop.
Catálogos do checkout e locales regionais completos ficam ocultos no ensaio.

A raiz de runtime é formada pelos payloads do conjunto de 587 RPMs resolvidos (incluindo DejaVu como fonte básica do ensaio),
sem Cargo, Rust, Go, GCC ou pacotes -devel e sem montar o SDK do host.
A matriz final passou nove cenários no RPM instalado e nove nessa raiz. A gravação dos quatro valores foi validada em um teste nativo GTK do controle e seus callbacks, pois AT-SPI não expõe a ação do ComboRow.
O solver parte de rpmdb vazia com chaves públicas já confiadas no host.
Essa combinação comprova resolução e runtime; extração de payloads não deve
ser descrita como instalação limpa completa, teste dos scriptlets ou boot de
uma ISO. A sessão GNOME e o vegad usados na interação nativa são os do host.
O teste completo de uma instalação GNOME nova permanece na qualificação das
ISOs após a auditoria #78.

Firmware/TDX: correção e ensaios no repositório Desktop, documento
`docs/installer-firmware-requirements.md`. UEFI exigido; TDX não requerido.

## Entrega

A qualificação acima foi realizada antes da integração das fontes no GitHub.
O RPM continua sendo uma instalação de teste local; a publicação OBS e os
gates restantes da issue não estão cobertos por esta evidência. Atualizar os
snapshots do kit offline #4 após a integração; o kit anterior não muda.
Para reverter somente a instalação de teste, reinstalar o RPM local anterior
5.1.38-0.local1 por Zypper com permissão de downgrade; as preferências novas
são compatíveis com a desserialização anterior e não mudam o locale do GNOME.
