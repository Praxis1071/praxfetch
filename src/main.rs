mod ascii;
mod info;

use clap::Parser;
use colored::Colorize;

#[derive(Parser)]
#[command(name="praxfetch", version, about="Fast zero-subprocess Linux system information fetch")]
struct Args {
    #[arg(long, help="Disable ANSI colors")]
    no_color: bool,
}

fn main() {
    let args = Args::parse();
    if args.no_color { colored::control::set_override(false); }

    let s = info::collect();
    let logo = ascii::logo(&s.os);
    let host = if s.host.vendor.is_empty() { s.host.product } else { format!("{} {}", s.host.vendor, s.host.product) };
    let fields = vec![
        ("OS", s.os.name),
        ("Host", host),
        ("Kernel", s.kernel),
        ("Uptime", info::uptime(s.uptime)),
        ("Packages", if s.packages.count > 0 { format!("{} ({})", s.packages.count, s.packages.manager) } else { s.packages.manager }),
        ("Shell", info::shell(&s.shell, &s.shell_version)),
        ("DE/WM", s.desktop),
        ("Terminal", s.terminal),
        ("CPU", format!("{} ({} cores)", s.cpu.model, s.cpu.cores)),
        ("Memory", info::memory(&s.memory)),
    ];

    println!();
    println!("{}", format!("{}@{}", s.user, s.hostname).green().bold());
    for i in 0..logo.len().max(fields.len()) {
        let left = logo.get(i).map(String::as_str).unwrap_or("");
        let right = fields.get(i).map(|(k,v)| format!("{} {}", format!("{k:<11}").cyan().bold(), v)).unwrap_or_default();
        println!("{:<24}  {}", left, right);
    }
    println!();
    println!("                 {}", ascii::blocks());
    println!();
}
