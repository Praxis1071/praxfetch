mod ascii;
mod info;
mod output;
mod tux_3d;

use clap::Parser;
use output::OutputLayout;

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
        help = "Replace the ASCII Tux with an interactive inline 3D Tux (Ratty terminal required)"
    )]
    tux_3d: bool,
}

fn main() {
    let args = Args::parse();
    let no_color = args.no_color || std::env::var_os("NO_COLOR").is_some();

    if no_color {
        colored::control::set_override(false);
    }

    let system = info::collect();
    let output = OutputLayout::new(&system);

    if args.tux_3d {
        if let Err(error) = tux_3d::run(&output) {
            eprintln!("praxfetch: inline 3D Tux failed: {error}");
            std::process::exit(1);
        }
    } else if let Err(error) = output.print_classic() {
        eprintln!("praxfetch: could not write output: {error}");
        std::process::exit(1);
    }
}
