//! System metrics collection with per-sensor hardware/simulation fallback.
//!
//! [`SystemReader`] probes the host at construction time to discover which
//! sensors are available (CPU, temperature, battery, fan, network). For each
//! missing sensor, [`SimState`] generates plausible simulated values so the
//! app works identically on any platform.

use battery::Manager as BatteryManager;
use sysinfo::{Components, Networks, System};
use std::time::Instant;

/// A single network interface with its name and IPv4 address.
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
}

/// A snapshot of all system metrics at a point in time.
///
/// Each metric has a corresponding `*_real` flag indicating whether the value
/// came from actual hardware or from the simulator.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SystemInfo {
    pub cpu_percent: f32,
    pub temp_celsius: f32,
    pub ram_used_bytes: u64,
    pub ram_total_bytes: u64,
    pub battery_percent: f32,
    pub power_status: String,
    pub fan_rpm: u32,
    pub uptime_secs: u64,
    pub networks: Vec<NetworkInterface>,
    pub cpu_real: bool,
    pub temp_real: bool,
    pub ram_real: bool,
    pub battery_real: bool,
    pub fan_real: bool,
    pub network_real: bool,
}

impl SystemInfo {
    /// Converts `ram_used_bytes` to gigabytes.
    pub fn ram_used_gb(&self) -> f64 {
        self.ram_used_bytes as f64 / 1_073_741_824.0
    }

    /// Converts `ram_total_bytes` to gigabytes.
    pub fn ram_total_gb(&self) -> f64 {
        self.ram_total_bytes as f64 / 1_073_741_824.0
    }

    /// Formats `uptime_secs` as a human-readable string (e.g. "2h 34m").
    pub fn uptime_formatted(&self) -> String {
        let days = self.uptime_secs / 86400;
        let hours = (self.uptime_secs % 86400) / 3600;
        let mins = (self.uptime_secs % 3600) / 60;
        if days > 0 {
            format!("{}d {}h {}m", days, hours, mins)
        } else if hours > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}m", mins)
        }
    }
}

/// Simulated sensor values for platforms missing real hardware.
///
/// Produces smoothly oscillating CPU, temperature, RAM, and battery values
/// that look natural in the UI.
struct SimState {
    cpu_phase: f64,
    temp_current: f32,
    ram_current: f64,
    battery_percent: f32,
    battery_charging: bool,
}

impl SimState {
    fn new() -> Self {
        Self {
            cpu_phase: 0.0,
            temp_current: 55.0,
            ram_current: 3.5,
            battery_percent: 75.0,
            battery_charging: false,
        }
    }

    /// Advances all simulated sensors by `dt_secs`.
    fn tick(&mut self, dt_secs: f64) {
        self.cpu_phase += dt_secs * 0.3;

        // Temperature tracks a synthetic CPU load curve
        let synthetic_cpu = (47.5 + 37.5 * self.cpu_phase.sin()).clamp(10.0, 85.0);
        let target_temp = 40.0 + (synthetic_cpu as f32 / 85.0) * 35.0;
        self.temp_current += (target_temp - self.temp_current) * 0.05;

        self.ram_current += (rand::random::<f64>() - 0.5) * 0.1;
        self.ram_current = self.ram_current.clamp(2.0, 6.0);

        // Battery drains slowly then recharges, cycling endlessly
        if self.battery_charging {
            self.battery_percent += 0.02;
            if self.battery_percent >= 100.0 {
                self.battery_percent = 100.0;
                self.battery_charging = false;
            }
        } else {
            self.battery_percent -= 0.01;
            if self.battery_percent <= 5.0 {
                self.battery_percent = 5.0;
                self.battery_charging = true;
            }
        }
    }

    /// Derives a simulated fan RPM from the current temperature.
    fn fan_rpm(&self) -> u32 {
        let ratio = ((self.temp_current - 40.0) / 35.0).clamp(0.0, 1.0);
        1200 + (ratio * 3300.0) as u32
    }
}

/// Reads system metrics, mixing real hardware sensors with simulated fallbacks.
///
/// This struct is `!Send` because the `battery` crate uses `Rc` internally,
/// so it must live on a dedicated OS thread rather than a tokio task.
pub struct SystemReader {
    sys: System,
    components: Components,
    networks: Networks,
    battery_mgr: Option<BatteryManager>,
    start_time: Instant,
    sim: SimState,
    has_real_temp: bool,
    has_real_battery: bool,
    has_real_fan: bool,
    has_real_network: bool,
    last_tick: Instant,
    last_temp: f32,
    last_battery: (f32, String),
    last_fan: u32,
}

