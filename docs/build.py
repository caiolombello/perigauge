"""Generate docs/index.html (PT) and docs/en/index.html (EN) from one source."""
import pathlib
DOCS = pathlib.Path(__file__).resolve().parent

def ic(n, cls=""):
    return f'<svg class="{cls}" aria-hidden="true" focusable="false"><use href="#i-{n}"/></svg>'
def st(kind, label):
    return f'<span class="state s-{kind}">{ic({"ok":"ok","warn":"warn","critical":"crit","charging":"bolt","stale":"stale"}[kind])}{label}</span>'

TPL = r'''<!doctype html>
<html lang="@@lang@@">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>@@title@@</title>
<meta name="description" content="@@desc@@">
<meta name="color-scheme" content="light dark">
<meta name="theme-color" content="#F6F7FC" media="(prefers-color-scheme: light)">
<meta name="theme-color" content="#0E1124" media="(prefers-color-scheme: dark)">
<link rel="canonical" href="@@canon@@">
<link rel="alternate" hreflang="pt-BR" href="https://caiolombello.github.io/perigauge/">
<link rel="alternate" hreflang="en" href="https://caiolombello.github.io/perigauge/en/">
<link rel="alternate" hreflang="x-default" href="https://caiolombello.github.io/perigauge/">
<meta property="og:type" content="website">
<meta property="og:site_name" content="PeriGauge">
<meta property="og:locale" content="@@oglocale@@">
<meta property="og:title" content="@@title@@">
<meta property="og:description" content="@@desc@@">
<meta property="og:image" content="https://caiolombello.github.io/perigauge/assets/perigauge-social-card.png">
<meta property="og:image:type" content="image/png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:alt" content="@@ogalt@@">
<meta name="twitter:card" content="summary_large_image">
<link rel="icon" type="image/svg+xml" href="@@root@@assets/favicon.svg">
<link rel="stylesheet" href="@@root@@style.css">
<script src="@@root@@site.js" defer></script>
</head>
<body>
<a class="skip-link" href="#main">@@skip@@</a>

<svg xmlns="http://www.w3.org/2000/svg" width="0" height="0" style="position:absolute" aria-hidden="true" focusable="false">
 <symbol id="i-ok" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M8 12.5l2.7 2.7 5.5-5.7"/></symbol>
 <symbol id="i-warn" viewBox="0 0 24 24"><path d="M12 4l9 16H3z"/><path d="M12 10v4.2M12 17.2v.1"/></symbol>
 <symbol id="i-crit" viewBox="0 0 24 24"><path d="M8.5 3h7L21 8.5v7L15.5 21h-7L3 15.5v-7z"/><path d="M12 7.5v5.5M12 16v.1"/></symbol>
 <symbol id="i-bolt" viewBox="0 0 24 24"><path d="M13 3L6 13.5h5L10 21l8-11h-5z"/></symbol>
 <symbol id="i-stale" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" stroke-dasharray="3 3"/><path d="M12 7.5V12l3 2"/></symbol>
 <symbol id="i-dev" viewBox="0 0 24 24"><path d="M9 7l-5 5 5 5M15 7l5 5-5 5"/></symbol>
 <symbol id="i-gen" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8v.1"/></symbol>
</svg>

<p class="prerelease"><span class="wrap" style="display:block">@@prerelease@@</span></p>

<header class="site-header">
 <div class="wrap header-inner">
  <a class="brand" href="@@home@@" aria-label="PeriGauge"><img src="@@root@@assets/perigauge.svg" width="32" height="32" alt=""><span>PeriGauge</span></a>
  <nav class="site-nav" aria-label="@@navlabel@@">
   <a href="#dispositivos">@@n1@@</a>
   <a href="#notificacoes">@@n2@@</a>
   <a href="#privacidade">@@n3@@</a>
   <a href="#instalar">@@n4@@</a>
   <a href="#roadmap">@@n5@@</a>
  </nav>
  <nav class="lang" aria-label="@@langlabel@@">@@langlinks@@</nav>
 </div>
</header>

<main id="main" tabindex="-1">
<section class="hero">
 <div class="wrap hero-grid">
  <div>
   <span class="badge">@@badge@@</span>
   <h1>@@h1@@</h1>
   <p class="lead">@@lead@@</p>
   <div class="actions">
    <a class="btn" href="#instalar">@@cta1@@</a>
    <a class="btn btn-ghost" href="#dispositivos">@@cta2@@</a>
    <a class="btn btn-ghost" href="https://github.com/caiolombello/perigauge">@@gh@@</a>
   </div>
  </div>
  <figure class="product-shot" aria-labelledby="shot-caption">
   <div class="shot-controls" role="group" aria-label="@@shot_styles@@" hidden>
    <button type="button" aria-pressed="true" data-shot="@@root@@assets/perigauge-popup-rings.png" data-shot-alt="@@shot_rings_alt@@">@@shot_rings@@</button>
    <button type="button" aria-pressed="false" data-shot="@@root@@assets/perigauge-popup-bars.png" data-shot-alt="@@shot_bars_alt@@">@@shot_bars@@</button>
   </div>
   <img id="product-shot" src="@@root@@assets/perigauge-popup-rings.png" alt="@@shot_rings_alt@@" width="434" height="550" fetchpriority="high" decoding="async">
   <figcaption id="shot-caption">@@shot_caption@@</figcaption>
   <noscript><p class="shot-fallback"><a href="@@root@@assets/perigauge-popup-bars.png">@@shot_bars_link@@</a></p></noscript>
  </figure>
 </div>
 <div class="wrap">
  <h2 class="sr-only">@@legendh@@</h2>
  <ul class="legend" aria-label="@@legendh@@">@@legend@@</ul>
 </div>
</section>

<section id="dispositivos" class="section-alt">
 <div class="wrap">
  <p class="eyebrow">@@e1@@</p>
  <h2>@@h_dev@@</h2>
  <p class="lead">@@dev_lead@@</p>
  <div class="table-wrap" tabindex="0" role="region" aria-label="@@tablelabel@@">
  <table>
   <caption>@@tablecap@@</caption>
   <thead><tr><th scope="col">@@c1@@</th><th scope="col">@@c2@@</th><th scope="col">@@c3@@</th><th scope="col">@@c4@@</th><th scope="col">@@c5@@</th></tr></thead>
   <tbody>
@@rows@@
   </tbody>
  </table>
  </div>
  <p class="lead" style="margin-top:16px;font-size:.95rem">@@dev_note@@ <a href="@@compat@@">COMPATIBILITY.md</a>.</p>
 </div>
</section>

<section id="notificacoes">
 <div class="wrap">
  <p class="eyebrow">@@e2@@</p>
  <h2>@@h_not@@</h2>
  <p class="lead">@@not_lead@@</p>
  <ul class="grid">
   <li class="card"><span class="thr s-warn">20%</span><h3>@@t20h@@</h3><p>@@st20@@ @@t20@@</p></li>
   <li class="card"><span class="thr s-critical">10%</span><h3>@@t10h@@</h3><p>@@st10@@ @@t10@@</p></li>
   <li class="card"><span class="thr s-critical">5%</span><h3>@@t5h@@</h3><p>@@st10@@ @@t5@@</p></li>
   <li class="card"><span class="thr s-ok">100%</span><h3>@@t100h@@</h3><p>@@stok@@ @@t100@@</p></li>
  </ul>
  <ul class="grid">
   <li class="card"><h3>@@r1h@@</h3><p>@@r1@@</p></li>
   <li class="card"><h3>@@r2h@@</h3><p>@@r2@@</p></li>
   <li class="card"><h3>@@r3h@@</h3><p>@@r3@@</p></li>
  </ul>
 </div>
</section>

<section id="privacidade" class="section-alt">
 <div class="wrap">
  <p class="eyebrow">@@e3@@</p>
  <h2>@@h_priv@@</h2>
  <p class="lead">@@priv_lead@@</p>
  <ul class="grid">
   <li class="card"><h3>@@p1h@@</h3><p>@@p1@@</p></li>
   <li class="card"><h3>@@p2h@@</h3><p>@@p2@@</p></li>
   <li class="card"><h3>@@p3h@@</h3><p>@@p3@@</p></li>
   <li class="card"><h3>@@p4h@@</h3><p>@@p4@@</p></li>
  </ul>
 </div>
</section>

<section id="instalar">
 <div class="wrap">
  <p class="eyebrow">@@e4@@</p>
  <h2>@@h_inst@@</h2>
  <p class="lead">@@inst_lead@@</p>
  <div class="code"><pre id="cmds"><code># @@inst_c1@@
curl -LO https://github.com/caiolombello/perigauge/releases/download/v0.1.0-alpha.2/perigauge-0.1.0-alpha.2-x86_64-linux.tar.gz
curl -LO https://github.com/caiolombello/perigauge/releases/download/v0.1.0-alpha.2/SHA256SUMS
sha256sum -c SHA256SUMS
tar xf perigauge-0.1.0-alpha.2-x86_64-linux.tar.gz
cd perigauge-0.1.0-alpha.2
# @@inst_c2@@
./install.sh --dry-run
./install.sh --enable-service</code></pre></div>
  <div class="copy-row">
   <button class="btn btn-ghost" type="button" hidden data-copy-target="cmds" data-copy-status="copy-status" data-copied="@@copied@@" data-failed="@@failed@@">@@copy@@</button>
   <span id="copy-status" role="status" aria-live="polite"></span>
  </div>
  <p class="lead" style="margin-top:18px">@@udev@@</p>
 </div>
</section>

<section id="roadmap" class="section-alt">
 <div class="wrap">
  <p class="eyebrow">@@e5@@</p>
  <h2>@@h_road@@</h2>
  <ul class="grid">
   <li class="card"><h3>@@m1h@@</h3><p>@@m1@@</p></li>
   <li class="card"><h3>@@m2h@@</h3><p>@@m2@@</p></li>
   <li class="card"><h3>@@m3h@@</h3><p>@@m3@@</p></li>
   <li class="card"><h3>@@m4h@@</h3><p>@@m4@@</p></li>
  </ul>
 </div>
</section>
</main>

<footer>
 <div class="wrap foot">
  <p>@@foot@@</p>
  <p><a href="https://github.com/caiolombello/perigauge">@@gh@@</a> · <a href="https://github.com/caiolombello/perigauge/releases">@@releases@@</a> · <a href="@@compat@@">COMPATIBILITY.md</a></p>
 </div>
</footer>
</body>
</html>
'''

