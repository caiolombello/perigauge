use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use perigauge::backends::{galaxy_buds, hidpp, keychron};
use perigauge::collect::{self, Collector};
use perigauge::config::{self, Config};
use perigauge::model::{Charging, Device, Kind, Snapshot, now};
use perigauge::{hidraw, notify};

const HELP: &str = "\
PeriGauge — bateria de periféricos sem fio / wireless peripheral batteries

Uso / usage:
  perigauge status [--json] [--refresh]   níveis atuais / current levels
  perigauge daemon                        serviço de usuário com notificações
  perigauge doctor                        diagnóstico de acesso e backends
  perigauge notify-test                   envia uma notificação de teste
  perigauge --version
";

fn t(pt: &'static str, en: &'static str) -> &'static str {
    if notify::portuguese() { pt } else { en }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("status");
    let flags = if args.is_empty() { &[][..] } else { &args[1..] };
    match command {
        "status" => status(flags),
        "daemon" => daemon(),
        "doctor" => doctor(),
        "notify-test" => notify_test(),
        "--version" | "-V" | "version" => {
            println!("perigauge {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "--help" | "-h" | "help" => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("perigauge: {}: {other}\n\n{HELP}", t("comando desconhecido", "unknown command"));
            ExitCode::from(2)
        }
    }
}

fn load_config() -> Config {
    let (config, warnings) = Config::load();
    for warning in warnings {
        eprintln!("perigauge: {warning}");
    }
    config
}

