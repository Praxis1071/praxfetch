use colored::Colorize;

use crate::info::OsInfo;

pub fn logo(_os: &OsInfo) -> Vec<String> {
    [
        "       .--.",
        "      |o_o |",
        "      |:_/ |",
        "     //   \\ \\",
        "    (|     | )",
        "   /'\\_   _/\\",
        "   \\___)=(___/",
    ]
    .iter()
    .map(|line| (*line).to_owned())
    .collect()
}

/// Return the terminal-column width of the built-in ASCII logo.
pub fn logo_width(lines: &[String]) -> usize {
    lines
        .iter()
        .map(|line| visible_width(line))
        .max()
        .unwrap_or(0)
}

/// Pad a logo line to an exact terminal-column width.
pub fn pad_logo_line(line: &str, width: usize) -> String {
    let current = visible_width(line);
    if current >= width {
        return line.to_owned();
    }
    format!("{}{}", line, " ".repeat(width - current))
}

fn visible_width(value: &str) -> usize {
    let chars: Vec<char> = value.chars().collect();
    let mut width = 0usize;
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];

        if ch == '\u{1b}' && index + 1 < chars.len() && chars[index + 1] == '[' {
            index += 2;
            while index < chars.len() {
                let next = chars[index];
                if next.is_ascii_alphabetic() {
                    index += 1;
                    break;
                }
                index += 1;
            }
            continue;
        }

        width += 1;
        index += 1;
    }

    width
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_distribution_uses_the_same_tux_logo() {
        let distributions = ["cachyos", "arch", "ubuntu", "debian", "fedora", "nixos"];
        let expected = logo(&OsInfo::default());

        for id in distributions {
            let os = OsInfo {
                id: id.to_owned(),
                ..Default::default()
            };
            assert_eq!(logo(&os), expected);
        }
        assert!(expected.iter().all(|line| line == line.trim_end()));
        assert!(expected.iter().any(|line| line.contains("o_o")));
    }

    #[test]
    fn logo_width_uses_visible_columns() {
        let lines = vec!["abc".to_owned(), "\x1b[31m123456\x1b[0m".to_owned()];
        assert_eq!(logo_width(&lines), 6);
    }

    #[test]
    fn logo_padding_is_exact() {
        assert_eq!(pad_logo_line("abc", 6), "abc   ");
        assert_eq!(pad_logo_line("abcdef", 3), "abcdef");
        let ansi = "\x1b[31mA\x1b[0m";
        assert_eq!(visible_width(&pad_logo_line(ansi, 3)), 3);
    }
}
