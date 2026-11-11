mod os_release;

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    ffi::{CString, OsString},
    fs,
    os::unix::ffi::{OsStrExt, OsStringExt},
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
#[allow(dead_code)]
pub struct CpuInfo {
    pub model: String,
    pub physical_cores: Option<usize>,
    pub threads: usize,
    pub frequency_mhz: Option<u64>,
}

#[derive(Default)]
#[allow(dead_code)]
pub struct TemperatureInfo {
    pub cpu_celsius: Option<f32>,
}

#[derive(Default)]
pub struct MemoryInfo {
    pub total_kb: Option<u64>,
    pub available_kb: Option<u64>,
}

#[derive(Default)]
pub struct SwapInfo {
    pub total_kb: Option<u64>,
    pub free_kb: Option<u64>,
}

#[derive(Default)]
#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct GpuInfo {
    pub devices: Vec<GpuDevice>,
}

#[derive(Default)]
#[allow(dead_code)]
pub struct GpuDevice {
    pub name: String,
    pub driver: String,
}

#[derive(Default)]
#[allow(dead_code)]
pub struct DisplayInfo {
    pub devices: Vec<DisplayDevice>,
}

#[derive(Default)]
pub struct DisplayDevice {
    pub connector: String,
}

#[derive(Default)]
pub struct NetworkInfo {
    pub interfaces: usize,
    pub wireless: usize,
    pub up: usize,
}

