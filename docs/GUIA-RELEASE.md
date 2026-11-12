# Luti Color Picker 1.0.1

## Compilação manual

A partir da raiz do projeto, rode `make build` (ou `make dist`) para tentar gerar os sete formatos. O alvo executa `./scripts/build-package.sh all`. O script continua com os demais formatos e informa ao final quais foram gerados, quais foram pulados por falta de ferramentas e quais falharam. Para gerar ou repetir apenas um formato, use o comando correspondente na tabela.

Os arquivos são gravados em `dist/`, que o Git ignora. Você pode informar outra pasta como segundo argumento.

| Formato | Comando | Ferramentas adicionais |
| --- | --- | --- |
| AppImage | `./scripts/build-package.sh appimage` | `appimagetool` |
| Debian e Ubuntu | `./scripts/build-package.sh deb` | `dpkg-deb` |
| Fedora e derivados | `./scripts/build-package.sh rpm` | `rpmbuild` e ferramentas de empacotamento RPM da distribuição |
| Arch e derivados | `./scripts/build-package.sh arch` | `makepkg` e `base-devel`, como usuário comum |
| AppRun portátil | `./scripts/build-package.sh run` | GNU tar e gzip. O arquivo executável `.run` extrai o AppDir e abre o AppRun |
| Flatpak | `./scripts/build-package.sh flatpak` | `flatpak`, `flatpak-builder` e o remoto Flathub no escopo do usuário |
| Snap | `./scripts/build-package.sh snap` | Snapcraft, com o ambiente de compilação configurado para core24 |

Para os cinco formatos nativos, instale Rust, Cargo, os arquivos de desenvolvimento do GTK 3.24, `pkg-config` e GNU binutils. Compile os pacotes nativos na distribuição compatível mais antiga que você pretende atender e repita a compilação em cada arquitetura desejada. A versão mínima de glibc depende do sistema de compilação. AppImage e `.run` usam GTK e outras bibliotecas da máquina de destino. Para gerenciar um AppImage sem FUSE durante a compilação, use `APPIMAGE_EXTRACT_AND_RUN=1` se o seu `appimagetool` for distribuído como AppImage.

O Flatpak compila no SDK GNOME 49 e usa uma cópia local das dependências Rust. Se o remoto estiver ausente, configure com:

```sh
flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
```

Flatpak e Snap usam ambientes isolados. Em Wayland, a captura depende do portal disponível no desktop quando o pacote não tem acesso ao Spectacle ou ao grim. Nesse modo, o portal pode fornecer só a cor, sem lupa própria e sem miniatura do entorno. Teste a captura, a área de transferência e a bandeja em cada formato antes de publicar.

Referências de empacotamento: [AppImage](https://docs.appimage.org/packaging-guide/manual.html), [Flatpak](https://docs.flatpak.org/en/latest/first-build.html) e [Snapcraft](https://documentation.ubuntu.com/snapcraft/stable/explanation/parts-lifecycle/).

## Testar os pacotes

- AppImage: `chmod +x ./luti-color-picker-*.AppImage` e execute o arquivo.
- AppRun: `chmod +x ./luti-color-picker-*.run` e execute o arquivo.
- Debian e Ubuntu: `sudo apt install ./luti-color-picker_*.deb`.
- Fedora: `sudo dnf install ./luti-color-picker-*.rpm`.
- Arch: `sudo pacman -U ./luti-color-picker-*.pkg.tar.zst`.
- Flatpak: `flatpak install --user ./luti-color-picker-*.flatpak`.
- Snap local: `sudo snap install --dangerous ./luti-color-picker-*.snap`. Esse parâmetro é exigido para um arquivo local sem assinatura da Snap Store.

No aplicativo instalado, abra **Sobre → Créditos** e confirme que Lucas Tiago aparece. Faça uma captura e confirme que o valor copiado corresponde ao formato selecionado.

## Tag anotada

Crie a tag após revisar e confirmar as alterações num commit. O comando abaixo cria a tag localmente.

```sh
git tag -a v1.0.1 -m "Luti Color Picker 1.0.1 — paletas, gradientes, extração de cores e histórico rolável"
```

Quando for publicar, envie o commit da versão e depois a tag com `git push origin v1.0.1`. Abra uma release no GitHub para essa tag, cole o texto abaixo e anexe os arquivos que você compilou e testou.

## Texto para o GitHub Release

### Luti Color Picker 1.0.1

- **Histórico** com rolagem e sem limite fixo de capturas. Use **Palette** em qualquer cartão para enviar a cor a uma paleta existente ou criar uma nova.
- **Paletas editáveis** com exportação em SVG com amostras vetoriais, PNG, TXT, JSON, GPL, variáveis CSS e Tailwind CSS v3 e v4.
- **Gradientes lineares** com prévia, controle de ângulo e posição de cada cor, cópia de CSS e exportação em PNG.
- **Cores de imagens** com extração de cores dominantes e salvamento como paleta.
- **Interface** com botão **Copy** visível ao passar o mouse e aba **Créditos** funcional.
- **Empacotamento local** com `make build` e correção do empacotamento em sistemas configurados em português.

Os anexos incluem AppImage, AppRun (`.run`), Debian/Ubuntu (`.deb`), RPM e Arch (`.pkg.tar.zst`) para **Linux x86_64**. Os binários requerem GTK 3.24 e glibc 2.39 ou superior. O código inclui receitas de Flatpak e Snap. Esses dois formatos ainda precisam ser compilados e testados.
