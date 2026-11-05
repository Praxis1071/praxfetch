mod ascii;
mod info;

use clap::Parser;
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "praxfetch",
    version,
    about = "Fast zero-subprocess Linux system information fetch"
)]
struct Args {
    #[arg(long, help = "Disable ANSI colors")]
    no_color: bool,
}

fn main() {
    let args = Args::parse();
    let no_color = args.no_color || std::env::var_os("NO_COLOR").is_some();

    if no_color {
        colored::control::set_override(false);
    }

    let s = info::collect();
    let logo = ascii::logo(&s.os);

    let host = if !s.host.vendor.is_empty() && !s.host.product.is_empty() {
        format!("{} {}", s.host.vendor, s.host.product)
    } else if !s.host.product.is_empty() {
        s.host.product.clone()
    } else if !s.host.vendor.is_empty() {
        s.host.vendor.clone()
    } else {
        "unknown".to_owned()
    };

    let os = info::os_label(&s.os);
    let packages = match s.packages.count {
        Some(count) => format!("{} ({})", count, s.packages.manager),
        None => format!("{} (count unavailable)", s.packages.manager),
    };

    let load = s
        .load_avg
        .map(|(one, five, fifteen)| format!("{one:.2} {five:.2} {fifteen:.2}"))
        .unwrap_or_else(|| "unknown".to_owned());

    let fields = [
        ("OS", os),
        ("Host", host),
        ("Kernel", s.kernel),
        ("Uptime", info::uptime(s.uptime)),
        ("Load Avg", load),
        ("Packages", packages),
        ("Shell", info::shell(&s.shell, &s.shell_version)),
        ("DE/WM", s.desktop),
        ("Terminal", s.terminal),
        ("CPU", {
            let topology = match s.cpu.physical_cores {
                Some(cores) if s.cpu.threads > 0 => {
                    format!("{cores} cores / {} threads", s.cpu.threads)
                }
                Some(cores) => format!("{cores} cores"),
                None if s.cpu.threads > 0 => format!("{} threads", s.cpu.threads),
                None => "topology unavailable".to_owned(),
            };
            format!("{} ({topology})", s.cpu.model)
        }),
        (
            "CPU Temp",
            s.temperature
                .cpu_celsius
                .map(|temperature| format!("{temperature:.1} °C"))
                .unwrap_or_else(|| "unavailable".to_owned()),
        ),
        (
            "CPU Freq",
            s.cpu
                .frequency_mhz
                .map(|frequency| format!("{:.2} GHz", frequency as f64 / 1000.0))
                .unwrap_or_else(|| "unavailable".to_owned()),
        ),
        ("Memory", info::memory(&s.memory)),
        ("Swap", info::swap(&s.swap)),
        ("Disk", {
            let summary = info::disk(&s.disk);
            if s.disk.filesystems > 1 && summary != "unknown" {
                format!("{summary} ({} filesystems)", s.disk.filesystems)
            } else {
                summary
            }
        }),
        ("Battery", info::battery(&s.battery)),
        ("GPU", info::gpu(&s.gpu)),
        ("Display", info::display(&s.display)),
        ("Network", info::network(&s.network)),
    ];

    let label_width = fields
        .iter()
        .map(|(key, _)| key.chars().count())
        .max()
        .unwrap_or(0);

    let logo_width = ascii::logo_width(&logo);
    let logo_column_width = logo_width + 4;

    println!();
    println!("{}", format!("{}@{}", s.user, s.hostname).green().bold());

    for i in 0..logo.len().max(fields.len()) {
        let left = logo.get(i).map(String::as_str).unwrap_or("");
        let right = fields
            .get(i)
            .map(|(key, value)| {
                let key = format!("{:<width$}", key, width = label_width);
                format!("{} {}", key.cyan().bold(), value)
            })
            .unwrap_or_default();

        let logo_part = if left.is_empty() {
            " ".repeat(logo_column_width)
        } else {
            let padded = ascii::pad_logo_line(left, logo_column_width);
            ascii::colorize_logo_line(&padded, &s.os)
        };

        println!("{}{}", logo_part, right);
    }

    println!();
    println!(
        "{}{}",
        " ".repeat(logo_column_width),
        ascii::blocks(no_color)
    );
    println!();
}