impl SystemReader {
    /// Probes the system for available sensors and initializes fallback state.
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        let components = Components::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        let battery_mgr = BatteryManager::new().ok();
        let has_real_battery = battery_mgr
            .as_ref()
            .and_then(|m| m.batteries().ok())
            .and_then(|mut b| b.next())
            .and_then(|b| b.ok())
            .is_some();

        let has_real_temp = components.iter().next().is_some();
        let has_real_fan = Self::probe_fan_speed().is_some();
        let has_real_network = networks.iter().next().is_some();

        let now = Instant::now();
        Self {
            sys, components, networks, battery_mgr,
            start_time: now, sim: SimState::new(),
            has_real_temp, has_real_battery, has_real_fan, has_real_network,
            last_tick: now,
            last_temp: 50.0,
            last_battery: (50.0, "Unknown".to_string()),
            last_fan: 2000,
        }
    }

    /// Returns a human-readable summary of which sensors are real vs simulated.
    pub fn sensor_status_message(&self) -> String {
        format!(
            "[system] sensors: cpu=real, temp={}, ram=real, battery={}, fan={}, network={}",
            if self.has_real_temp { "real" } else { "sim" },
            if self.has_real_battery { "real" } else { "sim" },
            if self.has_real_fan { "real" } else { "sim" },
            if self.has_real_network { "real" } else { "sim" },
        )
    }

    /// Reads all system metrics, returning a complete [`SystemInfo`] snapshot.
    pub fn read(&mut self) -> SystemInfo {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;
        self.sim.tick(dt);

        self.sys.refresh_cpu_usage();
        let cpu_percent = self.sys.global_cpu_usage();

        self.sys.refresh_memory();
        let ram_used = self.sys.used_memory();
        let ram_total = self.sys.total_memory();

        let temp = if self.has_real_temp {
            self.components.refresh(false);
            let real_temp = self
                .components
                .iter()
                .filter_map(|c| c.temperature())
                .fold(0.0_f32, f32::max);
            if real_temp > 0.0 {
                self.last_temp = real_temp;
                real_temp
            } else {
                tracing::warn!("temperature sensor returned 0, using last known value");
                self.last_temp
            }
        } else {
            self.sim.temp_current
        };

        let (battery_pct, power_status) = if self.has_real_battery {
            let result = self.read_real_battery();
            if result.1 != "Unknown" {
                self.last_battery = result.clone();
            } else {
                tracing::warn!("battery read returned Unknown, using last known value");
            }
            if result.1 == "Unknown" {
                self.last_battery.clone()
            } else {
                result
            }
        } else {
            (
                self.sim.battery_percent,
                if self.sim.battery_charging { "Charging".to_string() } else { "Discharging".to_string() },
            )
        };

        let fan_rpm = if self.has_real_fan {
            match Self::probe_fan_speed() {
                Some(rpm) => {
                    self.last_fan = rpm;
                    rpm
                }
                None => {
                    tracing::warn!("fan sensor read failed, using last known value");
                    self.last_fan
                }
            }
        } else {
            self.sim.fan_rpm()
        };

        let networks = if self.has_real_network {
            self.read_real_networks()
        } else {
            vec![NetworkInterface { name: "wlan0".to_string(), ip: "10.210.25.42".to_string() }]
        };

        let uptime = now.duration_since(self.start_time).as_secs();

        SystemInfo {
            cpu_percent, temp_celsius: temp, ram_used_bytes: ram_used, ram_total_bytes: ram_total,
            battery_percent: battery_pct, power_status, fan_rpm, uptime_secs: uptime, networks,
            cpu_real: true, temp_real: self.has_real_temp, ram_real: true,
            battery_real: self.has_real_battery, fan_real: self.has_real_fan,
            network_real: self.has_real_network,
        }
    }

    /// Reads battery percentage and power state from the real battery manager.
    fn read_real_battery(&self) -> (f32, String) {
        let mgr = match &self.battery_mgr {
            Some(m) => m,
            None => return (self.sim.battery_percent, "Unknown".to_string()),
        };
        match mgr.batteries() {
            Ok(mut batteries) => {
                if let Some(Ok(bat)) = batteries.next() {
                    let pct = bat.state_of_charge().value * 100.0;
                    let status = format!("{:?}", bat.state());
                    (pct, status)
                } else {
                    (self.sim.battery_percent, "Unknown".to_string())
                }
            }
            Err(_) => (self.sim.battery_percent, "Unknown".to_string()),
        }
    }

    /// Enumerates real network interfaces with their IPv4 addresses.
    ///
    /// On Linux, uses `ip --brief addr show` for accurate results.
    /// Falls back to a fake wlan0 interface if none are found.
    fn read_real_networks(&mut self) -> Vec<NetworkInterface> {
        self.networks.refresh(false);
        let mut result: Vec<NetworkInterface> = Vec::new();

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = std::process::Command::new("ip")
                .args(["--brief", "addr", "show"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 && parts[1] == "UP" {
                        for part in &parts[2..] {
                            if part.contains('.') && part.contains('/') {
                                let ip = part.split('/').next().unwrap_or("").to_string();
                                if !ip.starts_with("127.") {
                                    result.push(NetworkInterface { name: parts[0].to_string(), ip });
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            for (name, _) in self.networks.iter() {
                result.push(NetworkInterface { name: name.clone(), ip: "N/A".to_string() });
            }
        }

        if result.is_empty() {
            result.push(NetworkInterface { name: "wlan0".to_string(), ip: "10.210.25.42".to_string() });
        }
        result
    }

    /// Reads fan speed from `/sys/class/hwmon` (Linux only).
    #[cfg(target_os = "linux")]
    fn probe_fan_speed() -> Option<u32> {
        use std::fs;
        let hwmon = fs::read_dir("/sys/class/hwmon").ok()?;
        for entry in hwmon.flatten() {
            let path = entry.path();
            if let Ok(files) = fs::read_dir(&path) {
                for file in files.flatten() {
                    let fname = file.file_name();
                    let fname_str = fname.to_string_lossy();
                    if fname_str.starts_with("fan") && fname_str.ends_with("_input") {
                        if let Ok(val) = fs::read_to_string(file.path()) {
                            if let Ok(rpm) = val.trim().parse::<u32>() {
                                if rpm > 0 { return Some(rpm); }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Fan speed is not available on non-Linux platforms.
    #[cfg(not(target_os = "linux"))]
    fn probe_fan_speed() -> Option<u32> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_formatting() {
        let info = SystemInfo {
            cpu_percent: 45.0, temp_celsius: 55.0,
            ram_used_bytes: 1_288_490_188, ram_total_bytes: 8_053_063_680,
            battery_percent: 72.0, power_status: "Discharging".to_string(),
            fan_rpm: 3200, uptime_secs: 9240, networks: vec![],
            cpu_real: true, temp_real: true, ram_real: true,
            battery_real: true, fan_real: true, network_real: true,
        };
        assert_eq!(info.uptime_formatted(), "2h 34m");
        assert!((info.ram_used_gb() - 1.2).abs() < 0.1);
        assert!((info.ram_total_gb() - 7.5).abs() < 0.1);
    }

    #[test]
    fn test_uptime_with_days() {
        let info = SystemInfo {
            uptime_secs: 90061,
            cpu_percent: 0.0, temp_celsius: 0.0, ram_used_bytes: 0, ram_total_bytes: 0,
            battery_percent: 0.0, power_status: String::new(), fan_rpm: 0, networks: vec![],
            cpu_real: true, temp_real: true, ram_real: true,
            battery_real: true, fan_real: true, network_real: true,
        };
        assert_eq!(info.uptime_formatted(), "1d 1h 1m");
    }

    #[test]
    fn test_sim_state_oscillates() {
        let mut sim = SimState::new();
        let initial_battery = sim.battery_percent;
        for _ in 0..100 { sim.tick(0.2); }
        assert_ne!(sim.battery_percent, initial_battery);
        assert!(sim.temp_current >= 35.0 && sim.temp_current <= 80.0);
    }

    #[test]
    fn test_sim_battery_cycle() {
        let mut sim = SimState::new();
        sim.battery_percent = 6.0;
        sim.battery_charging = false;
        for _ in 0..200 { sim.tick(1.0); }
        assert!(sim.battery_charging || sim.battery_percent > 5.0);
    }

    #[test]
    fn test_system_reader_creates() {
        let _reader = SystemReader::new();
    }

    #[test]
    fn test_system_reader_reads() {
        let mut reader = SystemReader::new();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let info = reader.read();
        assert!(info.cpu_percent >= 0.0 && info.cpu_percent <= 100.0);
        assert!(info.ram_total_bytes > 0);
    }

    #[test]
    fn test_sensor_status_message() {
        let reader = SystemReader::new();
        let msg = reader.sensor_status_message();
        assert!(msg.contains("cpu=real"));
        assert!(msg.contains("ram=real"));
        assert!(msg.contains("temp="));
        assert!(msg.contains("battery="));
    }
}
