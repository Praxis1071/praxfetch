use std::{env, fs, path::Path};

#[derive(Default)]
pub struct OsInfo {
    pub id: String,
    pub id_like: String,
    pub name: String,
    pub version: String,
    pub build_id: String,
}

#[derive(Default)]
pub struct HostInfo {
    pub vendor: String,
    pub product: String,
}

#[derive(Default)]
pub struct CpuInfo {
    pub model: String,
    pub cores: usize,
}

#[derive(Default)]
pub struct MemoryInfo {
    pub total_kb: u64,
    pub available_kb: u64,
}

#[derive(Default)]
pub struct SwapInfo {
    pub total_kb: u64,
    pub free_kb: u64,
}

#[derive(Default)]
pub struct PackageInfo {
    pub manager: String,
    pub count: Option<usize>,
}

#[derive(Default)]
pub struct BatteryInfo {
    pub percent: Option<f32>,
    pub charging: Option<bool>,
}

#[derive(Default)]
pub struct DiskInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Default)]
pub struct GpuInfo {
    pub name: String,
    pub driver: String,
}

#[derive(Default)]
pub struct DisplayInfo {
    pub connected: usize,
    pub resolution: String,
}

#[derive(Default)]
pub struct NetworkInfo {
    pub interfaces: usize,
    pub wireless: usize,
    pub up: usize,
}

fn read_path(path: &Path) -> String {
    fs::read_to_string(path).map(|s| s.trim().to_owned()).unwrap_or_default()
}

fn read_gpu() -> GpuInfo {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else { return GpuInfo::default(); };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("card") || name.contains('-') { continue; }
        let device = entry.path().join("device");
        let mut gpu_name = ["product_name", "product", "name"]
            .iter()
            .map(|key| read_path(&device.join(key)))
            .find(|value| !value.is_empty())
            .unwrap_or_default();
        let driver = fs::read_link(device.join("driver"))
            .ok()
            .and_then(|p| p.file_name().map(|v| v.to_string_lossy().into_owned()))
            .unwrap_or_default();
        if gpu_name.is_empty() {
            let vendor = read_path(&device.join("vendor"));
            let device_id = read_path(&device.join("device"));
            if !vendor.is_empty() && !device_id.is_empty() { gpu_name = format!("{vendor}:{device_id}"); }
        }
        if !gpu_name.is_empty() || !driver.is_empty() {
            return GpuInfo { name: if gpu_name.is_empty() { "unknown".into() } else { gpu_name }, driver };
        }
    }
    GpuInfo::default()
}

fn read_display() -> DisplayInfo {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else { return DisplayInfo::default(); };
    let mut connected = 0;
    let mut resolution = String::new();
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("card") || !name.contains('-') { continue; }
        let base = entry.path();
        if read_path(&base.join("status")) != "connected" { continue; }
        connected += 1;
        if resolution.is_empty() {
            resolution = read_path(&base.join("modes")).lines().next().unwrap_or_default().to_owned();
        }
    }
    DisplayInfo { connected, resolution }
}

fn read_network() -> NetworkInfo {
    let Ok(entries) = fs::read_dir("/sys/class/net") else { return NetworkInfo::default(); };
    let mut result = NetworkInfo::default();
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "lo" { continue; }
        result.interfaces += 1;
        if entry.path().join("wireless").exists() { result.wireless += 1; }
        if read_path(&entry.path().join("operstate")) == "up" { result.up += 1; }
    }
    result
}

pub struct SystemInfo {
    pub user: String,
    pub hostname: String,
    pub os: OsInfo,
    pub host: HostInfo,
    pub kernel: String,
    pub uptime: f64,
    pub load_avg: Option<(f64, f64, f64)>,
    pub packages: PackageInfo,
    pub shell: String,
    pub shell_version: String,
    pub desktop: String,
    pub terminal: String,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub swap: SwapInfo,
    pub battery: BatteryInfo,
    pub disk: DiskInfo,
    pub gpu: GpuInfo,
    pub display: DisplayInfo,
    pub network: NetworkInfo,
}

fn read(path: &str) -> String {
    fs::read_to_string(path)
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

fn read_os() -> OsInfo {
    let text = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut o = OsInfo::default();

    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let value = parse_os_release_value(value.trim());

        match key {
            "ID" => o.id = value,
            "ID_LIKE" => o.id_like = value,
            "NAME" => o.name = value,
            "VERSION" => o.version = value,
            "BUILD_ID" => o.build_id = value,
            _ => {}
        }
    }

    o
}