fn status(flags: &[String]) -> ExitCode {
    let (mut json, mut refresh) = (false, false);
    for flag in flags {
        match flag.as_str() {
            "--json" => json = true,
            "--refresh" => refresh = true,
            other => {
                eprintln!("perigauge status: {}: {other}", t("opção desconhecida", "unknown option"));
                return ExitCode::from(2);
            }
        }
    }
    let config = load_config();
    let previous = collect::load_snapshot();
    let max_age = config.interval_seconds as f64 + 15.0;
    let snapshot = match previous {
        Some(snapshot) if !refresh && now() - snapshot.generated_at < max_age => snapshot,
        // Re-checked under the lock: another process may have just collected.
        _ => Collector::new().refresh(&config, "oneshot", (!refresh).then_some(max_age)),
    };
    if json {
        match serde_json::to_string(&snapshot) {
            Ok(text) => println!("{text}"),
            Err(error) => {
                eprintln!("perigauge: {error}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        print_table(&snapshot);
    }
    ExitCode::SUCCESS
}

fn ago(timestamp: Option<f64>) -> String {
    let Some(timestamp) = timestamp else { return "—".into() };
    let seconds = (now() - timestamp).max(0.0) as u64;
    match seconds {
        0..60 => t("agora", "now").into(),
        60..3600 => format!("{} min", seconds / 60),
        3600..86400 => format!("{} h", seconds / 3600),
        _ => format!("{} d", seconds / 86400),
    }
}

fn kind_label(kind: Kind) -> &'static str {
    match kind {
        Kind::Mouse => "mouse",
        Kind::Keyboard => t("teclado", "keyboard"),
        Kind::Earbuds => t("fones", "earbuds"),
        Kind::Headset => "headset",
        Kind::Touchpad => "touchpad",
        Kind::Gamepad => "gamepad",
        Kind::Other => t("outro", "other"),
    }
}

fn battery_text(device: &Device) -> String {
    let main = match (device.battery.percent, device.battery.level) {
        (Some(percent), _) if device.battery.estimated => format!("~{percent}%"),
        (Some(percent), _) => format!("{percent}%"),
        (None, Some(level)) => format!("{level:?}").to_lowercase(),
        (None, None) => "—".into(),
    };
    let charging = match device.battery.charging {
        Charging::Charging => " ⚡",
        Charging::Full => " ✓",
        _ => "",
    };
    let parts: Vec<String> = device
        .components
        .iter()
        .filter_map(|c| {
            let label = match c.id.as_str() {
                "left" => t("E", "L"),
                "right" => t("D", "R"),
                "case" => t("estojo", "case"),
                _ => return None,
            };
            c.percent.map(|p| format!("{label} {p}%"))
        })
        .collect();
    if parts.is_empty() { format!("{main}{charging}") } else { format!("{main}{charging} ({})", parts.join(" · ")) }
}

fn print_table(snapshot: &Snapshot) {
    let visible: Vec<&Device> = snapshot.devices.iter().filter(|d| !d.hidden).collect();
    if visible.is_empty() {
        println!("{}", t("Nenhum periférico com bateria encontrado.", "No battery-powered peripheral found."));
    }
    for device in visible {
        let state = if !device.present {
            t("fora de alcance", "out of range").to_string()
        } else if let Some(error) = &device.error {
            error.clone()
        } else if device.stale {
            t("última leitura", "last reading").to_string()
        } else {
            "ok".to_string()
        };
        println!(
            "{:<28} {:<9} {:<34} {:<16} {}",
            device.name,
            kind_label(device.kind),
            battery_text(device),
            state,
            ago(device.updated_at)
        );
    }
    for issue in &snapshot.issues {
        println!("! {}", issue.detail);
    }
}

/// Keeps a single daemon per user session.
fn daemon_lock() -> std::io::Result<std::fs::File> {
    let dir = config::state_dir();
    config::ensure_private_dir(&dir)?;
    let file = OpenOptions::new().create(true).truncate(false).write(true).mode(0o600).open(dir.join("daemon.lock"))?;
    // SAFETY: flock on a valid, owned descriptor; held for the process lifetime.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(file)
}

fn daemon() -> ExitCode {
    let _lock = match daemon_lock() {
        Ok(lock) => lock,
        Err(error) => {
            eprintln!(
                "perigauge: {} ({error})",
                t("outro daemon já está em execução", "another daemon is already running")
            );
            return ExitCode::FAILURE;
        }
    };
    let mut config = load_config();
    let mut collector = Collector::new();
    if config.backends.galaxy_buds {
        match galaxy_buds::Monitor::start() {
            Ok(monitor) => collector.buds = Some(monitor),
            Err(error) => eprintln!("perigauge: Galaxy Buds: {error}"),
        }
    }
    let mut session: Option<zbus::blocking::Connection> = None;
    let mut memory = collect::load_notify_memory();
    eprintln!("perigauge {} daemon: {}s", env!("CARGO_PKG_VERSION"), config.interval_seconds);
    loop {
        let (latest, warnings) = Config::load();
        if latest != config {
            for warning in warnings {
                eprintln!("perigauge: {warning}");
            }
            config = latest;
        }
        let snapshot = collector.refresh(&config, "daemon", None);
        if session.is_none() {
            // (Re)connect lazily; a broken connection is dropped after a failure.
            session = perigauge::bus::session().map_err(|error| eprintln!("perigauge: session bus: {error}")).ok();
        }
        let before = memory.clone();
        let mut broken = false;
        let sent = notify::process(&config, &snapshot.devices, &mut memory, |message, replaces| {
            let connection = session.as_ref().ok_or_else(|| "no session bus".to_string())?;
            notify::send(connection, message, replaces).map_err(|error| {
                broken = true;
                error.to_string()
            })
        });
        if broken {
            session = None;
        }
        for (id, alert) in sent {
            eprintln!("perigauge: {id}: {alert:?}");
        }
        if memory != before {
            collect::save_notify_memory(&memory);
        }
        thread::sleep(Duration::from_secs(config.interval_seconds));
    }
}

fn notify_test() -> ExitCode {
    let connection = match perigauge::bus::session() {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("perigauge: {error}");
            return ExitCode::FAILURE;
        }
    };
    let message = notify::Message {
        summary: t("PeriGauge: notificação de teste", "PeriGauge: test notification").into(),
        body: t("Os avisos de bateria aparecerão assim.", "Battery alerts will look like this.").into(),
        icon: "battery",
        urgency: 1,
    };
    match notify::send(&connection, &message, 0) {
        Ok(id) => {
            println!("ok (id {id})");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("perigauge: {error}");
            ExitCode::FAILURE
        }
    }
}

fn check(ok: bool, text: String) {
    println!("{}{text}", if ok { "[ok]  " } else { "[!!]  " });
}

fn info(text: String) {
    println!("[--]  {text}");
}

fn doctor() -> ExitCode {
    println!("perigauge {}", env!("CARGO_PKG_VERSION"));
    let (config, warnings) = Config::load();
    check(
        warnings.is_empty(),
        format!("config: {} {}", config::config_dir().join("config.toml").display(), warnings.join("; ")),
    );

    let nodes = hidraw::enumerate();
    let accessible = |node: &Path| hidraw::Hidraw::open(node).map(|_| ()).map_err(|e| hidraw::error_code(&e));
    match nodes.iter().find(|n| keychron::is_vendor_interface(n)) {
        Some(node) => match accessible(&node.node) {
            Ok(()) => check(true, format!("Keychron Ultra-Link 8K: {}", node.node.display())),
            Err(code) => check(false, format!("Keychron Ultra-Link 8K: {} ({code})", node.node.display())),
        },
        None => info(t("Keychron Ultra-Link 8K: não conectado", "Keychron Ultra-Link 8K: not connected").into()),
    }
    let logitech: Vec<_> = nodes.iter().filter(|n| hidpp::is_hidpp_interface(n)).collect();
    if logitech.is_empty() {
        info(
            t(
                "Logitech HID++: nenhum receptor ou dispositivo Bluetooth",
                "Logitech HID++: no receiver or Bluetooth device",
            )
            .into(),
        );
    }
    for node in logitech {
        let label = hidpp::receiver_name(node.product).map(str::to_string).unwrap_or_else(|| node.name.clone());
        match accessible(&node.node) {
            Ok(()) => check(true, format!("Logitech {label}: {}", node.node.display())),
            Err(code) => check(false, format!("Logitech {label}: {} ({code})", node.node.display())),
        }
    }
    let rule = ["/etc/udev/rules.d/70-perigauge.rules", "/usr/lib/udev/rules.d/70-perigauge.rules"]
        .iter()
        .any(|p| Path::new(p).exists());
    check(
        rule,
        format!(
            "udev: 70-perigauge.rules {}",
            if rule { t("instalada", "installed") } else { t("ausente", "missing") }
        ),
    );
    if Path::new("/usr/lib/udev/rules.d/60-solaar.rules").exists()
        || Path::new("/etc/udev/rules.d/60-solaar.rules").exists()
    {
        info(
            t(
                "regra udev do Solaar presente (não é necessária pelo PeriGauge)",
                "Solaar udev rule present (not required by PeriGauge)",
            )
            .into(),
        );
    }

    match perigauge::backends::upower::collect() {
        Ok(devices) => check(true, format!("UPower: {} {}", devices.len(), t("periférico(s)", "peripheral(s)"))),
        Err(error) => check(false, format!("UPower: {error}")),
    }
    match perigauge::bus::session().and_then(|c| {
        c.call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "GetServerInformation",
            &(),
        )
        .and_then(|reply| reply.body().deserialize::<(String, String, String, String)>())
    }) {
        Ok((name, vendor, version, _)) => {
            check(true, format!("{}: {name} ({vendor} {version})", t("notificações", "notifications")))
        }
        Err(error) => check(false, format!("{}: {error}", t("notificações", "notifications"))),
    }
    match collect::load_snapshot() {
        Some(snapshot) => {
            let age = now() - snapshot.generated_at;
            let alive = snapshot.source == "daemon" && age < 3.0 * config.interval_seconds as f64;
            check(
                alive,
                format!(
                    "daemon: {} ({} {})",
                    if alive { t("ativo", "running") } else { t("inativo", "not running") },
                    t("última coleta há", "last collection"),
                    ago(Some(snapshot.generated_at))
                ),
            );
        }
        None => check(false, format!("daemon: {}", t("nenhuma coleta registrada", "no collection recorded"))),
    }
    ExitCode::SUCCESS
}
