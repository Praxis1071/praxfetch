mod ascii;
mod info;
mod tux_3d;

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

    #[arg(
        long = "3d",
        help = "Open the interactive 3D Tux view (requires Ratty terminal)"
    )]
    tux_3d: bool,
}

fn main() {
    let args = Args::parse();

    if args.tux_3d {
        if let Err(error) = tux_3d::run() {
            eprintln!("praxfetch: 3D Tux view failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    let no_color = args.no_color || std::env::var_os("NO_COLOR").is_some();

    if no_color {
        colored::control::set_override(false);
    }

    let s = info::collect();
    let logo = ascii::logo(&s.os);

    let host = if !s.host.product.is_empty() {
        s.host.product.clone()
    } else if !s.host.vendor.is_empty() {
        s.host.vendor.clone()
    } else {
        "unknown".to_owned()
    };

    let packages = s
        .packages
        .count
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unknown".to_owned());

    let fields = [
        ("", format!("{}@{}", s.user, s.hostname)),
        ("os", info::os_label(&s.os)),
        ("host", host),
        ("kernel", s.kernel),
        (
            "uptime",
            s.uptime
                .map(info::uptime)
                .unwrap_or_else(|| "unknown".to_owned()),
        ),
        ("pkgs", packages),
        ("memory", info::memory_compact(&s.memory)),
    ];

    let label_width = fields
        .iter()
        .map(|(key, _)| key.chars().count())
        .max()
        .unwrap_or(0);

    let logo_width = ascii::logo_width(&logo);
    let logo_column_width = logo_width + 4;

    for i in 0..logo.len().max(fields.len()) {
        let left = logo.get(i).map(String::as_str).unwrap_or("");
        let right = fields
            .get(i)
            .map(|(key, value)| {
                if key.is_empty() {
                    value.green().bold().to_string()
                } else {
                    let key = format!("{:<width$}", key, width = label_width);
                    format!("{} {}", key.cyan().bold(), value)
                }
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
}
