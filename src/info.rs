use std::{env, fs, path::Path};

#[derive(Default)] pub struct OsInfo { pub id:String, pub id_like:String, pub name:String, pub version:String, pub build_id:String }
#[derive(Default)] pub struct HostInfo { pub vendor:String, pub product:String }
#[derive(Default)] pub struct CpuInfo { pub model:String, pub cores:usize }
#[derive(Default)] pub struct MemoryInfo { pub total_kb:u64, pub available_kb:u64 }
#[derive(Default)] pub struct PackageInfo { pub manager:String, pub count:usize }
pub struct SystemInfo { pub user:String, pub hostname:String, pub os:OsInfo, pub host:HostInfo, pub kernel:String, pub uptime:f64, pub packages:PackageInfo, pub shell:String, pub shell_version:String, pub desktop:String, pub terminal:String, pub cpu:CpuInfo, pub memory:MemoryInfo }

fn read(path:&str)->String { fs::read_to_string(path).map(|s| s.trim().to_owned()).unwrap_or_default() }

fn read_os()->OsInfo {
    let text=fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut o=OsInfo::default();
    for line in text.lines() {
        if let Some((k,v))=line.split_once('=') {
            let v=v.trim().trim_matches('"');
            match k { "ID"=>o.id=v.into(), "ID_LIKE"=>o.id_like=v.into(), "NAME"=>o.name=v.into(), "VERSION"=>o.version=v.into(), "BUILD_ID"=>o.build_id=v.into(), _=>{} }
        }
    }
    o
}

fn read_cpu()->CpuInfo {
    let text=fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut model=String::new(); let mut cores=0;
    for line in text.lines() {
        if line.starts_with("processor") && line.contains(':') { cores+=1; }
        if model.is_empty() && line.starts_with("model name") { if let Some((_,v))=line.split_once(':') { model=v.trim().into(); } }
    }
    CpuInfo{model,cores}
}

fn read_memory()->MemoryInfo {
    let text=fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total=0; let mut available=0;
    for line in text.lines() {
        if let Some(v)=line.strip_prefix("MemTotal:") { total=v.split_whitespace().next().and_then(|x|x.parse().ok()).unwrap_or(0); }
        else if let Some(v)=line.strip_prefix("MemAvailable:") { available=v.split_whitespace().next().and_then(|x|x.parse().ok()).unwrap_or(0); }
    }
    MemoryInfo{total_kb:total,available_kb:available}
}

fn read_packages()->PackageInfo {
    if let Ok(entries)=fs::read_dir("/var/lib/pacman/local") { return PackageInfo{manager:"pacman".into(),count:entries.filter_map(Result::ok).filter(|e|e.path().is_dir()).count()}; }
    if Path::new("/var/lib/dpkg/status").exists() { let text=read("/var/lib/dpkg/status"); return PackageInfo{manager:"dpkg".into(),count:text.split("\n\n").filter(|p|p.lines().any(|l|l=="Status: install ok installed")).count()}; }
    if Path::new("/lib/apk/db/installed").exists() { return PackageInfo{manager:"apk".into(),count:read("/lib/apk/db/installed").lines().filter(|l|l.starts_with("P:")).count()}; }
    PackageInfo{manager:"unknown".into(),count:0}
}

pub fn collect()->SystemInfo {
    let shell=env::var("SHELL").unwrap_or_else(|_|"unknown".into());
    let shell_version=env::var("BASH_VERSION").or_else(|_|env::var("ZSH_VERSION")).or_else(|_|env::var("FISH_VERSION")).unwrap_or_default();
    let desktop=env::var("XDG_CURRENT_DESKTOP").or_else(|_|env::var("DESKTOP_SESSION")).unwrap_or_else(|_|"unknown".into());
    let terminal=env::var("TERM_PROGRAM").or_else(|_|env::var("TERM")).unwrap_or_else(|_|"unknown".into());
    SystemInfo {
        user:env::var("USER").or_else(|_|env::var("USERNAME")).unwrap_or_else(|_|"unknown".into()),
        hostname:read("/proc/sys/kernel/hostname"), os:read_os(),
        host:HostInfo{vendor:read("/sys/class/dmi/id/sys_vendor"),product:read("/sys/class/dmi/id/product_name")},
        kernel:read("/proc/sys/kernel/osrelease"),
        uptime:read("/proc/uptime").split_whitespace().next().and_then(|x|x.parse().ok()).unwrap_or(0.0),
        packages:read_packages(), shell, shell_version, desktop, terminal, cpu:read_cpu(), memory:read_memory()
    }
}

pub fn uptime(s:f64)->String { let n=s as u64; let d=n/86400; let h=n%86400/3600; let m=n%3600/60; if d>0 { format!("{d}d {h}h {m}m") } else { format!("{h}h {m}m") } }
pub fn memory(m:&MemoryInfo)->String { if m.total_kb==0 { return "unknown".into(); } let used=m.total_kb.saturating_sub(m.available_kb); format!("{:.1} GiB / {:.1} GiB ({:.0}%)",used as f64/1048576.0,m.total_kb as f64/1048576.0,used as f64/m.total_kb as f64*100.0) }
pub fn shell(s:&str,v:&str)->String { let name=Path::new(s).file_name().and_then(|x|x.to_str()).unwrap_or(s); if v.is_empty(){name.into()}else{format!("{name} {v}")} }