fn read_dir_entries(path: &Path) -> Option<Vec<fs::DirEntry>> {
    fs::read_dir(path).ok()?.collect::<Result<Vec<_>, _>>().ok()
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

#[allow(dead_code)]
fn read_gpu() -> GpuInfo {
    let Some(entries) = read_dir_entries(Path::new("/sys/class/drm")) else {
        return GpuInfo::default();
    };
    let mut devices = Vec::new();
    let mut pending_pci_lookups = Vec::new();
    let mut wanted_ids = BTreeSet::new();

    for entry in entries {
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

#[allow(dead_code)]
fn read_display() -> DisplayInfo {
    let Some(entries) = read_dir_entries(Path::new("/sys/class/drm")) else {
        return DisplayInfo::default();
    };
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

        let connector = name
            .split_once('-')
            .map(|(_, connector)| connector.to_owned())
            .unwrap_or(name);

        let device = DisplayDevice { connector };

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

#[allow(dead_code)]
fn read_network() -> NetworkInfo {
    let Some(entries) = read_dir_entries(Path::new("/sys/class/net")) else {
        return NetworkInfo::default();
    };

    let mut result = NetworkInfo::default();

    for entry in entries {
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

#[derive(Default)]
#[allow(dead_code)]
pub struct SystemInfo {
    pub user: String,
    pub hostname: String,
    pub os: OsInfo,
    pub host: HostInfo,
    pub kernel: String,
    pub uptime: Option<f64>,
    pub load_avg: Option<(f64, f64, f64)>,
    pub packages: PackageInfo,
    pub shell: String,
    pub shell_version: String,
    pub desktop: String,
    pub terminal: String,
    pub cpu: CpuInfo,
    pub temperature: TemperatureInfo,
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
    let text = match fs::read_to_string("/etc/os-release") {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::read_to_string("/usr/lib/os-release").unwrap_or_default()
        }
        Err(_) => String::new(),
    };
    let mut o = OsInfo::default();

    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let value = os_release::parse_value(value.trim());

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

#[allow(dead_code)]
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
        model = "unknown".to_owned();
    }

    CpuInfo {
        model,
        physical_cores,
        threads,
        frequency_mhz: read_cpu_frequency_mhz(),
    }
}

fn weighted_frequency_mhz(samples: &[(u64, u128)]) -> Option<u64> {
    let (weighted_total, total_weight) = samples.iter().try_fold(
        (0u128, 0u128),
        |(total, weight_total), (frequency, weight)| {
            if *frequency == 0 || *weight == 0 {
                return None;
            }
            let weighted = u128::from(*frequency).checked_mul(*weight)?;
            Some((
                total.checked_add(weighted)?,
                weight_total.checked_add(*weight)?,
            ))
        },
    )?;

    if total_weight == 0 {
        return None;
    }
    let rounded_khz = weighted_total.checked_add(total_weight.checked_mul(500)?)? / total_weight;
    u64::try_from(rounded_khz / 1000).ok()
}

fn read_cpu_frequency_mhz() -> Option<u64> {
    let root = Path::new("/sys/devices/system/cpu/cpufreq");
    let Some(entries) = read_dir_entries(root) else {
        return read_per_cpu_frequency_mhz();
    };

    let mut policy_paths: Vec<_> = entries
        .into_iter()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("policy"))
        .map(|entry| entry.path())
        .collect();
    policy_paths.sort();

    // affected_cpus is the online CPU set for a policy. Ignore empty/offline
    // policies rather than assigning them a fabricated weight of one.
    let policies: Vec<_> = policy_paths
        .into_iter()
        .filter_map(|path| {
            let weight = parse_cpu_list(&read_path(&path.join("affected_cpus"))).len() as u128;
            (weight > 0).then_some((path, weight))
        })
        .collect();

    if policies.is_empty() {
        return read_per_cpu_frequency_mhz();
    }

    // Prefer hardware-reported averages/current values. scaling_cur_freq is
    // only a last resort because it can represent the last requested P-state.
    for attribute in ["cpuinfo_avg_freq", "cpuinfo_cur_freq", "scaling_cur_freq"] {
        let mut samples = Vec::with_capacity(policies.len());
        let mut complete = true;

        for (policy, weight) in &policies {
            let frequency = read_path(&policy.join(attribute))
                .parse::<u64>()
                .ok()
                .filter(|value| *value > 0);
            let Some(frequency) = frequency else {
                complete = false;
                break;
            };
            samples.push((frequency, *weight));
        }

        if complete {
            if let Some(frequency) = weighted_frequency_mhz(&samples) {
                return Some(frequency);
            }
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

fn parse_temperature_millidegrees(value: &str) -> Option<f32> {
    let millidegrees = value.trim().parse::<i64>().ok()?;
    // Keep the physical sensor range broad enough for cold environments.
    // CPU-specific hwmon drivers and thermal zones provide temperature units.
    if !(0..=150_000).contains(&millidegrees) {
        return None;
    }

    Some(millidegrees as f32 / 1000.0)
}

fn sensor_has_fault(value: &str) -> bool {
    value.trim() == "1"
}

fn is_cpu_hwmon(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        "coretemp",
        "k10temp",
        "zenpower",
        "cpu_thermal",
        "x86_pkg_temp",
        "peci_cputemp",
    ]
    .iter()
    .any(|chip| name.contains(chip))
}

fn temperature_priority(label: &str) -> u8 {
    let label = label.to_ascii_lowercase();
    if label.contains("tdie") {
        0
    } else if label.contains("package") {
        1
    } else if label.contains("tctl") {
        2
    } else if label.contains("cpu") || label.contains("x86_pkg_temp") {
        3
    } else {
        4
    }
}

fn choose_cpu_temperature(readings: &[(String, f32)]) -> Option<f32> {
    let best_priority = readings
        .iter()
        .filter(|(label, _)| !label.to_ascii_lowercase().contains("tctl"))
        .map(|(label, _)| temperature_priority(label))
        .min()?;

    readings
        .iter()
        .filter(|(label, _)| {
            !label.to_ascii_lowercase().contains("tctl")
                && temperature_priority(label) == best_priority
        })
        .map(|(_, temperature)| *temperature)
        .reduce(f32::max)
}

#[allow(dead_code)]
fn read_cpu_temperature() -> TemperatureInfo {
    let mut readings = Vec::new();

    if let Some(chips) = read_dir_entries(Path::new("/sys/class/hwmon")) {
        for chip in chips {
            let chip_path = chip.path();
            let chip_name = read_path(&chip_path.join("name"));
            if !is_cpu_hwmon(&chip_name) {
                continue;
            }

            let Some(attributes) = read_dir_entries(&chip_path) else {
                continue;
            };

            for attribute in attributes {
                let file_name = attribute.file_name().to_string_lossy().into_owned();
                let Some(index) = file_name
                    .strip_prefix("temp")
                    .and_then(|name| name.strip_suffix("_input"))
                else {
                    continue;
                };
                if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
                    continue;
                }

                let sensor_prefix = format!("temp{index}");
                if sensor_has_fault(&read_path(
                    &chip_path.join(format!("{sensor_prefix}_fault")),
                )) {
                    continue;
                }

                let Some(temperature) =
                    parse_temperature_millidegrees(&read_path(&attribute.path()))
                else {
                    continue;
                };
                let label = read_path(&chip_path.join(format!("{sensor_prefix}_label")));
                let label = if label.is_empty() && chip_name == "k10temp" && index == "1" {
                    "Tctl".to_owned()
                } else if label.is_empty() {
                    format!("{chip_name} temp{index}")
                } else {
                    label
                };
                readings.push((label, temperature));
            }
        }
    }

    if choose_cpu_temperature(&readings).is_none() {
        if let Some(zones) = read_dir_entries(Path::new("/sys/class/thermal")) {
            for zone in zones {
                let name = zone.file_name().to_string_lossy().into_owned();
                if !name.starts_with("thermal_zone") {
                    continue;
                }

                let label = read_path(&zone.path().join("type"));
                let lower = label.to_ascii_lowercase();
                if !(lower.contains("cpu")
                    || lower.contains("package")
                    || lower.contains("x86_pkg_temp"))
                {
                    continue;
                }

                if let Some(temperature) =
                    parse_temperature_millidegrees(&read_path(&zone.path().join("temp")))
                {
                    readings.push((label, temperature));
                }
            }
        }
    }

    TemperatureInfo {
        cpu_celsius: choose_cpu_temperature(&readings),
    }
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

fn parse_meminfo_value(value: &str) -> Option<u64> {
    let mut fields = value.split_whitespace();
    let amount = fields.next()?.parse::<u64>().ok()?;
    (fields.next()? == "kB" && fields.next().is_none()).then_some(amount)
}

fn parse_meminfo(text: &str) -> (MemoryInfo, SwapInfo) {
    let mut memory = MemoryInfo::default();
    let mut swap = SwapInfo::default();

    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let parsed = parse_meminfo_value(value);
        match key {
            "MemTotal" => memory.total_kb = parsed,
            "MemAvailable" => memory.available_kb = parsed,
            "SwapTotal" => swap.total_kb = parsed,
            "SwapFree" => swap.free_kb = parsed,
            _ => {}
        }
    }

    (memory, swap)
}

fn read_memory_and_swap() -> (MemoryInfo, SwapInfo) {
    let text = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    parse_meminfo(&text)
}

fn parse_uptime_seconds(text: &str) -> Option<f64> {
    let seconds = text.split_whitespace().next()?.parse::<f64>().ok()?;
    (seconds.is_finite() && seconds >= 0.0).then_some(seconds)
}

fn parse_load_average(text: &str) -> Option<(f64, f64, f64)> {
    let mut fields = text.split_whitespace();
    let values = (
        fields.next()?.parse::<f64>().ok()?,
        fields.next()?.parse::<f64>().ok()?,
        fields.next()?.parse::<f64>().ok()?,
    );

    [values.0, values.1, values.2]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
        .then_some(values)
}

#[allow(dead_code)]
fn read_load_avg() -> Option<(f64, f64, f64)> {
    parse_load_average(&read("/proc/loadavg"))
}

fn count_package_directories(path: &Path) -> Option<usize> {
    fs::read_dir(path).ok()?.try_fold(0usize, |count, entry| {
        let entry = entry.ok()?;
        let is_directory = entry.file_type().ok()?.is_dir();
        if is_directory {
            count.checked_add(1)
        } else {
            Some(count)
        }
    })
}

fn count_dpkg_installed(text: &str) -> usize {
    let mut count = 0usize;
    let mut installed = false;

    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() {
            if installed {
                count += 1;
                installed = false;
            }
        } else if line == "Status: install ok installed" {
            installed = true;
        }
    }

    if installed {
        count += 1;
    }
    count
}

fn count_apk_packages(text: &str) -> usize {
    text.lines()
        .filter_map(|line| line.strip_prefix("P:"))
        .filter(|name| !name.trim().is_empty())
        .count()
}

fn read_packages() -> PackageInfo {
    let pacman_db = Path::new("/var/lib/pacman/local");
    if pacman_db.exists() {
        return PackageInfo {
            manager: "pacman".into(),
            count: count_package_directories(pacman_db),
        };
    }

    let dpkg_status = Path::new("/var/lib/dpkg/status");
    if dpkg_status.exists() {
        return PackageInfo {
            manager: "dpkg".into(),
            count: fs::read_to_string(dpkg_status)
                .ok()
                .map(|text| count_dpkg_installed(&text)),
        };
    }

    let apk_db = Path::new("/lib/apk/db/installed");
    if apk_db.exists() {
        return PackageInfo {
            manager: "apk".into(),
            count: fs::read_to_string(apk_db)
                .ok()
                .map(|text| count_apk_packages(&text)),
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

#[allow(dead_code)]
fn read_battery() -> BatteryInfo {
    let Some(entries) = read_dir_entries(Path::new("/sys/class/power_supply")) else {
        return BatteryInfo::default();
    };

    let mut batteries = Vec::new();
    let mut any_charging = false;
    let mut any_discharging = false;
    let mut any_not_charging = false;
    let mut all_full = true;

    for entry in entries {
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
                .and_then(|v| v.trim().parse::<f64>().ok())
                .filter(|value| value.is_finite() && *value > 0.0);
            let charge_full = fs::read_to_string(base.join("charge_full"))
                .ok()
                .and_then(|v| v.trim().parse::<f64>().ok())
                .filter(|value| value.is_finite() && *value > 0.0);

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

fn aggregate_battery_capacity(batteries: &[(f32, Option<f64>, Option<f64>)]) -> Option<f32> {
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
    batteries: &[(f32, Option<f64>, Option<f64>)],
    use_energy: bool,
) -> Option<f32> {
    let weights = batteries
        .iter()
        .map(|battery| if use_energy { battery.1 } else { battery.2 })
        .collect::<Option<Vec<_>>>()?;
    let total_weight: f64 = weights.iter().sum();

    if !total_weight.is_finite() || total_weight <= 0.0 {
        return None;
    }

    let weighted_percent: f64 = batteries
        .iter()
        .zip(weights.iter())
        .map(|(battery, weight)| f64::from(battery.0) * weight)
        .sum();
    let percent = weighted_percent / total_weight;

    percent.is_finite().then_some(percent as f32)
}

fn decode_mount_field(value: &[u8]) -> Option<PathBuf> {
    let mut decoded = Vec::with_capacity(value.len());
    let mut index = 0;

    while index < value.len() {
        if value[index] == b'\\' {
            if index + 3 >= value.len() {
                return None;
            }

            let escape = &value[index + 1..index + 4];
            let decoded_byte = match escape {
                b"040" => b' ',
                b"011" => b'\t',
                b"012" => b'\n',
                b"134" => b'\\',
                _ => return None,
            };
            decoded.push(decoded_byte);
            index += 4;
        } else {
            if value[index] == 0 {
                return None;
            }
            decoded.push(value[index]);
            index += 1;
        }
    }

    Some(PathBuf::from(OsString::from_vec(decoded)))
}

#[cfg(test)]
fn parse_mountinfo(text: &str) -> Vec<MountEntry> {
    parse_mountinfo_bytes(text.as_bytes())
}

fn parse_mountinfo_bytes(text: &[u8]) -> Vec<MountEntry> {
    let mut mounts = Vec::new();

    for line in text.split(|byte| *byte == b'\n') {
        let fields: Vec<_> = line
            .split(|byte| byte.is_ascii_whitespace())
            .filter(|field| !field.is_empty())
            .collect();
        let Some(separator) = fields.iter().position(|field| *field == b"-") else {
            continue;
        };

        if separator < 6 || fields.len() < separator + 4 {
            continue;
        }

        let device = fields[2];
        let Some(colon) = device.iter().position(|byte| *byte == b':') else {
            continue;
        };
        let (major, minor_with_colon) = device.split_at(colon);
        let minor = &minor_with_colon[1..];
        if major.is_empty()
            || minor.is_empty()
            || !major.iter().all(|byte| byte.is_ascii_digit())
            || !minor.iter().all(|byte| byte.is_ascii_digit())
            || std::str::from_utf8(major)
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .is_none()
            || std::str::from_utf8(minor)
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .is_none()
        {
            continue;
        }

        let Ok(device) = std::str::from_utf8(device) else {
            continue;
        };
        let Ok(fs_type) = std::str::from_utf8(fields[separator + 1]) else {
            continue;
        };
        let Some(mount_point) = decode_mount_field(fields[4]) else {
            continue;
        };

        if is_pseudo_filesystem(fs_type) || is_nonlocal_or_stacked_filesystem(fs_type) {
            continue;
        }

        mounts.push(MountEntry {
            device: device.to_owned(),
            mount_point,
        });
    }

    mounts
}

fn is_nonlocal_or_stacked_filesystem(fs_type: &str) -> bool {
    matches!(
        fs_type,
        "nfs"
            | "nfs4"
            | "cifs"
            | "smb3"
            | "smbfs"
            | "ncpfs"
            | "9p"
            | "ceph"
            | "afs"
            | "coda"
            | "glusterfs"
            | "lustre"
            | "davfs"
            | "orangefs"
            | "overlay"
            | "fuse"
    ) || fs_type.starts_with("fuse.")
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
        u64::try_from(u128::from(stat.f_frsize)).ok()?
    } else {
        u64::try_from(u128::from(stat.f_bsize)).ok()?
    };
    let total =
        u64::try_from(u128::from(stat.f_blocks).checked_mul(u128::from(block_size))?).ok()?;
    let available = u64::try_from(u128::from(stat.f_bavail).checked_mul(u128::from(block_size))?)
        .ok()?
        .min(total);

    (total > 0).then_some((total, available))
}

#[allow(dead_code)]
fn read_disk() -> DiskInfo {
    let Ok(mountinfo) = fs::read("/proc/self/mountinfo") else {
        return DiskInfo::default();
    };

    let mut seen_devices = BTreeSet::new();
    let mut result = DiskInfo::default();

    for mount in parse_mountinfo_bytes(&mountinfo) {
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
        return DiskInfo::default();
    }

    result.available_bytes = result.available_bytes.min(result.total_bytes);
    result
}

pub fn collect() -> SystemInfo {
    // The default interface intentionally displays only this compact set.
    // Do not probe hidden hardware modules on the default path: several of
    // them enumerate sysfs or mounted filesystems and add work without
    // contributing anything to the rendered output.
    let (memory, _) = read_memory_and_swap();

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
        uptime: parse_uptime_seconds(&read("/proc/uptime")),
        packages: read_packages(),
        memory,
        ..SystemInfo::default()
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

pub fn memory_compact(m: &MemoryInfo) -> String {
    let Some(total) = m.total_kb.filter(|total| *total > 0) else {
        return "unknown".into();
    };
    let total_mb = total / 1024;
    let Some(available) = m.available_kb else {
        return format!("unknown / {total_mb}M");
    };
    let used_mb = total.saturating_sub(available.min(total)) / 1024;
    format!("{used_mb}M / {total_mb}M")
}

#[allow(dead_code)]
pub fn memory(m: &MemoryInfo) -> String {
    let Some(total) = m.total_kb.filter(|total| *total > 0) else {
        return "unknown".into();
    };

    let total_gib = total as f64 / 1_048_576.0;
    let Some(available) = m.available_kb else {
        return format!("{total_gib:.1} GiB total (usage unavailable)");
    };
    let available = available.min(total);
    let used = total.saturating_sub(available);
    let percent = used as f64 / total as f64 * 100.0;

    format!(
        "{:.1} GiB / {:.1} GiB ({:.0}%)",
        used as f64 / 1_048_576.0,
        total_gib,
        percent
    )
}

#[allow(dead_code)]
pub fn swap(s: &SwapInfo) -> String {
    let Some(total) = s.total_kb else {
        return "unknown".into();
    };
    if total == 0 {
        return "none".into();
    }

    let total_gib = total as f64 / 1_048_576.0;
    let Some(free) = s.free_kb else {
        return format!("{total_gib:.1} GiB total (usage unavailable)");
    };
    let free = free.min(total);
    let used = total.saturating_sub(free);

    format!(
        "{:.1} GiB / {:.1} GiB ({:.0}%)",
        used as f64 / 1_048_576.0,
        total_gib,
        used as f64 / total as f64 * 100.0
    )
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn display(d: &DisplayInfo) -> String {
    if d.devices.is_empty() {
        return "none".into();
    }

    d.devices
        .iter()
        .map(|device| device.connector.clone())
        .collect::<Vec<_>>()
        .join(" / ")
}

#[allow(dead_code)]
pub fn network(n: &NetworkInfo) -> String {
    if n.interfaces == 0 {
        return "none".into();
    }

    format!(
        "{} up / {} interfaces ({} wireless)",
        n.up, n.interfaces, n.wireless
    )
}

#[allow(dead_code)]
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
    fn cpu_temperature_parser_rejects_invalid_values() {
        assert_eq!(parse_temperature_millidegrees("42000"), Some(42.0));
        assert_eq!(parse_temperature_millidegrees("150001"), None);
        assert_eq!(parse_temperature_millidegrees("1500"), Some(1.5));
        assert_eq!(parse_temperature_millidegrees("not-a-number"), None);
    }

    #[test]
    fn cpu_temperature_does_not_mislabel_tctl_as_physical_die_temperature() {
        assert_eq!(choose_cpu_temperature(&[("Tctl".into(), 80.0)]), None);
        let readings = vec![("Tctl".into(), 80.0), ("Tdie".into(), 72.0)];
        assert_eq!(choose_cpu_temperature(&readings), Some(72.0));
    }

    #[test]
    fn cpu_temperature_prefers_die_or_package_over_core_readings() {
        let readings = vec![
            ("Core 0".into(), 70.0),
            ("Core 1".into(), 72.0),
            ("Package id 0".into(), 65.0),
            ("Tctl".into(), 68.0),
        ];
        assert_eq!(choose_cpu_temperature(&readings), Some(65.0));

        let amd = vec![("Tctl".into(), 80.0), ("Tdie".into(), 72.0)];
        assert_eq!(choose_cpu_temperature(&amd), Some(72.0));
        let core_fallback = vec![("Tctl".into(), 80.0), ("Core 0".into(), 72.0)];
        assert_eq!(choose_cpu_temperature(&core_fallback), Some(72.0));
        let thermal_fallback = vec![("Tctl".into(), 80.0), ("x86_pkg_temp".into(), 69.0)];
        assert_eq!(choose_cpu_temperature(&thermal_fallback), Some(69.0));
    }

    #[test]
    fn cpu_hwmon_filter_excludes_unrelated_chips() {
        assert!(is_cpu_hwmon("coretemp"));
        assert!(is_cpu_hwmon("k10temp"));
        assert!(is_cpu_hwmon("zenpower"));
        assert!(!is_cpu_hwmon("amdgpu"));
        assert!(!is_cpu_hwmon("nvme"));
    }

    #[test]
    fn cpu_frequency_policy_average_uses_online_cpu_weights() {
        assert_eq!(
            weighted_frequency_mhz(&[(3_000_000, 2), (1_000_000, 1)]),
            Some(2_333)
        );
        assert_eq!(weighted_frequency_mhz(&[(3_000_000, 0)]), None);
        assert_eq!(weighted_frequency_mhz(&[]), None);
        assert_eq!(weighted_frequency_mhz(&[(u64::MAX, u128::MAX)]), None);
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
    fn meminfo_parser_preserves_missing_and_rejects_malformed_values() {
        assert_eq!(parse_meminfo_value("1024 kB"), Some(1024));
        assert_eq!(parse_meminfo_value("1024 MB"), None);
        assert_eq!(parse_meminfo_value("not-a-number kB"), None);
        assert_eq!(parse_meminfo_value("1024 kB trailing"), None);

        let fixture = "MemTotal: 8192 kB\nSwapTotal: 0 kB\nSwapFree: 0 kB\n";
        let (memory, swap) = parse_meminfo(fixture);
        assert_eq!(memory.total_kb, Some(8192));
        assert_eq!(memory.available_kb, None);
        assert_eq!(swap.total_kb, Some(0));
        assert_eq!(swap.free_kb, Some(0));
    }

    #[test]
    fn compact_memory_uses_integer_megabytes_and_clamps_available_memory() {
        assert_eq!(
            memory_compact(&MemoryInfo {
                total_kb: Some(15_985_772),
                available_kb: Some(11_829_356),
            }),
            "4059M / 15611M"
        );
        assert_eq!(
            memory_compact(&MemoryInfo {
                total_kb: Some(1024),
                available_kb: Some(2048),
            }),
            "0M / 1M"
        );
    }

    #[test]
    fn compact_memory_keeps_missing_usage_unknown() {
        assert_eq!(
            memory_compact(&MemoryInfo {
                total_kb: Some(1024),
                available_kb: None,
            }),
            "unknown / 1M"
        );
        assert_eq!(memory_compact(&MemoryInfo::default()), "unknown");
    }

    #[test]
    fn memory_and_swap_do_not_report_missing_values_as_zero_usage() {
        assert_eq!(
            memory(&MemoryInfo {
                total_kb: Some(1_048_576),
                available_kb: None
            }),
            "1.0 GiB total (usage unavailable)"
        );
        assert_eq!(
            swap(&SwapInfo {
                total_kb: Some(1_048_576),
                free_kb: None
            }),
            "1.0 GiB total (usage unavailable)"
        );
        assert_eq!(swap(&SwapInfo::default()), "unknown");
    }

    #[test]
    fn cpu_temperature_accepts_zero_and_checks_upper_bound() {
        assert_eq!(parse_temperature_millidegrees("0"), Some(0.0));
        assert_eq!(parse_temperature_millidegrees("150000"), Some(150.0));
        assert_eq!(parse_temperature_millidegrees("150001"), None);
    }

    #[test]
    fn cpu_hwmon_filter_includes_peci_cpu_sensors() {
        assert!(is_cpu_hwmon("peci_cputemp"));
    }

    #[test]
    fn hwmon_fault_flag_marks_sensor_unreliable() {
        assert!(sensor_has_fault("1"));
        assert!(!sensor_has_fault("0"));
        assert!(!sensor_has_fault(""));
    }

    #[test]
    fn load_average_parser_does_not_shift_malformed_fields() {
        assert_eq!(
            parse_load_average("0.10 0.20 0.30 1/100 1234"),
            Some((0.10, 0.20, 0.30))
        );
        assert_eq!(parse_load_average("bad 0.20 0.30"), None);
        assert_eq!(parse_load_average("0.10 bad 0.30"), None);
        assert_eq!(parse_load_average("0.10 0.20"), None);
        assert_eq!(parse_load_average("-0.1 0.2 0.3"), None);
        assert_eq!(parse_load_average("NaN 0.2 0.3"), None);
    }

    #[test]
    fn uptime_parser_rejects_missing_and_invalid_values() {
        assert_eq!(parse_uptime_seconds("123.5 456.0"), Some(123.5));
        assert_eq!(parse_uptime_seconds("0 0"), Some(0.0));
        assert_eq!(parse_uptime_seconds(""), None);
        assert_eq!(parse_uptime_seconds("invalid 0"), None);
        assert_eq!(parse_uptime_seconds("-1 0"), None);
        assert_eq!(parse_uptime_seconds("NaN 0"), None);
        assert_eq!(parse_uptime_seconds("inf 0"), None);
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
            total_kb: Some(1_048_576),
            available_kb: Some(2_000_000),
        });
        assert_eq!(value, "0.0 GiB / 1.0 GiB (0%)");
    }

    #[test]
    fn swap_handles_missing_and_clamped_values() {
        assert_eq!(
            swap(&SwapInfo {
                total_kb: Some(0),
                free_kb: Some(0)
            }),
            "none"
        );
        assert_eq!(
            swap(&SwapInfo {
                total_kb: Some(1_048_576),
                free_kb: Some(2_000_000),
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
        let decoded = decode_mount_field(br"/mnt/a\040b\011c\012d\134e").unwrap();
        assert_eq!(decoded.as_os_str().as_bytes(), b"/mnt/a b\tc\nd\\e");
        assert_eq!(decode_mount_field(br"/mnt/bad\777"), None);
        assert_eq!(decode_mount_field(br"/mnt/bad\04x"), None);
    }

    #[test]
    fn mountinfo_parser_preserves_non_utf8_mount_paths() {
        let mountinfo = b"41 25 8:4 / /mnt/\xffdisk rw - ext4 /dev/sda4 rw\n";
        let mounts = parse_mountinfo_bytes(mountinfo);
        assert_eq!(mounts.len(), 1);
        assert_eq!(
            mounts[0].mount_point.as_os_str().as_bytes(),
            b"/mnt/\xffdisk"
        );
    }

    #[test]
    fn mountinfo_parser_rejects_unknown_escape_sequences() {
        let mountinfo = "42 25 8:5 / /mnt/bad\\777 rw - ext4 /dev/sda5 rw\n";
        assert!(parse_mountinfo(mountinfo).is_empty());
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
    fn storage_filter_excludes_remote_userspace_and_overlay_mounts() {
        for fs_type in [
            "nfs",
            "nfs4",
            "cifs",
            "smb3",
            "9p",
            "ceph",
            "afs",
            "glusterfs",
            "lustre",
            "davfs",
            "orangefs",
            "fuse",
            "fuse.sshfs",
            "fuse.rclone",
            "overlay",
        ] {
            assert!(is_nonlocal_or_stacked_filesystem(fs_type), "{fs_type}");
        }
        assert!(!is_nonlocal_or_stacked_filesystem("ext4"));
        assert!(!is_nonlocal_or_stacked_filesystem("btrfs"));
        assert!(!is_nonlocal_or_stacked_filesystem("fuseblk"));
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
                },
                DisplayDevice {
                    connector: "HDMI-A-1".into(),
                },
            ],
        });
        assert_eq!(value, "eDP-1 / HDMI-A-1");
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
    fn pacman_package_counter_counts_only_complete_directory_entries() {
        let root =
            std::env::temp_dir().join(format!("praxfetch-pacman-db-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("alpha-1.0-1")).unwrap();
        fs::create_dir_all(root.join("beta-2.0-1")).unwrap();
        fs::write(root.join("db.lck"), "").unwrap();

        assert_eq!(count_package_directories(&root), Some(2));
        assert_eq!(
            count_package_directories(&root.join("does-not-exist")),
            None
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dpkg_package_counter_handles_lf_crlf_and_incomplete_stanzas() {
        let fixture = "Package: alpha\nStatus: install ok installed\n\nPackage: beta\nStatus: deinstall ok config-files\n\nPackage: gamma\nStatus: install ok installed";
        assert_eq!(count_dpkg_installed(fixture), 2);

        let crlf = "Package: alpha\r\nStatus: install ok installed\r\n\r\nPackage: beta\r\nStatus: install ok installed\r\n";
        assert_eq!(count_dpkg_installed(crlf), 2);
        assert_eq!(count_dpkg_installed("malformed stanza without status"), 0);
    }

    #[test]
    fn apk_package_counter_requires_a_nonempty_package_name() {
        let fixture = "C:Q1\nP:busybox\nV:1.0\n\nP:musl\nV:1.2\nP:\n";
        assert_eq!(count_apk_packages(fixture), 2);
        assert_eq!(count_apk_packages("not an installed package"), 0);
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
        let invalid_weight = [(50.0, Some(f64::INFINITY), None)];
        assert_eq!(aggregate_battery_capacity(&invalid_weight), None);
    }

    #[test]
    fn battery_without_hardware_is_none() {
        assert_eq!(battery(&BatteryInfo::default()), "none");
    }


}
