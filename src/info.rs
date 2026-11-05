use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::CString,
    fs,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub struct OsInfo {
    pub id: String,
    pub id_like: String,
    pub name: String,
    pub pretty_name: String,
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
    pub physical_cores: Option<usize>,
    pub threads: usize,
    pub frequency_mhz: Option<u64>,
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
    pub status: BatteryStatus,
}

#[derive(Default, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    #[default]
    Unknown,
}

#[derive(Default)]
pub struct DiskInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub filesystems: usize,
}

struct MountEntry {
    device: String,
    mount_point: PathBuf,
}

#[derive(Default)]
pub struct GpuInfo {
    pub devices: Vec<GpuDevice>,
}

#[derive(Default)]
pub struct GpuDevice {
    pub name: String,
    pub driver: String,
}

#[derive(Default)]
pub struct DisplayInfo {
    pub devices: Vec<DisplayDevice>,
}

#[derive(Default)]
pub struct DisplayDevice {
    pub connector: String,
    pub resolution: String,
}

#[derive(Default)]
pub struct NetworkInfo {
    pub interfaces: usize,
    pub wireless: usize,
    pub up: usize,
}

fn read_path(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

fn gpu_vendor_name(vendor: &str) -> Option<&'static str> {
    match vendor.to_ascii_lowercase().as_str() {
        "0x8086" => Some("Intel"),
        "0x1002" => Some("AMD"),
        "0x10de" => Some("NVIDIA"),
        "0x102b" => Some("Matrox"),
        "0x121a" => Some("3dfx"),
        "0x1414" => Some("Microsoft"),
        "0x15ad" => Some("VMware"),
        "0x1af4" => Some("Virtio"),
        "0x1b36" => Some("QEMU"),
        _ => None,
    }
}

fn read_gpu() -> GpuInfo {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return GpuInfo::default();
    };
    let mut devices = Vec::new();
    let mut pending_pci_lookups = Vec::new();
    let mut wanted_ids = BTreeSet::new();

    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }

        let device = entry.path().join("device");
        let mut gpu_name = ["product_name", "product", "name"]
            .iter()
            .map(|key| read_path(&device.join(key)))
            .find(|value| !value.is_empty())
            .unwrap_or_default();

        let driver = fs::read_link(device.join("driver"))
            .ok()
            .and_then(|path| {
                path.file_name()
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .unwrap_or_default();

        let mut pci_lookup = None;
        if gpu_name.is_empty() {
            let vendor = normalize_pci_id(&read_path(&device.join("vendor")));
            let device_id = normalize_pci_id(&read_path(&device.join("device")));
            if !vendor.is_empty() && !device_id.is_empty() {
                wanted_ids.insert((vendor.clone(), device_id.clone()));
                gpu_name = match gpu_vendor_name(&format!("0x{vendor}")) {
                    Some(vendor_name) => format!("{vendor_name} GPU 0x{device_id}"),
                    None => format!("0x{vendor}:0x{device_id}"),
                };
                pci_lookup = Some((vendor, device_id));
            }
        }

        if !gpu_name.is_empty() || !driver.is_empty() {
            let index = devices.len();
            devices.push(GpuDevice {
                name: if gpu_name.is_empty() {
                    "unknown".into()
                } else {
                    gpu_name
                },
                driver,
            });
            if let Some((vendor, device_id)) = pci_lookup {
                pending_pci_lookups.push((index, vendor, device_id));
            }
        }
    }

    if !wanted_ids.is_empty() {
        let names = read_pci_device_names(&wanted_ids);
        for (index, vendor, device_id) in pending_pci_lookups {
            if let Some(model) = names.get(&(vendor.clone(), device_id.clone())) {
                let vendor_name = gpu_vendor_name(&format!("0x{vendor}"));
                devices[index].name = match vendor_name {
                    Some(name) => format!("{name} {model}"),
                    None => model.clone(),
                };
            }
        }
    }

    GpuInfo { devices }
}

fn normalize_pci_id(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .to_ascii_lowercase()
}

