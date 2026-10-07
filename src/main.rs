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
    if args.no_color {
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
    let packages = if s.packages.count > 0 {
        format!("{} ({})", s.packages.count, s.packages.manager)
    } else {
        s.packages.manager.clone()
    };

    let fields = vec![
        ("OS", os),
        ("Host", host),
        ("Kernel", s.kernel),
        ("Uptime", info::uptime(s.uptime)),
        ("Packages", packages),
        ("Shell", info::shell(&s.shell, &s.shell_version)),
        ("DE/WM", s.desktop),
        ("Terminal", s.terminal),
        ("CPU", format!("{} ({} cores)", s.cpu.model, s.cpu.cores)),
        ("Memory", info::memory(&s.memory)),
    ];

    let label_width = fields
        .iter()
        .map(|(key, _)| key.chars().count())
        .max()
        .unwrap_or(0);

    let logo_width = logo
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);

    let logo_column_width = logo_width + 4;

    println!();
    println!("{}", format!("{}@{}", s.user, s.hostname).green().bold());

    for i in 0..logo.len().max(fields.len()) {
        let left = logo.get(i).map(String::as_str).unwrap_or("");
        let left_padded = format!(
            "{left:<logo_column_width$}",
            logo_column_width = logo_column_width
        );

        let right = fields
            .get(i)
            .map(|(key, value)| {
                let key = format!("{key:<label_width$}", label_width = label_width);
                format!("{} {}", key.cyan().bold(), value)
            })
            .unwrap_or_default();

        println!("{}{}", ascii::colorize_logo_line(&left_padded, &s.os), right);
    }

    println!();
    println!(
        "{}{}",
        " ".repeat(logo_column_width),
        ascii::blocks(args.no_color)
    );
    println!();
}