fn parse_os_release_value(value: &str) -> String {
    let value = value.trim();

    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let quoted = (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'');

        if quoted {
            let inner = &value[1..value.len() - 1];
            return unescape_os_release(inner);
        }
    }

    value.to_owned()
}

fn unescape_os_release(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some(next) => output.push(next),
                None => output.push('\\'),
            }
        } else {
            output.push(ch);
        }
    }

    output
}

fn read_cpu() -> CpuInfo {
    let text = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    let mut model = String::new();
    let mut cores = 0usize;

    for line in text.lines() {
        if line.starts_with("processor") && line.contains(':') {
            cores += 1;
        }

        if model.is_empty() {
            for key in ["model name", "Model", "Hardware"] {
                if line.starts_with(key) {
                    if let Some((_, value)) = line.split_once(':') {
                        let value = value.trim();
                        if !value.is_empty() {
                            model = value.to_owned();
                            break;
                        }
                    }
                }
            }
        }
    }

    if cores == 0 {
        cores = count_cpu_range(&read("/sys/devices/system/cpu/online"));
    }

    if model.is_empty() {
        model = read("/sys/devices/virtual/dmi/id/product_name");
    }

    if model.is_empty() {
        model = "unknown".to_owned();
    }

    CpuInfo { model, cores }
}

fn count_cpu_range(value: &str) -> usize {
    value
        .split(',')
        .map(|range| {
            let mut parts = range.trim().split('-');
            let start = parts.next().and_then(|v| v.parse::<usize>().ok());
            let end = parts.next().and_then(|v| v.parse::<usize>().ok());

            match (start, end) {
                (Some(start), Some(end)) if end >= start => end - start + 1,
                (Some(_), None) => 1,
                _ => 0,
            }
        })
        .sum()
}

fn read_memory_and_swap() -> (MemoryInfo, SwapInfo) {
    let text = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut total = 0;
    let mut available = 0;
    let mut swap_total = 0;
    let mut swap_free = 0;

    for line in text.lines() {
        if let Some(v) = line.strip_prefix("MemTotal:") {
            total = v
                .split_whitespace()
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("MemAvailable:") {
            available = v
                .split_whitespace()
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("SwapTotal:") {
            swap_total = v
                .split_whitespace()
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("SwapFree:") {
            swap_free = v
                .split_whitespace()
                .next()
                .and_then(|x| x.parse().ok())
                .unwrap_or(0);
        }
    }

    (
        MemoryInfo {
            total_kb: total,
            available_kb: available,
        },
        SwapInfo {
            total_kb: swap_total,
            free_kb: swap_free,
        },
    )
}

fn read_load_avg() -> Option<(f64, f64, f64)> {
    let text = read("/proc/loadavg");
    let mut values = text
        .split_whitespace()
        .take(3)
        .filter_map(|value| value.parse::<f64>().ok());

    Some((values.next()?, values.next()?, values.next()?))
}

fn read_packages() -> PackageInfo {
    if let Ok(entries) = fs::read_dir("/var/lib/pacman/local") {
        return PackageInfo {
            manager: "pacman".into(),
            count: Some(
                entries
                    .filter_map(Result::ok)
                    .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .count(),
            ),
        };
    }

    if Path::new("/var/lib/dpkg/status").exists() {
        let text = read("/var/lib/dpkg/status");
        return PackageInfo {
            manager: "dpkg".into(),
            count: Some(
                text.split("\n\n")
                    .filter(|paragraph| {
                        paragraph
                            .lines()
                            .any(|line| line.trim() == "Status: install ok installed")
                    })
                    .count(),
            ),
        };
    }

    if Path::new("/lib/apk/db/installed").exists() {
        return PackageInfo {
            manager: "apk".into(),
            count: Some(
                read("/lib/apk/db/installed")
                    .lines()
                    .filter(|line| line.starts_with("P:"))
                    .count(),
            ),
        };
    }

    if Path::new("/var/lib/rpm/Packages").exists()
        || Path::new("/var/lib/rpm/rpmdb.sqlite").exists()
    {
        return PackageInfo {
            manager: "rpm".into(),
            count: None,
        };
    }

    PackageInfo {
        manager: "unknown".into(),
        count: None,
    }
}

fn read_battery() -> BatteryInfo {
    let Ok(entries) = fs::read_dir("/sys/class/power_supply") else {
        return BatteryInfo::default();
    };

    let mut count = 0u32;
    let mut percent_sum = 0.0f32;
    let mut any_charging = false;
    let mut any_discharging = false;

    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("BAT") {
            continue;
        }

        let base = entry.path();
        if let Some(percent) = fs::read_to_string(base.join("capacity"))
            .ok()
            .and_then(|v| v.trim().parse::<f32>().ok())
            .filter(|v| (0.0..=100.0).contains(v))
        {
            percent_sum += percent;
            count += 1;
        }

        match fs::read_to_string(base.join("status"))
            .ok()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("charging") | Some("full") => any_charging = true,
            Some("discharging") => any_discharging = true,
            _ => {}
        }
    }

    if count == 0 {
        return BatteryInfo::default();
    }

    let charging = if any_charging {
        Some(true)
    } else if any_discharging {
        Some(false)
    } else {
        None
    };

    BatteryInfo {
        percent: Some(percent_sum / count as f32),
        charging,
    }
}

fn read_disk() -> DiskInfo {
    let path = std::ffi::CString::new("/").expect("static path contains no NUL");
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::zeroed();

    let result = unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) };
    if result != 0 {
        return DiskInfo::default();
    }

    let stat = unsafe { stat.assume_init() };
    let block_size = stat.f_frsize as u64;
    let total = (stat.f_blocks as u64).saturating_mul(block_size);
    let available = (stat.f_bavail as u64).saturating_mul(block_size);

    DiskInfo {
        total_bytes: total,
        available_bytes: available.min(total),
    }
}