fn parse_pci_ids(
    text: &str,
    wanted: &BTreeSet<(String, String)>,
) -> BTreeMap<(String, String), String> {
    let mut names = BTreeMap::new();
    let mut current_vendor = String::new();

    for line in text.lines() {
        if line.starts_with("\t\t") {
            continue;
        }

        if let Some(device_line) = line.strip_prefix('\t') {
            if current_vendor.is_empty() {
                continue;
            }

            let mut fields = device_line.split_whitespace();
            let Some(device_id) = fields.next() else {
                continue;
            };
            if device_id.len() != 4 || !device_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                continue;
            }

            let key = (current_vendor.clone(), device_id.to_ascii_lowercase());
            if wanted.contains(&key) {
                let name = fields.collect::<Vec<_>>().join(" ");
                if !name.is_empty() {
                    names.insert(key, name);
                }
            }
        } else {
            let mut fields = line.split_whitespace();
            let Some(vendor_id) = fields.next() else {
                current_vendor.clear();
                continue;
            };
            current_vendor =
                if vendor_id.len() == 4 && vendor_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    vendor_id.to_ascii_lowercase()
                } else {
                    String::new()
                };
        }

        if names.len() == wanted.len() {
            break;
        }
    }

    names
}

fn read_pci_device_names(
    wanted: &BTreeSet<(String, String)>,
) -> BTreeMap<(String, String), String> {
    for path in [
        "/usr/share/hwdata/pci.ids",
        "/usr/share/misc/pci.ids",
        "/usr/share/pci.ids",
    ] {
        if let Ok(text) = fs::read_to_string(path) {
            let names = parse_pci_ids(&text, wanted);
            if !names.is_empty() {
                return names;
            }
        }
    }

    BTreeMap::new()
}

fn read_display() -> DisplayInfo {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return DisplayInfo::default();
    };

    let entries: Vec<_> = entries.filter_map(Result::ok).collect();
    let mut devices = Vec::new();
    let mut unknown = Vec::new();

    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with("card") || !name.contains('-') {
            continue;
        }

        let base = entry.path();
        let status = read_path(&base.join("status"));
        if status != "connected" && status != "unknown" {
            continue;
        }

        let resolution = read_path(&base.join("modes"))
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned();

        let connector = name
            .split_once('-')
            .map(|(_, connector)| connector.to_owned())
            .unwrap_or(name);

        let device = DisplayDevice {
            connector,
            resolution,
        };

        if status == "connected" {
            devices.push(device);
        } else {
            unknown.push(device);
        }
    }

    // DRM can report "unknown" when reliable probing is unavailable.
    // Only use those fallback connectors if no definitely connected one exists.
    if devices.is_empty() {
        devices = unknown;
    }

    devices.sort_by(|a, b| a.connector.cmp(&b.connector));
    DisplayInfo { devices }
}

