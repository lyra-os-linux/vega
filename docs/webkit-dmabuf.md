# Renderer DMA-BUF do WebKit desligado — Vega #166

O `vega-gtk` define `WEBKIT_DISABLE_DMABUF_RENDERER=1` no início do `main`
(`vega-gtk/src/webkit_env.rs`). Sem isso, o WebKitGTK 2.52 abortava com
corrupção de heap em cerca de metade dos fechamentos do Vega, deixando um
coredump de 6 a 10 MB a cada vez.

## Causa

O renderer DMA-BUF guarda o dispositivo GBM num singleton estático
(`WebCore::GBMDevice`). No `exit()` do WebKitWebProcess, o destrutor
`~GBMDevice()` (`GBMDevice.cpp:63`) chama `gbm_device_destroy`, que desmonta a
tela DRI do Mesa (`st_screen_destroy`). Isso corre contra os destrutores do
Mesa e do glvnd (`__eglFini`). Conforme a ordem, o `free()` encontra o heap
corrompido ("corrupted double-linked list"). O defeito é do WebKit e também
aparece no Fedora (WebKitGTK 2.52.3, Red Hat 2464924 e 2478138).

## Escopo do contorno

- Só o renderer DMA-BUF é desligado. O compositing acelerado continua ligado,
  e o WebKit passa a compor por memória compartilhada.
- Um valor já definido pelo usuário é respeitado, inclusive `0`.
- Os apps gráficos que o Vega abre não herdam a variável: URIs e apps do
  GNOME (via `AppLaunchContext`), `malcontent-control` e `lyra-vms`.
  Ferramentas de linha de comando (`secret-tool`, `glib-compile-schemas`)
  não são afetadas por ela.

## Validação

Medido em 28/10/2026 num Acer Nitro AN515-57 (Intel UHD + GTX 1650, Mesa
26.0.7, WebKitGTK 2.52.5, NVIDIA 615.71.09), com 20 ciclos de abrir, esperar
8 s e encerrar:

| Variante | Coredumps |
| --- | --- |
| padrão | 10/20 |
| só Mesa (`__EGL_VENDOR_LIBRARY_FILENAMES`) | 11/20 |
| `WEBKIT_DISABLE_DMABUF_RENDERER=1` | 0/20 |
| `WEBKIT_DISABLE_COMPOSITING_MODE=1` | 0/20 |

Os fechamentos pelo botão X têm a mesma taxa (2/5 sem o contorno).

## Remoção

Retirar `webkit_env.rs` quando o WebKitGTK da base corrigir a destruição do
`GBMDevice` no encerramento. Para confirmar, repetir o ensaio acima sem a
variável.
