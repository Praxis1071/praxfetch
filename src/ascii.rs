use colored::Colorize;
use crate::info::OsInfo;

pub fn logo(os:&OsInfo)->Vec<String> {
    let id=os.id.to_lowercase(); let like=os.id_like.to_lowercase(); let has=|x:&str| id==x || like.split_whitespace().any(|v|v==x);
    let lines=if id=="cachyos" { vec!["      .----.","     /      \","    /  .--.  \","   /  /    \\  \","  /__/      \\__\\","      CachyOS"]
    } else if has("arch") { vec!["       /\\","      /  \","     /\\   \","    /      \","   /   ,,   \","  /   |  |   \"," /_-''    ''-_\\"]
    } else if has("ubuntu") { vec!["      .-::::-.","   .::::::::::::.","  ::::::::::::::::"," :::::::' . '::::::"," ::::::  : :  ::::::","  ::::::::::::::::::","    '::::::::::::'"]
    } else if has("debian") { vec!["      _____","     /     \","    /  .-.  \","   /  (   )  \","  /    `-'    \"," /______________\\"]
    } else if has("fedora") { vec!["     __________","   /            \","  /   _______    \"," |   /       \\   |"," |  |    F    |  |","  \\  \\_______/  /","   \\____________/"]
    } else if has("nixos") { vec!["      \\  //","       \\//","    ____\\//____","   /    //\\    \","  /____//__\\____\\","      N I X O S"]
    } else if has("gentoo") { vec!["      _-----_","     /       \","    /  /---\\  \","   |  |     |  |","    \\  \\---/  /","     \\_____/"]
    } else if has("alpine") { vec!["       /\\","      /  \","     / /\\ \","    / /  \\ \","   /_/____\\_\\","      ALPINE"]
    } else if id=="void" { vec!["      ______","     /      \","    /  /\\    \","   /__/  \\____\\","      VOID LINUX"]
    } else { vec!["       .--.","      |o_o |","      |:_/ |","     //   \\ \","    (|     | )","   /'\\_   _/`\\","   \\___)=(___/"] };
    lines.into_iter().map(|s| match id.as_str() { "cachyos"=>s.bright_cyan().bold().to_string(), "arch"=>s.cyan().bold().to_string(), "ubuntu"=>s.red().bold().to_string(), "debian"=>s.bright_red().bold().to_string(), "fedora"=>s.bright_blue().bold().to_string(), "nixos"=>s.blue().bold().to_string(), "gentoo"=>s.magenta().bold().to_string(), "alpine"=>s.bright_cyan().bold().to_string(), "void"=>s.green().bold().to_string(), _=>s.yellow().bold().to_string() }).collect()
}

pub fn blocks()->String { let mut out=String::new(); for c in 40..48 { out.push_str(&format!("\\x1b[{c}m███")); } for c in 100..108 { out.push_str(&format!("\\x1b[{c}m███")); } out.push_str("\\x1b[0m"); out }