fn read_network() -> NetworkInfo {
    let Ok(entries) = fs::read_dir("/sys/class/net") else {
        return NetworkInfo::default();
    };

    let mut result = NetworkInfo::default();

    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "lo" {
            continue;
        }

        result.interfaces += 1;
        let path = entry.path();

        if path.join("wireless").exists() {
            result.wireless += 1;
        }

        if read_path(&path.join("operstate")) == "up" {
            result.up += 1;
        }
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
    let text = fs::read_to_string("/etc/os-release")
        .or_else(|_| fs::read_to_string("/usr/lib/os-release"))
        .unwrap_or_default();
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
            "PRETTY_NAME" => o.pretty_name = value,
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
                Some(next @ ('\\' | '"' | '$' | '`')) => output.push(next),
                Some(next) => {
                    output.push('\\');
                    output.push(next);
                }
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
    let mut proc_threads = 0usize;

    for line in text.lines() {
        if line.starts_with("processor") && line.contains(':') {
            proc_threads += 1;
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

    let online_cpus = parse_cpu_list(&read("/sys/devices/system/cpu/online"));
    let threads = if online_cpus.is_empty() {
        proc_threads
    } else {
        online_cpus.len()
    };

    let physical_cores = if online_cpus.is_empty() {
        None
    } else {
        read_physical_core_count(&online_cpus)
    };

    if model.is_empty() {
        model = read("/sys/devices/virtual/dmi/id/product_name");
    }

    if model.is_empty() {
        model = "unknown".to_owned();
    }

    CpuInfo {
        model,
        physical_cores,
        threads,
        frequency_mhz: read_cpu_frequency_mhz(),
    }
}

fn read_cpu_frequency_mhz() -> Option<u64> {
    let root = Path::new("/sys/devices/system/cpu/cpufreq");
    let Ok(entries) = fs::read_dir(root) else {
        return read_per_cpu_frequency_mhz();
    };

    let mut policies: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("policy"))
        .map(|entry| entry.path())
        .collect();
    policies.sort();

    if policies.is_empty() {
        return read_per_cpu_frequency_mhz();
    }

    // Prefer hardware-reported averages/current values. scaling_cur_freq is
    // only a last resort because it can represent the last requested P-state.
    for attribute in ["cpuinfo_avg_freq", "cpuinfo_cur_freq", "scaling_cur_freq"] {
        let mut weighted_total = 0u128;
        let mut total_weight = 0u128;
        let mut complete = true;

        for policy in &policies {
            let frequency = read_path(&policy.join(attribute))
                .parse::<u64>()
                .ok()
                .filter(|value| *value > 0);
            let Some(frequency) = frequency else {
                complete = false;
                break;
            };

            let weight = parse_cpu_list(&read_path(&policy.join("affected_cpus")))
                .len()
                .max(1) as u128;
            weighted_total += frequency as u128 * weight;
            total_weight += weight;
        }

        if complete && total_weight > 0 {
            return Some(((weighted_total / total_weight + 500) / 1000) as u64);
        }
    }

    read_per_cpu_frequency_mhz()
}

fn read_per_cpu_frequency_mhz() -> Option<u64> {
    let cpus = parse_cpu_list(&read("/sys/devices/system/cpu/online"));
    if cpus.is_empty() {
        return None;
    }

    for attribute in ["cpuinfo_avg_freq", "cpuinfo_cur_freq", "scaling_cur_freq"] {
        let frequencies: Vec<u64> = cpus
            .iter()
            .filter_map(|cpu| {
                read_path(
                    &Path::new("/sys/devices/system/cpu")
                        .join(format!("cpu{cpu}/cpufreq"))
                        .join(attribute),
                )
                .parse::<u64>()
                .ok()
                .filter(|value| *value > 0)
            })
            .collect();

        if frequencies.len() == cpus.len() {
            let average = frequencies.iter().map(|value| *value as u128).sum::<u128>()
                / frequencies.len() as u128;
            return Some(((average + 500) / 1000) as u64);
        }
    }

    None
}

fn parse_cpu_list(value: &str) -> Vec<usize> {
    let mut cpus = BTreeSet::new();

    for item in value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let mut bounds = item.split('-');
        let Some(start) = bounds.next().and_then(|part| part.parse::<usize>().ok()) else {
            continue;
        };

        match (bounds.next(), bounds.next()) {
            (None, None) => {
                cpus.insert(start);
            }
            (Some(end), None) => {
                if let Ok(end) = end.parse::<usize>() {
                    if end >= start && end - start <= 65_536 {
                        cpus.extend(start..=end);
                    }
                }
            }
            _ => {}
        }
    }

    cpus.into_iter().collect()
}

fn read_physical_core_count(online_cpus: &[usize]) -> Option<usize> {
    let mut cores = Vec::with_capacity(online_cpus.len());

    for cpu in online_cpus {
        let topology = format!("/sys/devices/system/cpu/cpu{cpu}/topology");
        let package_id = read_path(&Path::new(&topology).join("physical_package_id"))
            .parse::<i32>()
            .ok()?;

        // A negative package ID is the kernel's "topology unavailable" fallback.
        if package_id < 0 {
            return None;
        }

        let sibling_cpus = parse_cpu_list(&read_path(&Path::new(&topology).join("core_cpus_list")));
        if sibling_cpus.is_empty() || !sibling_cpus.contains(cpu) {
            return None;
        }

        cores.push((package_id, sibling_cpus));
    }

    if cores.is_empty() {
        None
    } else {
        Some(count_unique_core_groups(cores))
    }
}

