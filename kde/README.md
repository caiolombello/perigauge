# perif-battery para KDE Plasma 6

Plasmoid nativo para acompanhar a bateria do Keychron M6 e Logitech MX Keys.
O leitor Python usa apenas a biblioteca padrão, o HID local do Keychron e o
comando local `solaar`; não acessa a rede e não requer `sudo`.

## Instalação

```bash
./install.sh
```

O comando instala o leitor em `~/.local/bin/perif-battery` e o pacote Plasma
`com.opsteam.perifbattery`, mas **não altera nem adiciona nada a painéis**.
Depois, adicione manualmente “Bateria de periféricos” no modo de edição do
painel. A leitura ocorre a cada 60 segundos; o popup e o menu de contexto têm
“Atualizar agora”.

O indicador compacto só mostra periféricos conectados: ícone de mouse/teclado,
percentual quando houver espaço, amarelo abaixo de 36% e vermelho abaixo de
16%. O popup também mostra estado, cache/defasagem e a última leitura.

Quando o Solaar só fornecer um nível textual (por exemplo, `low`), o widget o
exibe como “aprox.” sem inventar um percentual. Erros de acesso ao receptor,
Solaar ausente, timeout ou dispositivo em repouso aparecem somente no popup;
o indicador compacto permanece discreto.

## Remoção

```bash
./uninstall.sh
```

O desinstalador só remove o helper se ele ainda for idêntico ao arquivo deste
repositório, evitando apagar uma personalização posterior.

## Desenvolvimento e validação

```bash
python3 -m unittest discover -s tests -v
python3 -m py_compile perif-battery
shellcheck install.sh uninstall.sh
python3 -m json.tool package/metadata.json >/dev/null
plasmawindowed com.opsteam.perifbattery
```

O cache privado fica em `~/.cache/perif-battery/last.json` (diretório `0700`,
arquivo `0600`).
