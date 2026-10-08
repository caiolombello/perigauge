# Identidade PeriGauge

**Ideia:** um medidor (arco de ~270°, aberto embaixo) com um mouse-cápsula no centro. O arco sólido é o nível; o trecho restante é a "trilha". Sem o mouse seria só um velocímetro; sem o arco, só um mouse. Funciona em 16/24/32 px e em monocromático: a trilha some e ainda restam arco parcial + cápsula.

## Arquivos
- `perigauge.svg` — ícone do app (fundo `#151A33`, arco `#A9B4FF`, trilha `#4B5490`, mouse `#F2F4FF`).
- `perigauge-symbolic.svg` (= 24 px) e `perigauge-symbolic-{16,24,32}.svg` — monocromáticos, `fill/stroke: currentColor` com a classe `ColorScheme-Text` (padrão Breeze/KDE). Geometria inteira: círculos de raio 10 (24 px; ponta em 3-4-5×2) e 6,5/13 (16/32 px; ponta 5-12-13), cápsulas e fendas em coordenadas inteiras. A trilha usa `opacity=.35`, como nos ícones Breeze.
- `perigauge-wordmark-{light,dark}.svg` — marca + nome. **Limitação:** o nome é `<text>` com pilha de fontes do sistema, não vetorizado; o furo do mouse é pintado com a cor do fundo (branco no claro, `#0E1124` no escuro).
- `tokens.json` / `tokens.css` — cores claro/escuro e estados. `check_contrast.py` recalcula os contrastes.

## Estados (nunca só cor)
| Estado | Forma | Rótulo (PT / EN) |
|---|---|---|
| ok | círculo cheio ● | Bom / Good |
| aviso | triângulo ▲ | Baixa / Low |
| crítico | losango/octógono ◆ com "!" | Crítica / Critical |
| carregando | raio ⚡ | Carregando / Charging |
| leitura antiga | círculo tracejado ◌ | Leitura antiga / Stale reading |

Além disso o comprimento do arco codifica o nível. Todo estado exibido deve trazer forma **e** texto.

## Contraste medido (WCAG 2.x, `python3 -I check_contrast.py`)
| Par | Claro | Escuro |
|---|---|---|
| texto `#151A33` / `#F2F4FF` sobre fundo | 15,99:1 | 17,04:1 |
| texto secundário sobre fundo | 7,15:1 | 9,75:1 |
| acento (links) sobre fundo | 6,06:1 | 9,48:1 |
| ok / aviso / crítico / carregando sobre fundo | 6,18 / 5,92 / 6,11 / 5,59 | 9,95 / 11,59 / 8,18 / 10,17 |
| botão (`#FFFFFF` / `#0E1124` sobre acento) | 6,49:1 | 9,48:1 |
| ícone: arco `#A9B4FF` e mouse `#F2F4FF` sobre `#151A33` | 8,68 / 15,61 | — |

Todos os textos ≥ 4,5:1 (AA) sobre `paper`, `surface` e `surface2`. A trilha do ícone (`#4B5490`, 2,42:1) e as linhas divisórias (`line`, ~1,5–1,8:1) são decorativas; o significado nunca depende delas.

## Regras de uso
- Painel: use o simbólico; deixe o sistema colorir. Nunca aplique cor de estado ao ícone do painel sem alterar também a forma/rótulo.
- Ilustrações de painel/popup são **conceitos** com valores fictícios, nunca screenshots, até existir um build real.
- Não use o ícone do app abaixo de 32 px; abaixo disso, use o simbólico.
- Sem fontes externas, CDN ou rastreamento. Busca web pelo nome "PeriGauge" (2026-10-08) não achou produto homônimo exato; não é uma checagem de marca registrada.