fn count_unique_core_groups<I>(groups: I) -> usize
where
    I: IntoIterator<Item = (i32, Vec<usize>)>,
{
    groups.into_iter().collect::<BTreeSet<_>>().len()
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

    if [
        "/var/lib/rpm/Packages",
        "/var/lib/rpm/rpmdb.sqlite",
        "/var/lib/rpm/Packages.db",
        "/usr/lib/sysimage/rpm/Packages",
        "/usr/lib/sysimage/rpm/rpmdb.sqlite",
        "/usr/lib/sysimage/rpm/Packages.db",
    ]
    .iter()
    .any(|path| Path::new(path).exists())
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

    let mut batteries = Vec::new();
    let mut any_charging = false;
    let mut any_discharging = false;
    let mut any_not_charging = false;
    let mut all_full = true;

    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let base = entry.path();
        let supply_type = read_path(&base.join("type"));
        if supply_type != "Battery" && !name.starts_with("BAT") {
            continue;
        }

        if read_path(&base.join("present")) == "0" {
            continue;
        }

        let percent = fs::read_to_string(base.join("capacity"))
            .ok()
            .and_then(|v| v.trim().parse::<f32>().ok())
            .filter(|v| (0.0..=100.0).contains(v));

        if let Some(percent) = percent {
            let energy_full = fs::read_to_string(base.join("energy_full"))
                .ok()
                .and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|value| *value > 0.0);
            let charge_full = fs::read_to_string(base.join("charge_full"))
                .ok()
                .and_then(|v| v.trim().parse::<f32>().ok())
                .filter(|value| *value > 0.0);

            batteries.push((percent, energy_full, charge_full));
        }

        match fs::read_to_string(base.join("status"))
            .ok()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("charging") => {
                any_charging = true;
                all_full = false;
            }
            Some("discharging") => {
                any_discharging = true;
                all_full = false;
            }
            Some("not charging") => {
                any_not_charging = true;
                all_full = false;
            }
            Some("full") => {}
            _ => all_full = false,
        }
    }

    let Some(percent) = aggregate_battery_capacity(&batteries) else {
        return BatteryInfo::default();
    };

    let status = if any_charging {
        BatteryStatus::Charging
    } else if any_discharging {
        BatteryStatus::Discharging
    } else if all_full {
        BatteryStatus::Full
    } else if any_not_charging {
        BatteryStatus::NotCharging
    } else {
        BatteryStatus::Unknown
    };

    BatteryInfo {
        percent: Some(percent),
        status,
    }
}

fn aggregate_battery_capacity(batteries: &[(f32, Option<f32>, Option<f32>)]) -> Option<f32> {
    if batteries.is_empty() {
        return None;
    }

    if batteries.iter().all(|battery| battery.1.is_some()) {
        weighted_battery_average(batteries, true)
    } else if batteries.iter().all(|battery| battery.2.is_some()) {
        weighted_battery_average(batteries, false)
    } else {
        Some(batteries.iter().map(|battery| battery.0).sum::<f32>() / batteries.len() as f32)
    }
}

fn weighted_battery_average(
    batteries: &[(f32, Option<f32>, Option<f32>)],
    use_energy: bool,
) -> Option<f32> {
    let weights = batteries
        .iter()
        .map(|battery| if use_energy { battery.1 } else { battery.2 })
        .collect::<Option<Vec<_>>>()?;
    let total_weight: f32 = weights.iter().sum();

    if total_weight <= 0.0 {
        return None;
    }

    let weighted_percent: f32 = batteries
        .iter()
        .zip(weights.iter())
        .map(|(battery, weight)| battery.0 * weight)
        .sum();

    Some(weighted_percent / total_weight)
}

fn decode_mount_field(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'\\' {
            if index + 3 >= bytes.len()
                || !bytes[index + 1..index + 4]
                    .iter()
                    .all(|byte| (b'0'..=b'7').contains(byte))
            {
                return None;
            }

            let octal = u16::from(bytes[index + 1] - b'0') * 64
                + u16::from(bytes[index + 2] - b'0') * 8
                + u16::from(bytes[index + 3] - b'0');
            decoded.push(u8::try_from(octal).ok()?);
            index += 4;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).ok()
}