pub fn collect() -> SystemInfo {
    let shell = env::var("SHELL").unwrap_or_else(|_| "unknown".into());
    let shell_version = env::var("BASH_VERSION")
        .or_else(|_| env::var("ZSH_VERSION"))
        .or_else(|_| env::var("FISH_VERSION"))
        .unwrap_or_default();

    let desktop = env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("DESKTOP_SESSION"))
        .unwrap_or_else(|_| "unknown".into());

    let terminal = env::var("TERM_PROGRAM")
        .or_else(|_| env::var("TERM"))
        .unwrap_or_else(|_| "unknown".into());

    let (memory, swap) = read_memory_and_swap();

    SystemInfo {
        user: env::var("USER")
            .or_else(|_| env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown".into()),
        hostname: read("/proc/sys/kernel/hostname"),
        os: read_os(),
        host: HostInfo {
            vendor: read("/sys/class/dmi/id/sys_vendor"),
            product: read("/sys/class/dmi/id/product_name"),
        },
        kernel: read("/proc/sys/kernel/osrelease"),
        uptime: read("/proc/uptime")
            .split_whitespace()
            .next()
            .and_then(|x| x.parse().ok())
            .unwrap_or(0.0),
        load_avg: read_load_avg(),
        packages: read_packages(),
        shell,
        shell_version,
        desktop,
        terminal,
        cpu: read_cpu(),
        memory,
        swap,
        battery: read_battery(),
        disk: read_disk(),
        gpu: read_gpu(),
        display: read_display(),
        network: read_network(),
    }
}

pub fn os_label(os: &OsInfo) -> String {
    if os.name.is_empty() {
        return "unknown".to_owned();
    }

    if !os.version.is_empty() {
        return format!("{} {}", os.name, os.version);
    }

    if !os.build_id.is_empty() {
        return format!("{} ({})", os.name, os.build_id);
    }

    os.name.clone()
}

pub fn uptime(seconds: f64) -> String {
    let n = seconds.max(0.0) as u64;
    let d = n / 86_400;
    let h = n % 86_400 / 3_600;
    let m = n % 3_600 / 60;

    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else {
        format!("{h}h {m}m")
    }
}

pub fn memory(m: &MemoryInfo) -> String {
    if m.total_kb == 0 {
        return "unknown".into();
    }

    let available = m.available_kb.min(m.total_kb);
    let used = m.total_kb.saturating_sub(available);
    let percent = used as f64 / m.total_kb as f64 * 100.0;

    format!(
        "{:.1} GiB / {:.1} GiB ({:.0}%)",
        used as f64 / 1_048_576.0,
        m.total_kb as f64 / 1_048_576.0,
        percent
    )
}