def rows(L):
    def badge(kind, text):
        icon = {"v": "ok", "d": "dev", "g": "gen"}[kind]
        col = {"v": "s-ok", "d": "s-warn", "g": "s-stale"}[kind]
        return f'<span class="state {col}">{ic(icon)}{text}</span>'
    R = [
     ("Keychron M6", L["c_ult"], L["c_ult2"], "v", L["ev_v"], L["n_m6"]),
     ("Logitech MX Keys · MX Master 3S", L["c_lg"], L["c_lg2"], "d", L["ev_d"], L["n_lg"]),
     ("Samsung Galaxy Buds3 Pro", "Bluetooth", L["c_bud"], "v", L["ev_v"], L["n_bud"]),
     (L["gen_dev"], L["gen_conn"], L["gen_back"], "g", L["ev_g"], L["n_gen"]),
    ]
    out = []
    for name, conn, back, k, evt, note in R:
        out.append(f'    <tr><th scope="row">{name}</th><td>{conn}</td><td>{back}</td><td>{badge(k, evt)}</td><td>{note}</td></tr>')
    return "\n".join(out)

PT = dict(
 lang="pt-BR", oglocale="pt_BR", root="", home="./", compat="../COMPATIBILITY.md" if False else "COMPATIBILITY.md",
 canon="https://caiolombello.github.io/perigauge/",
 title="PeriGauge — bateria dos seus periféricos no Linux",
 desc="Monitor de bateria de mouses, teclados e fones Bluetooth para o desktop Linux. Núcleo em Rust, local, sem rede e sem telemetria. Pré-alpha 0.1.0-alpha.2.",
 ogalt="PeriGauge: bateria dos periféricos no Linux. Pré-alpha 0.1.0-alpha.2.",
 skip="Ir para o conteúdo",
 prerelease="<strong>Pré-alpha 0.1.0-alpha.2.</strong> Leituras verificadas em hardware com Keychron M6 e Galaxy Buds3 Pro; Logitech ainda depende de validação em hardware.",
 navlabel="Seções", langlabel="Idioma",
 n1="Dispositivos", n2="Notificações", n3="Privacidade", n4="Instalação", n5="Roadmap",
 badge="Pré-alpha · Linux · KDE Plasma 6 primeiro",
 h1="A bateria do mouse, do teclado e dos fones, no seu painel.",
 lead="PeriGauge mostra um medidor por dispositivo no painel, na área de trabalho e num popup, e avisa antes de acabar. Escolha gauge circular ou barras, incluindo fone esquerdo, direito e estojo. Um único binário em Rust, tudo local: sem Solaar, sem rede, sem telemetria.",
 cta1="Ver instalação (pré-alpha)", cta2="Dispositivos suportados", gh="Código no GitHub", releases="Releases",
 shot_styles="Estilo de exibição", shot_rings="Gauge", shot_bars="Barras",
 shot_rings_alt="PeriGauge em gauge: Keychron M6 41%, Galaxy Buds3 Pro esquerdo 83%, direito 80% e estojo 79%.",
 shot_bars_alt="PeriGauge em barras: Keychron M6 41%, Galaxy Buds3 Pro esquerdo 83%, direito 80% e estojo 79%.",
 shot_caption="Interface Qt/QML do PeriGauge no KDE Plasma, com leituras reais de um Keychron M6 e Galaxy Buds3 Pro. Capturado em 8 de outubro de 2026.",
 shot_bars_link="Ver captura no estilo barras",
 legendh="Como os estados aparecem",
 e1="Dispositivos", h_dev="O que funciona hoje, e com que evidência",
 dev_lead="Separamos o que foi testado em hardware do que ainda está em desenvolvimento. Nenhum dispositivo é listado como suportado sem indicar o nível de evidência.",
 tablelabel="Matriz de compatibilidade (role horizontalmente em telas estreitas)",
 tablecap="Compatibilidade por dispositivo e nível de evidência. Pré-alpha.",
 c1="Dispositivo", c2="Conexão", c3="Backend", c4="Evidência", c5="Observações",
 c_ult="Receptor Keychron Ultra-Link 8K", c_ult2="Protocolo HID de fornecedor",
 c_lg="Receptor Bolt ou Bluetooth", c_lg2="HID++ 2.0 direto (sem Solaar)",
 c_bud="Protocolo SPP da Samsung",
 ev_v="Verificado em hardware", ev_d="Em desenvolvimento", ev_g="Genérico / esperado",
 n_m6="Lido de um M6 real pelo receptor Ultra-Link 8K.",
 n_lg="Testes com fixtures; hardware pendente.",
 n_bud="Leituras e atualizações de bateria verificadas via Bluetooth em um aparelho real: fone esquerdo, direito e estojo, quando informado pelo dispositivo.",
 gen_dev="Qualquer dispositivo que o UPower/BlueZ já reporte", gen_conn="Bluetooth / USB",
 gen_back="UPower e BlueZ (power_supply do kernel, BLE Battery Service, HFP)",
 n_gen="Depende do que o dispositivo reporta; não é garantia por modelo.",
 dev_note="Detalhes completos, incluindo dados lidos, em",
 e2="Notificações", h_not="Avisa na hora certa, sem repetir.",
 not_lead="Quatro limiares, entregues pelo servidor de notificações do desktop.",
 st20=st("warn","Aviso"), st10=st("critical","Crítico"), stok=st("ok","Carregado"),
 t20h="Aviso", t20="A bateria está baixa.",
 t10h="Crítico", t10="Hora de carregar.",
 t5h="Vai desligar", t5="O dispositivo está prestes a desligar.",
 t100h="Carregado", t100="Notifica ao chegar a 100%.",
 r1h="Histerese", r1="Há histerese para o mesmo aviso não se repetir a cada leitura.",
 r2h="Nunca com leitura antiga", r2="Leitura antiga ou percentual estimado (por tensão) não dispara aviso; o estado aparece como “Leitura antiga”. Se o dispositivo só informa um nível aproximado, o aviso segue o próprio dispositivo (baixo/crítico).",
 r3h="Não perturbe", r3="Como usa o servidor de notificações do desktop, o modo “não perturbe” é respeitado.",
 e3="Privacidade", h_priv="Local por padrão, de verdade.",
 priv_lead="PeriGauge só conversa com os seus dispositivos e com o desktop.",
 p1h="Sem rede", p1="Sem uso de rede: tudo acontece na sua máquina.",
 p2h="Sem telemetria", p2="Nenhuma métrica de uso é coletada ou enviada.",
 p3h="Sem Solaar", p3="O suporte Logitech usa HID++ 2.0 direto, sem depender do Solaar.",
 p4h="Daemon de usuário", p4="Um binário único, <code>perigauge</code>, rodando como serviço <code>systemd --user</code>, sem root.",
 e4="Instalação", h_inst="Instalação (pré-alpha)",
 inst_lead="Binário para Linux x86_64 com glibc 2.39 ou superior, testado no Ubuntu 26.04 com KDE Plasma 6. Confira o checksum e simule antes de instalar; tudo vai para o seu usuário, sem sudo. Para compilar do código-fonte, veja o README.",
 inst_c1="baixar a release e conferir o checksum", inst_c2="simular; depois instalar com o serviço de notificações",
 copy="Copiar comandos", copied="Comandos copiados.", failed="Não foi possível copiar.",
 udev="O acesso HID sem root exige uma regra udev. É um passo explícito e separado, com sudo, documentado no repositório; a instalação nunca o executa sozinha.",
 e5="Roadmap", h_road="O que vem a seguir",
 m1h="0.1.0-alpha.2", m1="Medidores por dispositivo e alternância entre gauge e barras. Publicado em outubro de 2026.",
 m2h="Plasmoid KDE Plasma 6", m2="Baterias sempre à vista no painel e na área de trabalho; popup com o mesmo estilo do painel.",
 m3h="Extensão GNOME Shell", m3="Planejada para depois do plasmoid.",
 m4h="Mais dispositivos", m4="Mais periféricos conforme houver hardware para verificar. Cada um com seu nível de evidência.",
 foot="PeriGauge · pré-alpha · local-first.",
 langlinks='<a href="./" lang="pt-BR" hreflang="pt-BR" aria-current="page">Português</a><a href="en/" lang="en" hreflang="en">English</a>',
)
EN = dict(PT)
EN.update(
 lang="en", oglocale="en_US", root="../", home="./", compat="../COMPATIBILITY.md",
 canon="https://caiolombello.github.io/perigauge/en/",
 title="PeriGauge — peripheral battery on your Linux desktop",
 desc="Battery monitor for Bluetooth mice, keyboards and headphones on the Linux desktop. Rust core, fully local, no network, no telemetry. Pre-alpha 0.1.0-alpha.2.",
 ogalt="PeriGauge: peripheral battery on Linux. Pre-alpha 0.1.0-alpha.2.",
 skip="Skip to content",
 prerelease="<strong>Pre-alpha 0.1.0-alpha.2.</strong> Battery readings verified on hardware with Keychron M6 and Galaxy Buds3 Pro; Logitech still awaits hardware validation.",
 navlabel="Sections", langlabel="Language",
 n1="Devices", n2="Notifications", n3="Privacy", n4="Install", n5="Roadmap",
 badge="Pre-alpha · Linux · KDE Plasma 6 first",
 h1="Your mouse, keyboard and headphone battery, right in your panel.",
 lead="PeriGauge shows a meter for each device in the panel, on the desktop and in a popup, and warns you before they run out. Choose circular gauges or bars, including left bud, right bud and case. A single Rust binary, fully local: no Solaar, no network, no telemetry.",
 cta1="See install (pre-alpha)", cta2="Supported devices", gh="Source on GitHub", releases="Releases",
 shot_styles="Display style", shot_rings="Gauge", shot_bars="Bars",
 shot_rings_alt="PeriGauge gauges: Keychron M6 41%, Galaxy Buds3 Pro left 83%, right 80% and case 79%.",
 shot_bars_alt="PeriGauge bars: Keychron M6 41%, Galaxy Buds3 Pro left 83%, right 80% and case 79%.",
 shot_caption="PeriGauge Qt/QML interface on KDE Plasma, with real readings from a Keychron M6 and Galaxy Buds3 Pro. Captured October 8, 2026.",
 shot_bars_link="View the bar-style screenshot",
 legendh="How states are shown",
 e1="Devices", h_dev="What works today, and with what evidence",
 dev_lead="We separate what was tested on hardware from what is still in development. No device is listed as supported without its evidence level.",
 tablelabel="Compatibility matrix (scroll horizontally on narrow screens)",
 tablecap="Compatibility by device and evidence level. Pre-alpha.",
 c1="Device", c2="Connection", c3="Backend", c4="Evidence", c5="Notes",
 c_ult="Keychron Ultra-Link 8K receiver", c_ult2="Vendor HID protocol",
 c_lg="Bolt receiver or Bluetooth", c_lg2="Direct HID++ 2.0 (no Solaar)",
 c_bud="Samsung SPP protocol",
 ev_v="Verified on hardware", ev_d="In development", ev_g="Generic / expected",
 n_m6="Read from a real M6 through the Ultra-Link 8K receiver.",
 n_lg="Fixture-based tests; hardware pending.",
 n_bud="Battery readings and updates verified over Bluetooth on a real device: left bud, right bud and case, when reported by the device.",
 gen_dev="Any device UPower/BlueZ already reports", gen_conn="Bluetooth / USB",
 gen_back="UPower and BlueZ (kernel power_supply, BLE Battery Service, HFP)",
 n_gen="Depends on what the device reports; not a per-model guarantee.",
 dev_note="Full details, including data read, in",
 e2="Notifications", h_not="Warns at the right time, without nagging.",
 not_lead="Four thresholds, delivered through the desktop notification server.",
 st20=st("warn","Warning"), st10=st("critical","Critical"), stok=st("ok","Charged"),
 t20h="Warning", t20="Battery is low.",
 t10h="Critical", t10="Time to charge.",
 t5h="Shutting down", t5="The device is about to power off.",
 t100h="Charged", t100="Notifies when it reaches 100%.",
 r1h="Hysteresis", r1="Hysteresis keeps the same alert from repeating on every reading.",
 r2h="Never on a stale reading", r2="A stale reading or a voltage-estimated percentage never alerts; the state shows as “Stale reading”. When a device only reports an approximate level, the alert follows the device itself (low/critical).",
 r3h="Do not disturb", r3="It uses the desktop notification server, so “do not disturb” mode is respected.",
 e3="Privacy", h_priv="Local by default, for real.",
 priv_lead="PeriGauge only talks to your devices and to your desktop.",
 p1h="No network", p1="No network use: everything happens on your machine.",
 p2h="No telemetry", p2="No usage metrics are collected or sent.",
 p3h="No Solaar", p3="Logitech support uses direct HID++ 2.0, with no Solaar dependency.",
 p4h="User daemon", p4="One binary, <code>perigauge</code>, running as a <code>systemd --user</code> service, no root.",
 e4="Install", h_inst="Install (pre-alpha)",
 inst_lead="Binary for Linux x86_64 with glibc 2.39 or newer, tested on Ubuntu 26.04 with KDE Plasma 6. Check the checksum and dry-run before installing; everything goes to your user, no sudo. To build from source, see the README.",
 inst_c1="download the release and check the checksum", inst_c2="dry-run; then install with the notification service",
 copy="Copy commands", copied="Commands copied.", failed="Could not copy.",
 udev="Root-less HID access needs a udev rule. It is an explicit, separate step that uses sudo, documented in the repository; the installer never runs it on its own.",
 e5="Roadmap", h_road="What comes next",
 m1h="0.1.0-alpha.2", m1="Per-device meters and switching between gauges and bars. Published October 2026.",
 m2h="KDE Plasma 6 plasmoid", m2="Always-visible batteries in the panel and on the desktop; popup follows the panel style.",
 m3h="GNOME Shell extension", m3="Planned for after the plasmoid.",
 m4h="More devices", m4="More peripherals as hardware is available to verify. Each with its own evidence level.",
 foot="PeriGauge · pre-alpha · local-first.",
 langlinks='<a href="../" lang="pt-BR" hreflang="pt-BR">Português</a><a href="./" lang="en" hreflang="en" aria-current="page">English</a>',
)
for L, path in ((PT, DOCS/"index.html"), (EN, DOCS/"en"/"index.html")):
    L = dict(L)
    pt = L["lang"] == "pt-BR"
    L["legend"] = "".join(f"<li>{st(k, t)}</li>" for k, t in (
        ("ok", "Bom" if pt else "Good"), ("warn", "Baixa" if pt else "Low"), ("critical", "Crítica" if pt else "Critical"),
        ("charging", "Carregando" if pt else "Charging"), ("stale", "Leitura antiga" if pt else "Stale reading")))
    L["rows"] = rows(L)
    if not pt: L["compat"] = "../COMPATIBILITY.md"
    html = TPL
    for k, v in L.items():
        html = html.replace(f"@@{k}@@", v)
    assert "@@" not in html, [w for w in html.split("@@")[1::2]]
    path.write_text(html, encoding="utf-8")
print("written")