fn parse_mountinfo(text: &str) -> Vec<MountEntry> {
    let mut mounts = Vec::new();

    for line in text.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        let Some(separator) = fields.iter().position(|field| *field == "-") else {
            continue;
        };

        if separator < 6 || fields.len() < separator + 4 {
            continue;
        }

        let device = fields[2];
        let Some((major, minor)) = device.split_once(':') else {
            continue;
        };
        if major.parse::<u32>().is_err() || minor.parse::<u32>().is_err() {
            continue;
        }

        let Some(mount_point) = decode_mount_field(fields[4]) else {
            continue;
        };
        let fs_type = fields[separator + 1];

        if is_pseudo_filesystem(fs_type) {
            continue;
        }

        mounts.push(MountEntry {
            device: device.to_owned(),
            mount_point: PathBuf::from(mount_point),
        });
    }

    mounts
}

fn is_pseudo_filesystem(fs_type: &str) -> bool {
    matches!(
        fs_type,
        "proc"
            | "sysfs"
            | "devtmpfs"
            | "devpts"
            | "tmpfs"
            | "ramfs"
            | "cgroup"
            | "cgroup2"
            | "mqueue"
            | "securityfs"
            | "debugfs"
            | "tracefs"
            | "configfs"
            | "pstore"
            | "efivarfs"
            | "bpf"
            | "rpc_pipefs"
            | "autofs"
            | "nsfs"
            | "fusectl"
    )
}

fn filesystem_capacity(path: &Path) -> Option<(u64, u64)> {
    let path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::zeroed();

    let result = unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) };
    if result != 0 {
        return None;
    }

    let stat = unsafe { stat.assume_init() };
    let block_size = if stat.f_frsize > 0 {
        stat.f_frsize as u64
    } else {
        stat.f_bsize as u64
    };
    let total = (stat.f_blocks as u64).checked_mul(block_size)?;
    let available = (stat.f_bavail as u64).checked_mul(block_size)?.min(total);

    (total > 0).then_some((total, available))
}

fn read_disk() -> DiskInfo {
    let Ok(mountinfo) = fs::read_to_string("/proc/self/mountinfo") else {
        return filesystem_capacity(Path::new("/"))
            .map(|(total_bytes, available_bytes)| DiskInfo {
                total_bytes,
                available_bytes,
                filesystems: 1,
            })
            .unwrap_or_default();
    };

    let mut seen_devices = BTreeSet::new();
    let mut result = DiskInfo::default();

    for mount in parse_mountinfo(&mountinfo) {
        // Bind mounts and multiple views of the same filesystem share the
        // same major:minor device number; count their capacity only once.
        if seen_devices.contains(&mount.device) {
            continue;
        }

        let Some((total, available)) = filesystem_capacity(&mount.mount_point) else {
            continue;
        };
        seen_devices.insert(mount.device);

        result.total_bytes = result.total_bytes.saturating_add(total);
        result.available_bytes = result.available_bytes.saturating_add(available);
        result.filesystems += 1;
    }

    if result.filesystems == 0 {
        return filesystem_capacity(Path::new("/"))
            .map(|(total_bytes, available_bytes)| DiskInfo {
                total_bytes,
                available_bytes,
                filesystems: 1,
            })
            .unwrap_or_default();
    }

    result.available_bytes = result.available_bytes.min(result.total_bytes);
    result
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
    if !os.pretty_name.is_empty() {
        return os.pretty_name.clone();
    }

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

    let state = match b.status {
        BatteryStatus::Charging => "charging",
        BatteryStatus::Discharging => "discharging",
        BatteryStatus::Full => "full",
        BatteryStatus::NotCharging => "not charging",
        BatteryStatus::Unknown => "unknown",
    };

    format!("{percent:.0}% ({state})")
}

pub fn gpu(g: &GpuInfo) -> String {
    if g.devices.is_empty() {
        return "unknown".into();
    }

    g.devices
        .iter()
        .map(
            |device| match (device.name.is_empty(), device.driver.is_empty()) {
                (true, true) => "unknown".to_owned(),
                (false, true) => device.name.clone(),
                (true, false) => device.driver.clone(),
                (false, false) => format!("{} ({})", device.name, device.driver),
            },
        )
        .collect::<Vec<_>>()
        .join(" / ")
}

pub fn display(d: &DisplayInfo) -> String {
    if d.devices.is_empty() {
        return "none".into();
    }

    d.devices
        .iter()
        .map(|device| {
            if device.resolution.is_empty() {
                device.connector.clone()
            } else {
                format!("{} {}", device.resolution, device.connector)
            }
        })
        .collect::<Vec<_>>()
        .join(" / ")
}