pub fn swap(s: &SwapInfo) -> String {
    if s.total_kb == 0 {
        return "none".into();
    }

    let free = s.free_kb.min(s.total_kb);
    let used = s.total_kb.saturating_sub(free);

    format!(
        "{:.1} GiB / {:.1} GiB ({:.0}%)",
        used as f64 / 1_048_576.0,
        s.total_kb as f64 / 1_048_576.0,
        used as f64 / s.total_kb as f64 * 100.0
    )
}

pub fn disk(d: &DiskInfo) -> String {
    if d.total_bytes == 0 {
        return "unknown".into();
    }

    let used = d.total_bytes.saturating_sub(d.available_bytes);
    format!(
        "{:.1} GiB / {:.1} GiB ({:.0}%)",
        used as f64 / 1_073_741_824.0,
        d.total_bytes as f64 / 1_073_741_824.0,
        used as f64 / d.total_bytes as f64 * 100.0
    )
}

pub fn battery(b: &BatteryInfo) -> String {
    let Some(percent) = b.percent else {
        return "none".into();
    };

    let state = match b.charging {
        Some(true) => "charging",
        Some(false) => "discharging",
        None => "unknown",
    };

    format!("{percent:.0}% ({state})")
}

pub fn gpu(g: &GpuInfo) -> String {
    match (g.name.is_empty(), g.driver.is_empty()) {
        (true, true) => "unknown".into(),
        (false, true) => g.name.clone(),
        (true, false) => g.driver.clone(),
        (false, false) => format!("{} ({})", g.name, g.driver),
    }
}

pub fn display(d: &DisplayInfo) -> String {
    if d.connected == 0 { return "none".into(); }
    if d.resolution.is_empty() { return format!("{} connected", d.connected); }
    format!("{} ({} connected)", d.resolution, d.connected)
}

pub fn network(n: &NetworkInfo) -> String {
    if n.interfaces == 0 { return "none".into(); }
    format!("{} up / {} interfaces ({} wireless)", n.up, n.interfaces, n.wireless)
}

pub fn shell(shell: &str, version: &str) -> String {
    let name = Path::new(shell)
        .file_name()
        .and_then(|x| x.to_str())
        .unwrap_or(shell);

    if version.is_empty() {
        name.into()
    } else {
        format!("{name} {version}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_ranges_are_counted() {
        assert_eq!(count_cpu_range("0-3"), 4);
        assert_eq!(count_cpu_range("0-3,8-11"), 8);
        assert_eq!(count_cpu_range("0,2,4"), 3);
        assert_eq!(count_cpu_range(""), 0);
    }

    #[test]
    fn uptime_is_formatted() {
        assert_eq!(uptime(0.0), "0h 0m");
        assert_eq!(uptime(3661.0), "1h 1m");
        assert_eq!(uptime(90_061.0), "1d 1h 1m");
    }

    #[test]
    fn memory_never_exceeds_total() {
        let value = memory(&MemoryInfo {
            total_kb: 1_048_576,
            available_kb: 2_000_000,
        });
        assert_eq!(value, "0.0 GiB / 1.0 GiB (0%)");
    }

    #[test]
    fn swap_handles_missing_and_clamped_values() {
        assert_eq!(swap(&SwapInfo { total_kb: 0, free_kb: 0 }), "none");
        assert_eq!(
            swap(&SwapInfo {
                total_kb: 1_048_576,
                free_kb: 2_000_000,
            }),
            "0.0 GiB / 1.0 GiB (0%)"
        );
    }

    #[test]
    fn disk_handles_empty_data() {
        assert_eq!(
            disk(&DiskInfo {
                total_bytes: 0,
                available_bytes: 0
            }),
            "unknown"
        );
    }

    #[test]
    fn battery_without_hardware_is_none() {
        assert_eq!(battery(&BatteryInfo::default()), "none");
    }

    #[test]
    fn os_release_quotes_and_escapes_are_parsed() {
        assert_eq!(parse_os_release_value("\"CachyOS 2026\""), "CachyOS 2026");
        assert_eq!(parse_os_release_value("'CachyOS'"), "CachyOS");
        assert_eq!(parse_os_release_value("\"A\\\\\\\"B\""), "A\"B");
    }
}
