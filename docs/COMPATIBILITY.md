# Compatibilidade do PeriGauge / Compatibility

> **Pré-alpha 0.1.0-alpha.2.** Esta matriz descreve o estado do desenvolvimento, não uma promessa de suporte.
> *Pre-alpha 0.1.0-alpha.2. This matrix describes development status, not a support promise.*

## Níveis de evidência / Evidence levels

| Nível | Significado |
|---|---|
| **Verificado em hardware** (*Verified on hardware*) | Lido de um dispositivo real. |
| **Em desenvolvimento** (*In development*) | Implementação em andamento; testes com fixtures, hardware ainda pendente. |
| **Genérico / esperado** (*Generic / expected*) | Deve funcionar se o UPower/BlueZ já reportar a bateria; depende do dispositivo. |

## Matriz / Matrix

| Dispositivo (*Device*) | Conexão (*Connection*) | Backend | Dados lidos (*Data read*) | Evidência (*Evidence*) | Observações (*Notes*) |
|---|---|---|---|---|---|
| Keychron M6 | Receptor Keychron Ultra-Link 8K | Protocolo HID de fornecedor | Nível de bateria | **Verificado em hardware** | — |
| Logitech MX Keys | Receptor Bolt | HID++ 2.0 direto (sem Solaar) | Nível de bateria | Em desenvolvimento | Testes com fixtures; hardware pendente. |
| Logitech MX Keys | Bluetooth | HID++ 2.0 direto (sem Solaar) | Nível de bateria | Em desenvolvimento | Testes com fixtures; hardware pendente. |
| Logitech MX Master 3S | Receptor Bolt | HID++ 2.0 direto (sem Solaar) | Nível de bateria | Em desenvolvimento | Testes com fixtures; hardware pendente. |
| Logitech MX Master 3S | Bluetooth | HID++ 2.0 direto (sem Solaar) | Nível de bateria | Em desenvolvimento | Testes com fixtures; hardware pendente. |
| Samsung Galaxy Buds3 Pro | Bluetooth | Protocolo SPP da Samsung | Bateria do fone esquerdo, do direito e do estojo, quando informado | **Verificado em hardware** | Leituras e atualizações reais via backend `galaxy-buds`; verificado no KDE Plasma 6.6 / Ubuntu 26.04 em 08/10/2026. |
| Qualquer dispositivo que o UPower/BlueZ já reporte | Bluetooth / USB | UPower e BlueZ: `power_supply` do kernel, BLE Battery Service, HFP | Nível reportado pelo dispositivo | Genérico / esperado | Depende do dispositivo; não é garantia por modelo. |

## Notificações / Notifications

Aviso em 20%, crítico em 10%, "vai desligar" em 5% e "carregado" em 100%; histerese para não repetir; nunca notifica com leitura antiga ou percentual estimado por tensão (nível aproximado informado pelo próprio dispositivo segue o dispositivo); o servidor de notificações do desktop respeita o modo "não perturbe".
*Warning at 20%, critical at 10%, "about to shut down" at 5%, "charged" at 100%; hysteresis to avoid repeats; never notifies on a stale reading or a voltage-estimated percentage (an approximate level reported by the device follows the device); the desktop notification server honors do-not-disturb.*

## Frontends

Principal: plasmoid do KDE Plasma 6. Extensão GNOME Shell: planejada para depois. / *Main: KDE Plasma 6 plasmoid. GNOME Shell extension: planned later.*