pub fn network(n: &NetworkInfo) -> String {
    if n.interfaces == 0 {
        return "none".into();
    }

    format!(
        "{} up / {} interfaces ({} wireless)",
        n.up, n.interfaces, n.wireless
    )
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
    fn pci_ids_parser_matches_only_requested_top_level_devices() {
        let database = "\
8086  Intel Corporation\n\
\t1234  Example Integrated Graphics\n\
\t\t0001  Example Subsystem\n\
1002  Advanced Micro Devices, Inc. [AMD/ATI]\n\
\tabcd  Example Radeon Device\n";
        let wanted = [
            ("8086".to_owned(), "1234".to_owned()),
            ("1002".to_owned(), "abcd".to_owned()),
        ]
        .into_iter()
        .collect();
        let names = parse_pci_ids(database, &wanted);
        assert_eq!(
            names
                .get(&("8086".into(), "1234".into()))
                .map(String::as_str),
            Some("Example Integrated Graphics")
        );
        assert_eq!(
            names
                .get(&("1002".into(), "abcd".into()))
                .map(String::as_str),
            Some("Example Radeon Device")
        );
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn pci_ids_are_normalized() {
        assert_eq!(normalize_pci_id("0x10DE"), "10de");
        assert_eq!(normalize_pci_id("8086"), "8086");
    }

    #[test]
    fn cpu_list_parser_deduplicates_and_ignores_invalid_ranges() {
        assert_eq!(parse_cpu_list("0-2,2,4-5,broken,8-7"), vec![0, 1, 2, 4, 5]);
    }

    #[test]
    fn physical_core_groups_are_deduplicated_per_package() {
        let groups = vec![
            (0, vec![0, 1]),
            (0, vec![0, 1]),
            (0, vec![2, 3]),
            (1, vec![0, 1]),
        ];
        assert_eq!(count_unique_core_groups(groups), 3);
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
        assert_eq!(
            swap(&SwapInfo {
                total_kb: 0,
                free_kb: 0
            }),
            "none"
        );
        assert_eq!(
            swap(&SwapInfo {
                total_kb: 1_048_576,
                free_kb: 2_000_000,
            }),
            "0.0 GiB / 1.0 GiB (0%)"
        );
    }

    #[test]
    fn mountinfo_parser_decodes_escaped_paths_and_skips_pseudo_filesystems() {
        let mountinfo = "\
36 25 8:1 / / rw,relatime - ext4 /dev/sda1 rw\n\
37 25 0:32 / /run rw,nosuid - tmpfs tmpfs rw\n\
38 25 8:2 / /mnt/My\\040Disk rw,relatime shared:1 - ext4 /dev/sda2 rw\n\
39 25 0:5 / /dev rw,nosuid - devtmpfs devtmpfs rw\n";
        let mounts = parse_mountinfo(mountinfo);
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[0].device, "8:1");
        assert_eq!(mounts[1].mount_point, PathBuf::from("/mnt/My Disk"));
    }

    #[test]
    fn mountinfo_parser_ignores_malformed_lines_and_unknown_escapes() {
        let mountinfo = "\
not a valid mount line\n\
40 25 8:3 / /mnt/bad\\999 rw - ext4 /dev/sda3 rw\n";
        assert!(parse_mountinfo(mountinfo).is_empty());
    }

    #[test]
    fn mount_field_decoder_handles_kernel_escape_sequences() {
        assert_eq!(
            decode_mount_field(r"/mnt/a\040b\011c\012d\134e").as_deref(),
            Some("/mnt/a b\tc\nd\\e")
        );
        assert_eq!(decode_mount_field(r"/mnt/bad\777"), None);
        assert_eq!(decode_mount_field(r"/mnt/bad\04x"), None);
    }

    #[test]
    fn pseudo_filesystem_filter_keeps_real_and_overlay_filesystems() {
        assert!(is_pseudo_filesystem("proc"));
        assert!(is_pseudo_filesystem("tmpfs"));
        assert!(!is_pseudo_filesystem("ext4"));
        assert!(!is_pseudo_filesystem("btrfs"));
        assert!(!is_pseudo_filesystem("overlay"));
        assert!(!is_pseudo_filesystem("fuseblk"));
    }

    #[test]
    fn disk_handles_empty_data() {
        assert_eq!(
            disk(&DiskInfo {
                total_bytes: 0,
                available_bytes: 0,
                filesystems: 0,
            }),
            "unknown"
        );
    }

    #[test]
    fn gpu_formats_multiple_devices() {
        let value = gpu(&GpuInfo {
            devices: vec![
                GpuDevice {
                    name: "Intel GPU".into(),
                    driver: "i915".into(),
                },
                GpuDevice {
                    name: "AMD GPU".into(),
                    driver: "amdgpu".into(),
                },
            ],
        });
        assert_eq!(value, "Intel GPU (i915) / AMD GPU (amdgpu)");
    }

    #[test]
    fn gpu_vendor_fallback_is_human_readable() {
        assert_eq!(gpu_vendor_name("0x8086"), Some("Intel"));
        assert_eq!(gpu_vendor_name("0x1002"), Some("AMD"));
        assert_eq!(gpu_vendor_name("0x10DE"), Some("NVIDIA"));
        assert_eq!(gpu_vendor_name("0xffff"), None);
    }

    #[test]
    fn display_formats_multiple_connectors() {
        let value = display(&DisplayInfo {
            devices: vec![
                DisplayDevice {
                    connector: "eDP-1".into(),
                    resolution: "1920x1080".into(),
                },
                DisplayDevice {
                    connector: "HDMI-A-1".into(),
                    resolution: "2560x1440".into(),
                },
            ],
        });
        assert_eq!(value, "1920x1080 eDP-1 / 2560x1440 HDMI-A-1");
    }

    #[test]
    fn network_formats_wireless_interface_count() {
        let value = network(&NetworkInfo {
            interfaces: 3,
            wireless: 1,
            up: 2,
        });
        assert_eq!(value, "2 up / 3 interfaces (1 wireless)");
    }

    #[test]
    fn pretty_os_name_takes_precedence() {
        let value = os_label(&OsInfo {
            name: "Linux".into(),
            pretty_name: "Example Linux Rolling".into(),
            version: "2026".into(),
            ..Default::default()
        });
        assert_eq!(value, "Example Linux Rolling");
    }

    #[test]
    fn battery_status_distinguishes_full_from_charging() {
        assert_eq!(
            battery(&BatteryInfo {
                percent: Some(100.0),
                status: BatteryStatus::Full,
            }),
            "100% (full)"
        );
        assert_eq!(
            battery(&BatteryInfo {
                percent: Some(72.0),
                status: BatteryStatus::Charging,
            }),
            "72% (charging)"
        );
        assert_eq!(
            battery(&BatteryInfo {
                percent: Some(72.0),
                status: BatteryStatus::NotCharging,
            }),
            "72% (not charging)"
        );
    }

    #[test]
    fn battery_capacity_never_mixes_energy_and_charge_units() {
        let mixed = [(50.0, Some(100.0), None), (100.0, None, Some(300.0))];
        assert_eq!(aggregate_battery_capacity(&mixed), Some(75.0));

        let energy = [(50.0, Some(100.0), None), (100.0, Some(300.0), None)];
        assert_eq!(aggregate_battery_capacity(&energy), Some(87.5));

        let charge = [(25.0, None, Some(100.0)), (75.0, None, Some(300.0))];
        assert_eq!(aggregate_battery_capacity(&charge), Some(62.5));
        assert_eq!(aggregate_battery_capacity(&[]), None);
    }

    #[test]
    fn battery_without_hardware_is_none() {
        assert_eq!(battery(&BatteryInfo::default()), "none");
    }

    #[test]
    fn os_release_quotes_and_escapes_are_parsed() {
        assert_eq!(parse_os_release_value("\"CachyOS 2026\""), "CachyOS 2026");
        assert_eq!(parse_os_release_value("'CachyOS'"), "CachyOS");
        assert_eq!(parse_os_release_value("\"A\\\"B\""), "A\"B");
        assert_eq!(parse_os_release_value("\"A\\qB\""), "A\\qB");
        assert_eq!(parse_os_release_value("\"A\\$B\""), "A$B");
    }
}
