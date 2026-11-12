# Luti Color Picker

Seletor de cores para Linux com lupa, histórico visual rolável, sem limite fixo de capturas e 11 formatos para copiar, incluindo HEX, RGB, HSL e LAB.

## Instalação

Extraia o pacote Linux e execute `./install.sh` dentro da pasta extraída. O instalador coloca o aplicativo e os atalhos em `~/.local`. Para executar pelo terminal, inclua `~/.local/bin` no `PATH` ou use `~/.local/bin/luti-color-picker`.

O pacote requer GTK 3.24 e as bibliotecas de sistema compatíveis com a máquina de compilação. Em Wayland, a lupa usa Spectacle no KDE ou grim em compositores compatíveis. A alternativa via XDG Desktop Portal depende do compositor e fornece a cor sem a imagem de contexto. Os sons usam `canberra-gtk-play`, quando disponível.

## Uso

Use o botão de captura e clique no pixel desejado. A roda do mouse ajusta o zoom. Escape ou o botão direito cancelam a seleção. Clique em um cartão para copiar a cor no formato selecionado ou use **Palette** no cartão para adicioná-la a uma paleta existente ou nova.

- **Palettes** salva paletas com nome e cores editáveis. Você pode adicionar a última captura e exportar em PNG, SVG com amostras editáveis, TXT, variáveis CSS, Tailwind CSS v3 e v4, JSON ou GPL para GIMP e Inkscape.
- **Gradients** permite ajustar cores, posições e ângulo, copiar o CSS ou exportar uma imagem PNG.
- **Image colors** extrai as cores dominantes de um arquivo e permite salvar o resultado como paleta.

A engrenagem abre **Settings**, onde você pode controlar a cópia automática, os sons e a permanência na bandeja após fechar a janela.

```sh
luti-color-picker --pick
luti-color-picker --about
luti-color-picker --background
```

O aplicativo salva o histórico, as paletas, o gradiente em edição e as preferências em `${XDG_DATA_HOME:-$HOME/.local/share}/luti-color-picker/state.json`. Se esse arquivo ainda não existir, ele lê o histórico anterior de `${XDG_CACHE_HOME:-$HOME/.cache}/colour-picker.json`.

## Compilação e pacote

Instale Rust, Cargo, as ferramentas de compilação e os arquivos de desenvolvimento do GTK 3.24.

```sh
cargo test --locked
cargo run --locked
./scripts/package-linux.sh
```

O pacote é gravado em `dist/`. Gere o pacote em uma distribuição compatível com o público que vai usá-lo.

Autor da versão Rust: **Lucas Tiago**. Veja o arquivo [LICENSE](LICENSE) para os termos MIT e os avisos de copyright preservados.

## Distribuição manual

Para gerar os pacotes localmente, execute:

```sh
make build
```

`make build` e `make dist` chamam `./scripts/build-package.sh all`. Os arquivos gerados ficam em `dist/`, ignorada pelo Git. O script continua após uma falha e lista os pacotes gerados, os formatos pulados por falta de ferramentas e as falhas de empacotamento. Use `./scripts/build-package.sh FORMATO` para gerar AppImage, `.run`, Debian, RPM, Arch, Flatpak ou Snap na sua máquina. O [guia de distribuição](docs/GUIA-RELEASE.md) lista as ferramentas, os comandos de instalação e um modelo para a release e a tag.
