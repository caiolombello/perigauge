#!/usr/bin/env python3
"""Calcula razões de contraste WCAG 2.x a partir de tokens.json."""
import json, pathlib

def lum(h):
    c = [int(h[i:i+2], 16) / 255 for i in (1, 3, 5)]
    c = [x / 12.92 if x <= 0.03928 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]

def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)

t = json.loads((pathlib.Path(__file__).parent / "tokens.json").read_text())
for mode in ("light", "dark"):
    m = t[mode]
    print(f"[{mode}]")
    for fg in ("text", "muted", "accent", "ok", "warn", "critical", "charging"):
        for bg in ("paper", "surface", "surface2"):
            r = ratio(m[fg], m[bg])
            print(f"  {fg:9} on {bg:8} {r:5.2f}:1 {'AA' if r >= 4.5 else 'FAIL'}{' AAA' if r >= 7 else ''}")
    print(f"  line on paper (UI, 3:1) {ratio(m['line'], m['paper']):.2f}")
print("[botão] ", end="")
print(f"light #FFFFFF on accent {ratio('#FFFFFF', t['light']['accent']):.2f}; dark #0E1124 on accent {ratio('#0E1124', t['dark']['accent']):.2f}")
print(f"[ícone] #A9B4FF on #151A33 {ratio('#A9B4FF','#151A33'):.2f}; #F2F4FF on #151A33 {ratio('#F2F4FF','#151A33'):.2f}; track #4B5490 on #151A33 {ratio('#4B5490','#151A33'):.2f}")
