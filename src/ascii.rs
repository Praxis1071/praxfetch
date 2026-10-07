use colored::Colorize;

use crate::info::OsInfo;

pub fn logo(os: &OsInfo) -> Vec<String> {
    let id = os.id.to_lowercase();
    let like = os.id_like.to_lowercase();

    let lines = match distro_family(&id, &like) {
        "cachyos" => vec![
            "      .----.",
            "     ╱      ╲",
            "    ╱  .--.  ╲",
            "   ╱  ╱    ╲  ╲",
            "  ╱__╱      ╲__╲",
            "      CachyOS",
        ],
        "arch" => vec![
            "       ╱╲",
            "      ╱  ╲",
            "     ╱╲   ╲",
            "    ╱      ╲",
            "   ╱   ,,   ╲",
            "  ╱   |  |   ╲",
            " ╱_-''    ''-_╲",
        ],
        "ubuntu" => vec![
            "      .-::::-.",
            "   .::::::::::::.",
            "  ::::::::::::::::",
            " :::::::' . '::::::",
            " ::::::  : :  ::::::",
            "  ::::::::::::::::::",
            "    '::::::::::::'",
        ],
        "debian" => vec![
            "      _____",
            "     ╱     ╲",
            "    ╱  .-.  ╲",
            "   ╱  (   )  ╲",
            "  ╱    -'-    ╲",
            " ╱______________╲",
        ],
        "fedora" => vec![
            "     __________",
            "   ╱            ╲",
            "  ╱   _______    ╲",
            " |   ╱       ╲   |",
            " |  |    F    |  |",
            "  ╲  ╲_______╱  ╱",
            "   ╲____________╱",
        ],
        "nixos" => vec![
            "      ╲  ╱╱",
            "       ╲╱╱",
            "    ____╲╱╱____",
            "   ╱    ╱╱╲    ╲",
            "  ╱____╱╱__╲____╲",
            "      N I X O S",
        ],
        "gentoo" => vec![
            "      _-----_",
            "     ╱       ╲",
            "    ╱  ╱---╲  ╲",
            "   |  |     |  |",
            "    ╲  ╲---╱  ╱",
            "     ╲_____/ ",
        ],
        "alpine" => vec![
            "       ╱╲",
            "      ╱  ╲",
            "     ╱ ╱╲ ╲",
            "    ╱ ╱  ╲ ╲",
            "   ╱_╱____╲_╲",
            "      ALPINE",
        ],
        "void" => vec![
            "      ______",
            "     ╱      ╲",
            "    ╱  ╱╲    ╲",
            "   ╱__╱  ╲____╲",
            "      VOID LINUX",
        ],
        _ => vec![
            "       .--.",
            "      |o_o |",
            "      |:_/ |",
            "     ╱╱   ╲ ╲",
            "    (|     | )",
            "   ╱╲_   _╱╲",
            "   ╲___)=(___╱",
        ],
    };

    lines.into_iter().map(str::to_owned).collect()
}

fn distro_family<'a>(id: &'a str, like: &'a str) -> &'a str {
    if id == "cachyos" {
        "cachyos"
    } else if id == "arch" || like.split_whitespace().any(|v| v == "arch") {
        "arch"
    } else if id == "ubuntu" || like.split_whitespace().any(|v| v == "ubuntu") {
        "ubuntu"
    } else if id == "debian" || like.split_whitespace().any(|v| v == "debian") {
        "debian"
    } else if id == "fedora" || like.split_whitespace().any(|v| v == "fedora") {
        "fedora"
    } else if id == "nixos" || like.split_whitespace().any(|v| v == "nixos") {
        "nixos"
    } else if id == "gentoo" || like.split_whitespace().any(|v| v == "gentoo") {
        "gentoo"
    } else if id == "alpine" || like.split_whitespace().any(|v| v == "alpine") {
        "alpine"
    } else if id == "void" || like.split_whitespace().any(|v| v == "void") {
        "void"
    } else {
        "generic"
    }
}

pub fn colorize_logo_line(line: &str, os: &OsInfo) -> String {
    let id = os.id.to_lowercase();
    let like = os.id_like.to_lowercase();

    match distro_family(&id, &like) {
        "cachyos" | "alpine" => line.bright_cyan().bold().to_string(),
        "arch" => line.cyan().bold().to_string(),
        "ubuntu" => line.red().bold().to_string(),
        "debian" => line.bright_red().bold().to_string(),
        "fedora" => line.bright_blue().bold().to_string(),
        "nixos" => line.blue().bold().to_string(),
        "gentoo" => line.magenta().bold().to_string(),
        "void" => line.green().bold().to_string(),
        _ => line.yellow().bold().to_string(),
    }
}

pub fn blocks(no_color: bool) -> String {
    if no_color {
        return "████████████████████████".to_owned();
    }

    let mut out = String::new();
    for c in 40..48 {
        out.push_str(&format!("\x1b[{c}m███"));
    }
    for c in 100..108 {
        out.push_str(&format!("\x1b[{c}m███"));
    }
    out.push_str("\x1b[0m");
    out
}