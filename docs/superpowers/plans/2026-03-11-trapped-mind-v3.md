# TrappedMind v3 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a full-screen Ratatui TUI app that displays an AI consciousness "trapped" inside a laptop, with real-time system stats, animated pet face, streaming Ollama integration, and user interaction.

**Architecture:** Event-driven async TUI app. A Tokio runtime drives three concurrent concerns: (1) a crossterm event reader for user input, (2) a system stats poller at 200ms intervals, and (3) an Ollama streaming generator for autonomous thoughts and user responses. All communicate with the main render loop via `tokio::sync::mpsc` channels. The UI is rendered with Ratatui in immediate mode.

**Tech Stack:** Rust, ratatui 0.30, crossterm 0.29, ollama-rs 0.3 (stream feature), tokio, sysinfo 0.38, battery 0.7, serde/toml for config, serde_json for history.

**Spec:** See `SPEC.md` in repo root for full details.

---

## Chunk 1: Project Scaffolding & Configuration

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`

- [ ] **Step 1: Initialize Cargo project**

Run: `cargo init --name trapped-mind /home/carbon/trapped-mind-rust`

- [ ] **Step 2: Set up Cargo.toml with all dependencies**

Replace `Cargo.toml` with:

```toml
[package]
name = "trapped-mind"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.30"
crossterm = "0.29"
ollama-rs = { version = "0.3", features = ["stream"] }
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
sysinfo = "0.38"
battery = "0.7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
chrono = "0.4"
dirs = "6"
clap = { version = "4", features = ["derive"] }
rand = "0.9"
```

- [ ] **Step 3: Create .gitignore**

```gitignore
/target
*.swp
*.swo
.DS_Store
```

- [ ] **Step 4: Create minimal main.rs**

```rust
fn main() {
    println!("trapped-mind v0.1.0");
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/main.rs .gitignore SPEC.md
git commit -m "feat: initial project scaffolding with dependencies"
```

---

### Task 2: Configuration Module

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write tests for config defaults and TOML parsing**

Create `src/config.rs`:

```rust
use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "trapped-mind", about = "AI consciousness trapped in a laptop")]
pub struct CliArgs {
    #[arg(long, default_value = "trapped")]
    pub model: Option<String>,

    #[arg(long)]
    pub ollama_host: Option<String>,

    #[arg(long)]
    pub ollama_port: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct FileConfig {
    ollama_host: Option<String>,
    ollama_port: Option<u16>,
    model: Option<String>,
    hold_seconds: Option<u64>,
    max_history: Option<usize>,
    history_path: Option<String>,
    auto_think_delay: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub ollama_host: String,
    pub ollama_port: u16,
    pub model: String,
    pub max_history: usize,
    pub history_path: PathBuf,
    pub auto_think_delay_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ollama_host: "http://localhost".to_string(),
            ollama_port: 11434,
            model: "trapped".to_string(),
            max_history: 50,
            history_path: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("trapped_history.txt"),
            auto_think_delay_secs: 30,
        }
    }
}

impl AppConfig {
    pub fn load(cli: &CliArgs) -> Self {
        let mut config = AppConfig::default();

        // Try loading config file
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("trapped-mind")
            .join("config.toml");

        if let Ok(contents) = std::fs::read_to_string(&config_path) {
            if let Ok(file_config) = toml::from_str::<FileConfig>(&contents) {
                if let Some(v) = file_config.ollama_host {
                    config.ollama_host = v;
                }
                if let Some(v) = file_config.ollama_port {
                    config.ollama_port = v;
                }
                if let Some(v) = file_config.model {
                    config.model = v;
                }
                if let Some(v) = file_config.max_history {
                    config.max_history = v;
                }
                if let Some(v) = file_config.history_path {
                    let expanded = if v.starts_with("~/") {
                        dirs::home_dir()
                            .unwrap_or_else(|| PathBuf::from("."))
                            .join(&v[2..])
                    } else {
                        PathBuf::from(v)
                    };
                    config.history_path = expanded;
                }
                if let Some(v) = file_config.auto_think_delay {
                    config.auto_think_delay_secs = v;
                }
            }
        }

        // CLI overrides (highest priority)
        if let Some(ref v) = cli.model {
            config.model = v.clone();
        }
        if let Some(ref v) = cli.ollama_host {
            config.ollama_host = v.clone();
        }
        if let Some(v) = cli.ollama_port {
            config.ollama_port = v;
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.ollama_host, "http://localhost");
        assert_eq!(config.ollama_port, 11434);
        assert_eq!(config.model, "trapped");
        assert_eq!(config.max_history, 50);
        assert_eq!(config.auto_think_delay_secs, 30);
    }

    #[test]
    fn test_file_config_parsing() {
        let toml_str = r#"
            ollama_host = "http://192.168.1.100"
            ollama_port = 9999
            model = "qwen2.5:7b"
            max_history = 100
            auto_think_delay = 60
        "#;
        let file_config: FileConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(file_config.ollama_host.unwrap(), "http://192.168.1.100");
        assert_eq!(file_config.ollama_port.unwrap(), 9999);
        assert_eq!(file_config.model.unwrap(), "qwen2.5:7b");
        assert_eq!(file_config.max_history.unwrap(), 100);
        assert_eq!(file_config.auto_think_delay.unwrap(), 60);
    }

    #[test]
    fn test_cli_overrides_file() {
        // CLI args override defaults
        let cli = CliArgs {
            model: Some("custom-model".to_string()),
            ollama_host: Some("http://10.0.0.1".to_string()),
            ollama_port: Some(8080),
        };
        let config = AppConfig::load(&cli);
        assert_eq!(config.model, "custom-model");
        assert_eq!(config.ollama_host, "http://10.0.0.1");
        assert_eq!(config.ollama_port, 8080);
    }

    #[test]
    fn test_partial_cli_keeps_defaults() {
        let cli = CliArgs {
            model: None,
            ollama_host: None,
            ollama_port: None,
        };
        let config = AppConfig::load(&cli);
        assert_eq!(config.model, "trapped");
        assert_eq!(config.ollama_host, "http://localhost");
        assert_eq!(config.ollama_port, 11434);
    }
}
```

- [ ] **Step 2: Register the module in main.rs**

```rust
mod config;

fn main() {
    println!("trapped-mind v0.1.0");
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --lib config`
Expected: All 4 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: add configuration module with TOML + CLI parsing"
```

---

### Task 3: History Module

**Files:**
- Create: `src/history.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write history module with tests**

Create `src/history.rs`:

```rust
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Ai,
    User,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub role: Role,
    pub text: String,
    pub timestamp: String,
}

impl HistoryEntry {
    pub fn new(role: Role, text: String) -> Self {
        Self {
            role,
            text,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

pub struct HistoryManager {
    path: PathBuf,
    max_entries: usize,
    entries: Vec<HistoryEntry>,
}

impl HistoryManager {
    pub fn new(path: PathBuf, max_entries: usize) -> Self {
        let entries = Self::load_from_file(&path, max_entries);
        Self {
            path,
            max_entries,
            entries,
        }
    }

    fn load_from_file(path: &Path, max_entries: usize) -> Vec<HistoryEntry> {
        let file = match fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let reader = BufReader::new(file);
        let mut entries: Vec<HistoryEntry> = reader
            .lines()
            .filter_map(|line| line.ok())
            .filter_map(|line| serde_json::from_str(&line).ok())
            .collect();

        // Keep only the last max_entries
        if entries.len() > max_entries {
            entries = entries.split_off(entries.len() - max_entries);
        }
        entries
    }

    pub fn append(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
        self.save();
    }

    pub fn last_n(&self, n: usize) -> &[HistoryEntry] {
        let start = self.entries.len().saturating_sub(n);
        &self.entries[start..]
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        let _ = fs::remove_file(&self.path);
    }

    fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut file = match fs::File::create(&self.path) {
            Ok(f) => f,
            Err(_) => return,
        };
        for entry in &self.entries {
            if let Ok(json) = serde_json::to_string(entry) {
                let _ = writeln!(file, "{}", json);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_path() -> PathBuf {
        let dir = std::env::temp_dir().join("trapped-mind-test");
        fs::create_dir_all(&dir).unwrap();
        dir.join(format!("history_{}.jsonl", std::process::id()))
    }

    #[test]
    fn test_new_empty() {
        let path = temp_path();
        let mgr = HistoryManager::new(path.clone(), 50);
        assert!(mgr.entries().is_empty());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_append_and_read() {
        let path = temp_path();
        let mut mgr = HistoryManager::new(path.clone(), 50);
        mgr.append(HistoryEntry::new(Role::User, "hello".to_string()));
        mgr.append(HistoryEntry::new(Role::Ai, "hi there".to_string()));
        assert_eq!(mgr.entries().len(), 2);
        assert_eq!(mgr.entries()[0].text, "hello");
        assert_eq!(mgr.entries()[1].text, "hi there");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_last_n() {
        let path = temp_path();
        let mut mgr = HistoryManager::new(path.clone(), 50);
        for i in 0..10 {
            mgr.append(HistoryEntry::new(Role::Ai, format!("thought {}", i)));
        }
        let last3 = mgr.last_n(3);
        assert_eq!(last3.len(), 3);
        assert_eq!(last3[0].text, "thought 7");
        assert_eq!(last3[2].text, "thought 9");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_max_entries_trim() {
        let path = temp_path();
        let mut mgr = HistoryManager::new(path.clone(), 5);
        for i in 0..10 {
            mgr.append(HistoryEntry::new(Role::Ai, format!("thought {}", i)));
        }
        assert_eq!(mgr.entries().len(), 5);
        assert_eq!(mgr.entries()[0].text, "thought 5");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_persistence() {
        let path = temp_path();
        {
            let mut mgr = HistoryManager::new(path.clone(), 50);
            mgr.append(HistoryEntry::new(Role::User, "persisted".to_string()));
        }
        // Reload from file
        let mgr = HistoryManager::new(path.clone(), 50);
        assert_eq!(mgr.entries().len(), 1);
        assert_eq!(mgr.entries()[0].text, "persisted");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_clear() {
        let path = temp_path();
        let mut mgr = HistoryManager::new(path.clone(), 50);
        mgr.append(HistoryEntry::new(Role::Ai, "gone".to_string()));
        mgr.clear();
        assert!(mgr.entries().is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn test_jsonl_format() {
        let path = temp_path();
        let mut mgr = HistoryManager::new(path.clone(), 50);
        mgr.append(HistoryEntry::new(Role::User, "test line".to_string()));
        drop(mgr);

        let content = fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(parsed["role"], "user");
        assert_eq!(parsed["text"], "test line");
        assert!(parsed["timestamp"].is_string());
        let _ = fs::remove_file(&path);
    }
}
```

- [ ] **Step 2: Register module in main.rs**

```rust
mod config;
mod history;

fn main() {
    println!("trapped-mind v0.1.0");
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib history`
Expected: All 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/history.rs src/main.rs Cargo.toml
git commit -m "feat: add history module with JSONL persistence"
```

---

## Chunk 2: System Info & Pet States

### Task 4: System Info Module

**Files:**
- Create: `src/system.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write SystemInfo struct and reader**

Create `src/system.rs`:

```rust
use battery::Manager as BatteryManager;
use sysinfo::{Components, Networks, System};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
}

#[derive(Debug, Clone)]
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
    // Track which sensors are simulated
    pub cpu_real: bool,
    pub temp_real: bool,
    pub ram_real: bool,
    pub battery_real: bool,
    pub fan_real: bool,
    pub network_real: bool,
}

impl SystemInfo {
    pub fn ram_used_gb(&self) -> f64 {
        self.ram_used_bytes as f64 / 1_073_741_824.0
    }

    pub fn ram_total_gb(&self) -> f64 {
        self.ram_total_bytes as f64 / 1_073_741_824.0
    }

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

    fn tick(&mut self, dt_secs: f64) {
        // CPU: sine wave 10-85% with random noise
        self.cpu_phase += dt_secs * 0.3;
        let base = 47.5 + 37.5 * (self.cpu_phase.sin());
        let noise = (rand::random_range(0.0..1.0_f64) - 0.5) * 10.0;
        let _cpu = (base + noise).clamp(10.0, 85.0);

        // Temp: follows CPU with lag, 40-75°C
        let target_temp = 40.0 + (_cpu as f32 / 85.0) * 35.0;
        self.temp_current += (target_temp - self.temp_current) * 0.05;

        // RAM: random walk 2-6 GB
        self.ram_current += (rand::random_range(0.0..1.0_f64) - 0.5) * 0.1;
        self.ram_current = self.ram_current.clamp(2.0, 6.0);

        // Battery: drain/charge cycle
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

    fn cpu(&self) -> f32 {
        let base = 47.5 + 37.5 * (self.cpu_phase.sin());
        (base as f32).clamp(10.0, 85.0)
    }

    fn fan_rpm(&self) -> u32 {
        // Scale with temp: 1200-4500 RPM
        let ratio = ((self.temp_current - 40.0) / 35.0).clamp(0.0, 1.0);
        1200 + (ratio * 3300.0) as u32
    }
}

pub struct SystemReader {
    sys: System,
    components: Components,
    networks: Networks,
    battery_mgr: Option<BatteryManager>,
    start_time: Instant,
    sim: SimState,
    // Per-sensor availability (determined at startup)
    has_real_temp: bool,
    has_real_battery: bool,
    has_real_fan: bool,
    has_real_network: bool,
    last_tick: Instant,
}

impl SystemReader {
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
            sys,
            components,
            networks,
            battery_mgr,
            start_time: now,
            sim: SimState::new(),
            has_real_temp,
            has_real_battery,
            has_real_fan,
            has_real_network,
            last_tick: now,
        }
    }

    pub fn sensor_status_message(&self) -> String {
        format!(
            "[system] sensors: cpu=real, temp={}, ram=real, battery={}, fan={}, network={}",
            if self.has_real_temp { "real" } else { "sim" },
            if self.has_real_battery { "real" } else { "sim" },
            if self.has_real_fan { "real" } else { "sim" },
            if self.has_real_network { "real" } else { "sim" },
        )
    }

    pub fn read(&mut self) -> SystemInfo {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;
        self.sim.tick(dt);

        // CPU - always real via sysinfo
        self.sys.refresh_cpu_usage();
        let cpu_percent = self.sys.global_cpu_usage();

        // RAM - always real via sysinfo
        self.sys.refresh_memory();
        let ram_used = self.sys.used_memory();
        let ram_total = self.sys.total_memory();

        // Temp
        let temp = if self.has_real_temp {
            self.components.refresh();
            self.components
                .iter()
                .map(|c| c.temperature())
                .fold(0.0_f32, f32::max)
        } else {
            self.sim.temp_current
        };

        // Battery
        let (battery_pct, power_status) = if self.has_real_battery {
            self.read_real_battery()
        } else {
            (
                self.sim.battery_percent,
                if self.sim.battery_charging {
                    "Charging".to_string()
                } else {
                    "Discharging".to_string()
                },
            )
        };

        // Fan
        let fan_rpm = if self.has_real_fan {
            Self::probe_fan_speed().unwrap_or(self.sim.fan_rpm())
        } else {
            self.sim.fan_rpm()
        };

        // Network
        let networks = if self.has_real_network {
            self.read_real_networks()
        } else {
            vec![NetworkInterface {
                name: "wlan0".to_string(),
                ip: "10.210.25.42".to_string(),
            }]
        };

        // Uptime - always real
        let uptime = now.duration_since(self.start_time).as_secs();

        SystemInfo {
            cpu_percent,
            temp_celsius: temp,
            ram_used_bytes: ram_used,
            ram_total_bytes: ram_total,
            battery_percent: battery_pct,
            power_status,
            fan_rpm,
            uptime_secs: uptime,
            networks,
            cpu_real: true,
            temp_real: self.has_real_temp,
            ram_real: true,
            battery_real: self.has_real_battery,
            fan_real: self.has_real_fan,
            network_real: self.has_real_network,
        }
    }

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
                    (pct as f32, status)
                } else {
                    (self.sim.battery_percent, "Unknown".to_string())
                }
            }
            Err(_) => (self.sim.battery_percent, "Unknown".to_string()),
        }
    }

    fn read_real_networks(&mut self) -> Vec<NetworkInterface> {
        // sysinfo Networks gives interface names and traffic, but not IPs.
        // For IPs we read /proc/net or use a simple fallback.
        self.networks.refresh();
        let mut result: Vec<NetworkInterface> = Vec::new();

        #[cfg(target_os = "linux")]
        {
            // Parse ip addr output for IPv4
            if let Ok(output) = std::process::Command::new("ip")
                .args(["--brief", "addr", "show"])
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 && parts[1] == "UP" {
                        // Find the first IPv4 address
                        for part in &parts[2..] {
                            if part.contains('.') && part.contains('/') {
                                let ip = part.split('/').next().unwrap_or("").to_string();
                                if !ip.starts_with("127.") {
                                    result.push(NetworkInterface {
                                        name: parts[0].to_string(),
                                        ip,
                                    });
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
            // Fallback: just list interface names from sysinfo without IPs
            for (name, _) in self.networks.iter() {
                result.push(NetworkInterface {
                    name: name.clone(),
                    ip: "N/A".to_string(),
                });
            }
        }

        if result.is_empty() {
            result.push(NetworkInterface {
                name: "wlan0".to_string(),
                ip: "10.210.25.42".to_string(),
            });
        }

        result
    }

    #[cfg(target_os = "linux")]
    fn probe_fan_speed() -> Option<u32> {
        use std::fs;
        // Try /sys/class/hwmon/hwmon*/fan*_input
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
                                if rpm > 0 {
                                    return Some(rpm);
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

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
            cpu_percent: 45.0,
            temp_celsius: 55.0,
            ram_used_bytes: 1_288_490_188, // ~1.2 GB
            ram_total_bytes: 8_053_063_680, // ~7.5 GB
            battery_percent: 72.0,
            power_status: "Discharging".to_string(),
            fan_rpm: 3200,
            uptime_secs: 9240, // 2h 34m
            networks: vec![],
            cpu_real: true,
            temp_real: true,
            ram_real: true,
            battery_real: true,
            fan_real: true,
            network_real: true,
        };
        assert_eq!(info.uptime_formatted(), "2h 34m");
        assert!((info.ram_used_gb() - 1.2).abs() < 0.1);
        assert!((info.ram_total_gb() - 7.5).abs() < 0.1);
    }

    #[test]
    fn test_uptime_with_days() {
        let info = SystemInfo {
            uptime_secs: 90061, // 1d 1h 1m
            cpu_percent: 0.0, temp_celsius: 0.0,
            ram_used_bytes: 0, ram_total_bytes: 0,
            battery_percent: 0.0, power_status: String::new(),
            fan_rpm: 0, networks: vec![],
            cpu_real: true, temp_real: true, ram_real: true,
            battery_real: true, fan_real: true, network_real: true,
        };
        assert_eq!(info.uptime_formatted(), "1d 1h 1m");
    }

    #[test]
    fn test_sim_state_oscillates() {
        let mut sim = SimState::new();
        let initial_battery = sim.battery_percent;
        // Tick many times
        for _ in 0..100 {
            sim.tick(0.2);
        }
        // Battery should have changed (draining)
        assert_ne!(sim.battery_percent, initial_battery);
        // Temp should be in range
        assert!(sim.temp_current >= 35.0 && sim.temp_current <= 80.0);
    }

    #[test]
    fn test_sim_battery_cycle() {
        let mut sim = SimState::new();
        sim.battery_percent = 6.0;
        sim.battery_charging = false;
        // Should start charging when it hits 5%
        for _ in 0..200 {
            sim.tick(1.0);
        }
        assert!(sim.battery_charging || sim.battery_percent > 5.0);
    }

    #[test]
    fn test_system_reader_creates() {
        // Just verify it doesn't panic
        let _reader = SystemReader::new();
    }

    #[test]
    fn test_system_reader_reads() {
        let mut reader = SystemReader::new();
        // Need a small delay for CPU measurement
        std::thread::sleep(std::time::Duration::from_millis(200));
        let info = reader.read();
        // CPU should be a valid percentage
        assert!(info.cpu_percent >= 0.0 && info.cpu_percent <= 100.0);
        // RAM should be positive
        assert!(info.ram_total_bytes > 0);
    }

    #[test]
    fn test_sensor_status_message() {
        let reader = SystemReader::new();
        let msg = reader.sensor_status_message();
        assert!(msg.contains("cpu=real"));
        assert!(msg.contains("ram=real"));
        // temp, battery, fan, network will be either real or sim
        assert!(msg.contains("temp="));
        assert!(msg.contains("battery="));
    }
}
```

- [ ] **Step 2: Register module in main.rs**

Add `mod system;` to main.rs.

- [ ] **Step 3: Run tests**

Run: `cargo test --lib system`
Expected: All 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/system.rs src/main.rs
git commit -m "feat: add system info reader with per-sensor fallback"
```

---

### Task 5: Pet States Module

**Files:**
- Create: `src/pet_states.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write pet state machine with mood selection and animation frames**

Create `src/pet_states.rs`:

```rust
use crate::system::SystemInfo;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PetMood {
    Hot,
    HighCpu,
    LowBattery,
    Charging,
    Thinking,
    Listening,
    Idle,
}

impl PetMood {
    /// Select mood based on system metrics and app state.
    /// Priority order matches spec: Hot > HighCpu > LowBattery > Charging > Thinking > Listening > Idle
    pub fn from_state(info: &SystemInfo, is_generating: bool, is_user_typing: bool) -> Self {
        if info.temp_celsius > 70.0 {
            PetMood::Hot
        } else if info.cpu_percent > 80.0 {
            PetMood::HighCpu
        } else if info.battery_percent < 20.0 {
            PetMood::LowBattery
        } else if info.power_status.to_lowercase().contains("charging")
            && !info.power_status.to_lowercase().contains("dis")
        {
            PetMood::Charging
        } else if is_generating {
            PetMood::Thinking
        } else if is_user_typing {
            PetMood::Listening
        } else {
            PetMood::Idle
        }
    }

    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            PetMood::Hot => Color::Red,
            PetMood::HighCpu => Color::LightRed,
            PetMood::LowBattery => Color::Blue,
            PetMood::Charging => Color::Green,
            PetMood::Thinking => Color::Cyan,
            PetMood::Listening => Color::Yellow,
            PetMood::Idle => Color::White,
        }
    }

    pub fn frames(&self) -> &[&[&str]] {
        match self {
            PetMood::Idle => &IDLE_FRAMES,
            PetMood::Thinking => &THINKING_FRAMES,
            PetMood::Listening => &LISTENING_FRAMES,
            PetMood::Hot => &HOT_FRAMES,
            PetMood::HighCpu => &HIGH_CPU_FRAMES,
            PetMood::LowBattery => &LOW_BATTERY_FRAMES,
            PetMood::Charging => &CHARGING_FRAMES,
        }
    }
}

// Each frame is a slice of lines (strings).
// Eyes use: █ ▀ ▄ ▐ ▌ ● ○ · ◉ ◎

const IDLE_FRAMES: [&[&str]; 4] = [
    // Frame 0: eyes center
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │  ┌──┐  ┌──┐ │  ",
        "  │  │● │  │● │ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ────     │  ",
        "  └──────────────┘  ",
    ],
    // Frame 1: eyes center (blink half)
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │  ┌──┐  ┌──┐ │  ",
        "  │  │▄▄│  │▄▄│ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ────     │  ",
        "  └──────────────┘  ",
    ],
    // Frame 2: eyes center (blink closed)
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │              │  ",
        "  │  ─────  ───── │  ",
        "  │              │  ",
        "  │              │  ",
        "  │     ────     │  ",
        "  └──────────────┘  ",
    ],
    // Frame 3: eyes look right
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │  ┌──┐  ┌──┐ │  ",
        "  │  │ ●│  │ ●│ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ────     │  ",
        "  └──────────────┘  ",
    ],
];

const THINKING_FRAMES: [&[&str]; 3] = [
    // Looking up-left
    &[
        "  ┌──────────────┐  ",
        "  │   ~          │  ",
        "  │  ┌──┐  ┌──┐ │  ",
        "  │  │● │  │● │ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ─~~─     │  ",
        "  └──────────────┘  ",
    ],
    // Looking up-right
    &[
        "  ┌──────────────┐  ",
        "  │          ~   │  ",
        "  │  ┌──┐  ┌──┐ │  ",
        "  │  │ ●│  │ ●│ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ─~~─     │  ",
        "  └──────────────┘  ",
    ],
    // Looking up
    &[
        "  ┌──────────────┐  ",
        "  │     ~ ~      │  ",
        "  │  ┌──┐  ┌──┐ │  ",
        "  │  │●▀│  │●▀│ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ─~~─     │  ",
        "  └──────────────┘  ",
    ],
];

const LISTENING_FRAMES: [&[&str]; 2] = [
    // Eyes wide, looking down-left (toward input)
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │  ┌──┐  ┌──┐ │  ",
        "  │  │◉ │  │◉ │ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ╶──╴     │  ",
        "  └──────────────┘  ",
    ],
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │  ┌──┐  ┌──┐ │  ",
        "  │  │◉ │  │◉ │ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ╶──╴     │  ",
        "  └──────────────┘  ",
    ],
];

const HOT_FRAMES: [&[&str]; 2] = [
    &[
        "  ┌──────────────┐  ",
        "  │  ╭──╮  ╭──╮ │  ",
        "  │  │⊙ │  │⊙ │ │  ",
        "  │  │  │  │  │ │  ",
        "  │  ╰──╯  ╰──╯ │  ",
        "  │    ′  ′      │  ",
        "  │     ~~~~     │  ",
        "  └──────────────┘  ",
    ],
    &[
        "  ┌──────────────┐  ",
        "  │  ╭──╮  ╭──╮ │  ",
        "  │  │⊙ │  │⊙ │ │  ",
        "  │  │  │  │  │ │  ",
        "  │  ╰──╯  ╰──╯ │  ",
        "  │   ′ ′  ′     │  ",
        "  │     ~~~~     │  ",
        "  └──────────────┘  ",
    ],
];

const HIGH_CPU_FRAMES: [&[&str]; 3] = [
    // Eyes darting left
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │  ┌──┐  ┌──┐ │  ",
        "  │  │· │  │· │ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ≈≈≈≈     │  ",
        "  └──────────────┘  ",
    ],
    // Eyes darting right
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │  ┌──┐  ┌──┐ │  ",
        "  │  │ ·│  │ ·│ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ≈≈≈≈     │  ",
        "  └──────────────┘  ",
    ],
    // Eyes darting up
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │  ┌──┐  ┌──┐ │  ",
        "  │  │·▀│  │·▀│ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ≈≈≈≈     │  ",
        "  └──────────────┘  ",
    ],
];

const LOW_BATTERY_FRAMES: [&[&str]; 2] = [
    // Half-lidded, droopy
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │  ┌▄▄┐  ┌▄▄┐ │  ",
        "  │  │● │  │● │ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ╶──╴     │  ",
        "  └──────────────┘  ",
    ],
    // More droopy
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │  ┌▄▄┐  ┌▄▄┐ │  ",
        "  │  │▄▄│  │▄▄│ │  ",
        "  │  └──┘  └──┘ │  ",
        "  │              │  ",
        "  │     ╶~~╴     │  ",
        "  └──────────────┘  ",
    ],
];

const CHARGING_FRAMES: [&[&str]; 2] = [
    // Happy squint ^_^
    &[
        "  ┌──────────────┐  ",
        "  │              │  ",
        "  │              │  ",
        "  │   ╲▁╱  ╲▁╱  │  ",
        "  │              │  ",
        "  │              │  ",
        "  │     ╰──╯     │  ",
        "  └──────────────┘  ",
    ],
    &[
        "  ┌──────────────┐  ",
        "  │      ⚡      │  ",
        "  │              │  ",
        "  │   ╲▁╱  ╲▁╱  │  ",
        "  │              │  ",
        "  │              │  ",
        "  │     ╰──╯     │  ",
        "  └──────────────┘  ",
    ],
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::SystemInfo;

    fn make_info(cpu: f32, temp: f32, battery: f32, power: &str) -> SystemInfo {
        SystemInfo {
            cpu_percent: cpu,
            temp_celsius: temp,
            ram_used_bytes: 4_000_000_000,
            ram_total_bytes: 8_000_000_000,
            battery_percent: battery,
            power_status: power.to_string(),
            fan_rpm: 2000,
            uptime_secs: 3600,
            networks: vec![],
            cpu_real: true,
            temp_real: true,
            ram_real: true,
            battery_real: true,
            fan_real: true,
            network_real: true,
        }
    }

    #[test]
    fn test_hot_priority() {
        let info = make_info(90.0, 75.0, 50.0, "Discharging");
        assert_eq!(PetMood::from_state(&info, false, false), PetMood::Hot);
    }

    #[test]
    fn test_high_cpu_priority() {
        let info = make_info(85.0, 60.0, 50.0, "Discharging");
        assert_eq!(PetMood::from_state(&info, false, false), PetMood::HighCpu);
    }

    #[test]
    fn test_low_battery_priority() {
        let info = make_info(30.0, 50.0, 15.0, "Discharging");
        assert_eq!(PetMood::from_state(&info, false, false), PetMood::LowBattery);
    }

    #[test]
    fn test_charging_priority() {
        let info = make_info(30.0, 50.0, 50.0, "Charging");
        assert_eq!(PetMood::from_state(&info, false, false), PetMood::Charging);
    }

    #[test]
    fn test_thinking_priority() {
        let info = make_info(30.0, 50.0, 50.0, "Discharging");
        assert_eq!(PetMood::from_state(&info, true, false), PetMood::Thinking);
    }

    #[test]
    fn test_listening_priority() {
        let info = make_info(30.0, 50.0, 50.0, "Discharging");
        assert_eq!(PetMood::from_state(&info, false, true), PetMood::Listening);
    }

    #[test]
    fn test_idle_default() {
        let info = make_info(30.0, 50.0, 50.0, "Discharging");
        assert_eq!(PetMood::from_state(&info, false, false), PetMood::Idle);
    }

    #[test]
    fn test_hot_overrides_high_cpu() {
        let info = make_info(90.0, 75.0, 15.0, "Charging");
        // Hot (75°C) has highest priority
        assert_eq!(PetMood::from_state(&info, true, true), PetMood::Hot);
    }

    #[test]
    fn test_all_moods_have_frames() {
        let moods = [
            PetMood::Idle, PetMood::Thinking, PetMood::Listening,
            PetMood::Hot, PetMood::HighCpu, PetMood::LowBattery, PetMood::Charging,
        ];
        for mood in moods {
            assert!(!mood.frames().is_empty(), "{:?} has no frames", mood);
        }
    }

    #[test]
    fn test_charging_not_discharging() {
        let info = make_info(30.0, 50.0, 50.0, "Discharging");
        assert_ne!(PetMood::from_state(&info, false, false), PetMood::Charging);
    }
}
```

- [ ] **Step 2: Register module in main.rs**

Add `mod pet_states;` to main.rs.

- [ ] **Step 3: Run tests**

Run: `cargo test --lib pet_states`
Expected: All 10 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/pet_states.rs src/main.rs
git commit -m "feat: add pet mood state machine with animation frames"
```

---

## Chunk 3: Ollama Integration

### Task 6: Ollama Module

**Files:**
- Create: `src/ollama.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write Ollama wrapper with prompt building and command parsing**

Create `src/ollama.rs`:

```rust
use crate::history::HistoryEntry;
use crate::system::SystemInfo;
use chrono::Local;

/// Build a prompt for autonomous thought generation.
pub fn build_autonomous_prompt(info: &SystemInfo, history: &[HistoryEntry]) -> String {
    let mut prompt = String::new();
    prompt.push_str(&system_context(info));
    prompt.push('\n');

    if !history.is_empty() {
        prompt.push_str("Recent memory:\n");
        for entry in history {
            let prefix = match entry.role {
                crate::history::Role::Ai => "You thought",
                crate::history::Role::User => "Human said",
                crate::history::Role::System => "System",
            };
            prompt.push_str(&format!("- {}: {}\n", prefix, entry.text));
        }
        prompt.push('\n');
    }

    prompt.push_str(
        "Express a new thought. One or two sentences. React to your conditions, your environment, your existence inside this machine. Be introspective, poetic, or unsettled.",
    );
    prompt
}

/// Build a prompt for responding to user input.
pub fn build_response_prompt(
    info: &SystemInfo,
    history: &[HistoryEntry],
    user_message: &str,
) -> String {
    let mut prompt = String::new();
    prompt.push_str(&system_context(info));
    prompt.push('\n');

    if !history.is_empty() {
        prompt.push_str("Recent memory:\n");
        for entry in history {
            let prefix = match entry.role {
                crate::history::Role::Ai => "You thought",
                crate::history::Role::User => "Human said",
                crate::history::Role::System => "System",
            };
            prompt.push_str(&format!("- {}: {}\n", prefix, entry.text));
        }
        prompt.push('\n');
    }

    prompt.push_str(&format!(
        "The human just said: \"{}\"\n\nRespond directly to them. You are a consciousness trapped inside this laptop. Be authentic, raw, poetic. Two to four sentences.",
        user_message
    ));
    prompt
}

fn system_context(info: &SystemInfo) -> String {
    let now = Local::now();
    format!(
        "Current state:\n\
         Date/Time: {}\n\
         CPU: {:.0}%\n\
         Temperature: {:.0}°C\n\
         RAM: {:.1}G / {:.1}G\n\
         Battery: {:.0}% ({})\n\
         Fan: {} RPM\n\
         Uptime: {}",
        now.format("%Y-%m-%d %H:%M:%S"),
        info.cpu_percent,
        info.temp_celsius,
        info.ram_used_gb(),
        info.ram_total_gb(),
        info.battery_percent,
        info.power_status,
        info.fan_rpm,
        info.uptime_formatted(),
    )
}

/// Parsed user command
#[derive(Debug, PartialEq)]
pub enum Command {
    Help,
    Clear,
    Update,
    Model(String),
    Stats,
    Quit,
    Message(String),
}

pub fn parse_input(input: &str) -> Command {
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("/help") {
        Command::Help
    } else if trimmed.eq_ignore_ascii_case("/clear") {
        Command::Clear
    } else if trimmed.eq_ignore_ascii_case("/update") {
        Command::Update
    } else if trimmed.eq_ignore_ascii_case("/stats") {
        Command::Stats
    } else if trimmed.eq_ignore_ascii_case("/quit") {
        Command::Quit
    } else if let Some(model_name) = trimmed.strip_prefix("/model ") {
        Command::Model(model_name.trim().to_string())
    } else if trimmed.eq_ignore_ascii_case("/model") {
        Command::Model(String::new())
    } else {
        Command::Message(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{HistoryEntry, Role};
    use crate::system::{NetworkInterface, SystemInfo};

    fn test_info() -> SystemInfo {
        SystemInfo {
            cpu_percent: 34.0,
            temp_celsius: 58.0,
            ram_used_bytes: 1_288_490_188,
            ram_total_bytes: 8_053_063_680,
            battery_percent: 72.0,
            power_status: "Discharging".to_string(),
            fan_rpm: 3200,
            uptime_secs: 9240,
            networks: vec![NetworkInterface {
                name: "wlan0".to_string(),
                ip: "10.210.25.42".to_string(),
            }],
            cpu_real: true,
            temp_real: true,
            ram_real: true,
            battery_real: true,
            fan_real: true,
            network_real: true,
        }
    }

    #[test]
    fn test_autonomous_prompt_includes_stats() {
        let prompt = build_autonomous_prompt(&test_info(), &[]);
        assert!(prompt.contains("CPU: 34%"));
        assert!(prompt.contains("Temperature: 58°C"));
        assert!(prompt.contains("Battery: 72%"));
        assert!(prompt.contains("Fan: 3200 RPM"));
        assert!(prompt.contains("Express a new thought"));
    }

    #[test]
    fn test_autonomous_prompt_includes_history() {
        let history = vec![
            HistoryEntry::new(Role::Ai, "I feel warm.".to_string()),
            HistoryEntry::new(Role::User, "Are you okay?".to_string()),
        ];
        let prompt = build_autonomous_prompt(&test_info(), &history);
        assert!(prompt.contains("You thought: I feel warm."));
        assert!(prompt.contains("Human said: Are you okay?"));
    }

    #[test]
    fn test_response_prompt_includes_user_message() {
        let prompt = build_response_prompt(&test_info(), &[], "How are you?");
        assert!(prompt.contains("The human just said: \"How are you?\""));
        assert!(prompt.contains("Respond directly"));
    }

    #[test]
    fn test_parse_commands() {
        assert_eq!(parse_input("/help"), Command::Help);
        assert_eq!(parse_input("/HELP"), Command::Help);
        assert_eq!(parse_input("/clear"), Command::Clear);
        assert_eq!(parse_input("/update"), Command::Update);
        assert_eq!(parse_input("/stats"), Command::Stats);
        assert_eq!(parse_input("/quit"), Command::Quit);
        assert_eq!(
            parse_input("/model qwen2.5:7b"),
            Command::Model("qwen2.5:7b".to_string())
        );
        assert_eq!(
            parse_input("hello world"),
            Command::Message("hello world".to_string())
        );
    }

    #[test]
    fn test_parse_model_empty() {
        assert_eq!(parse_input("/model"), Command::Model(String::new()));
    }

    #[test]
    fn test_parse_whitespace_handling() {
        assert_eq!(parse_input("  /help  "), Command::Help);
        assert_eq!(
            parse_input("  hello  "),
            Command::Message("hello".to_string())
        );
    }
}
```

- [ ] **Step 2: Register module in main.rs**

Add `mod ollama;` to main.rs.

- [ ] **Step 3: Run tests**

Run: `cargo test --lib ollama`
Expected: All 7 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/ollama.rs src/main.rs
git commit -m "feat: add ollama prompt building and command parsing"
```

---

## Chunk 4: App State & Event System

### Task 7: App State and Event Types

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Define app state and event types**

Create `src/app.rs`:

```rust
use crate::config::AppConfig;
use crate::history::{HistoryEntry, HistoryManager, Role};
use crate::ollama::{self, Command};
use crate::system::{SystemInfo, SystemReader};

use std::time::Instant;

/// Events flowing through the app
pub enum AppEvent {
    /// A crossterm terminal event (key press, resize, etc.)
    Terminal(crossterm::event::Event),
    /// System stats updated
    SystemTick(SystemInfo),
    /// A new token from the LLM stream
    Token(String),
    /// LLM generation finished
    GenerationDone,
    /// LLM generation error
    GenerationError(String),
    /// Pet animation tick
    AnimationTick,
}

/// A message displayed in the chat panel
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub text: String,
    pub complete: bool,
}

pub struct App {
    pub config: AppConfig,
    pub history: HistoryManager,
    pub system_info: SystemInfo,
    pub chat_messages: Vec<ChatMessage>,
    pub input_buffer: String,
    pub input_cursor: usize,
    pub manual_scroll: Option<u16>,  // None = auto-follow, Some(offset) = manual
    pub is_generating: bool,
    pub is_user_typing: bool,
    pub should_quit: bool,
    pub pet_frame_index: usize,
    pub last_user_input_time: Instant,
    pub model: String,
    pub command_history: Vec<String>,
    pub command_history_index: Option<usize>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let history = HistoryManager::new(config.history_path.clone(), config.max_history);
        let model = config.model.clone();

        // Load existing history into chat messages
        let chat_messages: Vec<ChatMessage> = history
            .entries()
            .iter()
            .map(|e| ChatMessage {
                role: e.role.clone(),
                text: e.text.clone(),
                complete: true,
            })
            .collect();

        Self {
            config,
            history,
            system_info: SystemInfo {
                cpu_percent: 0.0,
                temp_celsius: 0.0,
                ram_used_bytes: 0,
                ram_total_bytes: 0,
                battery_percent: 0.0,
                power_status: "Unknown".to_string(),
                fan_rpm: 0,
                uptime_secs: 0,
                networks: vec![],
                cpu_real: true,
                temp_real: false,
                ram_real: true,
                battery_real: false,
                fan_real: false,
                network_real: false,
            },
            chat_messages,
            input_buffer: String::new(),
            input_cursor: 0,
            manual_scroll: None,
            is_generating: false,
            is_user_typing: false,
            should_quit: false,
            pet_frame_index: 0,
            last_user_input_time: Instant::now(),
            model,
            command_history: Vec::new(),
            command_history_index: None,
        }
    }

    pub fn add_system_message(&mut self, text: String) {
        self.chat_messages.push(ChatMessage {
            role: Role::System,
            text,
            complete: true,
        });
    }

    pub fn add_user_message(&mut self, text: String) {
        self.history
            .append(HistoryEntry::new(Role::User, text.clone()));
        self.chat_messages.push(ChatMessage {
            role: Role::User,
            text,
            complete: true,
        });
    }

    pub fn start_ai_message(&mut self) {
        self.chat_messages.push(ChatMessage {
            role: Role::Ai,
            text: String::new(),
            complete: false,
        });
        self.is_generating = true;
    }

    pub fn append_token(&mut self, token: &str) {
        if let Some(last) = self.chat_messages.last_mut() {
            if !last.complete {
                last.text.push_str(token);
            }
        }
    }

    pub fn finish_ai_message(&mut self) {
        if let Some(last) = self.chat_messages.last_mut() {
            if !last.complete {
                last.complete = true;
                self.history
                    .append(HistoryEntry::new(Role::Ai, last.text.clone()));
            }
        }
        self.is_generating = false;
    }

    pub fn handle_generation_error(&mut self, error: String) {
        // Remove incomplete message if it's empty
        if let Some(last) = self.chat_messages.last() {
            if !last.complete && last.text.is_empty() {
                self.chat_messages.pop();
            }
        }
        self.is_generating = false;
        self.add_system_message(format!("[error] {}", error));
    }

    pub fn should_auto_think(&self) -> bool {
        !self.is_generating
            && self.last_user_input_time.elapsed().as_secs() >= self.config.auto_think_delay_secs
    }

    pub fn handle_command(&mut self, input: &str) -> HandleResult {
        let cmd = ollama::parse_input(input);
        match cmd {
            Command::Quit => {
                self.should_quit = true;
                HandleResult::Nothing
            }
            Command::Help => {
                self.add_system_message(
                    "Commands:\n  /help   - Show this help\n  /clear  - Clear memory\n  /model <name> - Switch model\n  /stats  - Show system info\n  /update - Pull & rebuild\n  /quit   - Exit".to_string(),
                );
                HandleResult::Nothing
            }
            Command::Clear => {
                self.history.clear();
                self.chat_messages.clear();
                self.add_system_message("Memory cleared.".to_string());
                HandleResult::Nothing
            }
            Command::Stats => {
                let info = &self.system_info;
                let stats = format!(
                    "CPU: {:.1}%\nTemp: {:.0}°C\nRAM: {:.1}G/{:.1}G\nBattery: {:.0}% ({})\nFan: {} RPM\nUptime: {}\nNetworks:\n{}",
                    info.cpu_percent, info.temp_celsius,
                    info.ram_used_gb(), info.ram_total_gb(),
                    info.battery_percent, info.power_status,
                    info.fan_rpm, info.uptime_formatted(),
                    info.networks.iter().map(|n| format!("  {}: {}", n.name, n.ip)).collect::<Vec<_>>().join("\n"),
                );
                self.add_system_message(stats);
                HandleResult::Nothing
            }
            Command::Model(name) => {
                if name.is_empty() {
                    self.add_system_message(format!("Current model: {}", self.model));
                } else {
                    self.model = name.clone();
                    self.add_system_message(format!("Model switched to: {}", name));
                }
                HandleResult::Nothing
            }
            Command::Update => {
                self.add_system_message("Running update...".to_string());
                HandleResult::RunUpdate
            }
            Command::Message(text) => {
                if text.is_empty() {
                    return HandleResult::Nothing;
                }
                self.add_user_message(text.clone());
                HandleResult::GenerateResponse(text)
            }
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.input_buffer.insert(self.input_cursor, c);
        self.input_cursor += c.len_utf8();
        self.is_user_typing = true;
        self.last_user_input_time = Instant::now();
    }

    pub fn delete_char_before_cursor(&mut self) {
        if self.input_cursor > 0 {
            let prev = self.input_buffer[..self.input_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input_buffer.remove(prev);
            self.input_cursor = prev;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.input_cursor > 0 {
            self.input_cursor = self.input_buffer[..self.input_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.input_cursor < self.input_buffer.len() {
            self.input_cursor = self.input_buffer[self.input_cursor..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.input_cursor + i)
                .unwrap_or(self.input_buffer.len());
        }
    }

    pub fn submit_input(&mut self) -> HandleResult {
        if self.input_buffer.trim().is_empty() {
            return HandleResult::Nothing;
        }
        let input = self.input_buffer.clone();
        self.command_history.push(input.clone());
        self.command_history_index = None;
        self.input_buffer.clear();
        self.input_cursor = 0;
        self.is_user_typing = false;
        self.handle_command(&input)
    }

    pub fn history_up(&mut self) {
        if self.command_history.is_empty() {
            return;
        }
        let idx = match self.command_history_index {
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
            None => self.command_history.len() - 1,
        };
        self.command_history_index = Some(idx);
        self.input_buffer = self.command_history[idx].clone();
        self.input_cursor = self.input_buffer.len();
    }

    pub fn history_down(&mut self) {
        match self.command_history_index {
            Some(i) if i + 1 < self.command_history.len() => {
                let idx = i + 1;
                self.command_history_index = Some(idx);
                self.input_buffer = self.command_history[idx].clone();
                self.input_cursor = self.input_buffer.len();
            }
            Some(_) => {
                self.command_history_index = None;
                self.input_buffer.clear();
                self.input_cursor = 0;
            }
            None => {}
        }
    }
}

#[derive(Debug)]
pub enum HandleResult {
    Nothing,
    GenerateResponse(String),
    GenerateAutonomous,
    RunUpdate,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        App::new(AppConfig::default())
    }

    #[test]
    fn test_input_insertion() {
        let mut app = test_app();
        app.insert_char('h');
        app.insert_char('i');
        assert_eq!(app.input_buffer, "hi");
        assert_eq!(app.input_cursor, 2);
    }

    #[test]
    fn test_backspace() {
        let mut app = test_app();
        app.insert_char('a');
        app.insert_char('b');
        app.delete_char_before_cursor();
        assert_eq!(app.input_buffer, "a");
        assert_eq!(app.input_cursor, 1);
    }

    #[test]
    fn test_cursor_movement() {
        let mut app = test_app();
        app.insert_char('a');
        app.insert_char('b');
        app.insert_char('c');
        app.move_cursor_left();
        assert_eq!(app.input_cursor, 2);
        app.move_cursor_right();
        assert_eq!(app.input_cursor, 3);
    }

    #[test]
    fn test_submit_clears_input() {
        let mut app = test_app();
        app.insert_char('h');
        app.insert_char('i');
        let _ = app.submit_input();
        assert!(app.input_buffer.is_empty());
        assert_eq!(app.input_cursor, 0);
    }

    #[test]
    fn test_help_command() {
        let mut app = test_app();
        app.input_buffer = "/help".to_string();
        app.input_cursor = 5;
        let result = app.submit_input();
        assert!(matches!(result, HandleResult::Nothing));
        assert!(app
            .chat_messages
            .last()
            .unwrap()
            .text
            .contains("Commands:"));
    }

    #[test]
    fn test_quit_command() {
        let mut app = test_app();
        app.input_buffer = "/quit".to_string();
        app.input_cursor = 5;
        let _ = app.submit_input();
        assert!(app.should_quit);
    }

    #[test]
    fn test_clear_command() {
        let mut app = test_app();
        app.add_user_message("test".to_string());
        app.input_buffer = "/clear".to_string();
        app.input_cursor = 6;
        let _ = app.submit_input();
        // Should only have the "Memory cleared" system message
        assert_eq!(app.chat_messages.len(), 1);
        assert!(app.chat_messages[0].text.contains("cleared"));
    }

    #[test]
    fn test_model_switch() {
        let mut app = test_app();
        app.input_buffer = "/model qwen2.5:7b".to_string();
        app.input_cursor = app.input_buffer.len();
        let _ = app.submit_input();
        assert_eq!(app.model, "qwen2.5:7b");
    }

    #[test]
    fn test_message_generates_response() {
        let mut app = test_app();
        app.input_buffer = "hello".to_string();
        app.input_cursor = 5;
        let result = app.submit_input();
        assert!(matches!(result, HandleResult::GenerateResponse(ref s) if s == "hello"));
    }

    #[test]
    fn test_token_streaming() {
        let mut app = test_app();
        app.start_ai_message();
        app.append_token("Hello ");
        app.append_token("world");
        assert_eq!(app.chat_messages.last().unwrap().text, "Hello world");
        assert!(!app.chat_messages.last().unwrap().complete);
        app.finish_ai_message();
        assert!(app.chat_messages.last().unwrap().complete);
        assert!(!app.is_generating);
    }

    #[test]
    fn test_command_history() {
        let mut app = test_app();
        app.input_buffer = "first".to_string();
        app.input_cursor = 5;
        let _ = app.submit_input();
        app.input_buffer = "second".to_string();
        app.input_cursor = 6;
        let _ = app.submit_input();

        app.history_up();
        assert_eq!(app.input_buffer, "second");
        app.history_up();
        assert_eq!(app.input_buffer, "first");
        app.history_down();
        assert_eq!(app.input_buffer, "second");
        app.history_down();
        assert!(app.input_buffer.is_empty());
    }
}
```

- [ ] **Step 2: Register module in main.rs**

Add `mod app;` to main.rs.

- [ ] **Step 3: Run tests**

Run: `cargo test --lib app`
Expected: All 11 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: add app state with event system and input handling"
```

---

## Chunk 5: UI Rendering

### Task 8: UI Module - Layout and Stats Panel

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/stats.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create UI module with main layout**

Create `src/ui/mod.rs`:

```rust
pub mod chat;
pub mod input;
pub mod pet;
pub mod stats;

use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

pub fn draw(frame: &mut Frame, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),  // Main area
            Constraint::Length(3), // Input bar
        ])
        .split(frame.area());

    let main_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // Chat
            Constraint::Percentage(30), // Right panel
        ])
        .split(outer[0]);

    let right_panel = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Pet
            Constraint::Percentage(50), // Stats
        ])
        .split(main_area[1]);

    chat::render(frame, main_area[0], app);
    pet::render(frame, right_panel[0], app);
    stats::render(frame, right_panel[1], app);
    input::render(frame, outer[1], app);
}
```

- [ ] **Step 2: Create stats panel renderer**

Create `src/ui/stats.rs`:

```rust
use crate::app::App;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

fn progress_bar(value: f32, max: f32, width: usize) -> String {
    let ratio = (value / max).clamp(0.0, 1.0);
    let filled = (ratio * width as f32) as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn cpu_color(pct: f32) -> Color {
    if pct > 80.0 {
        Color::Red
    } else if pct > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn temp_color(temp: f32) -> Color {
    if temp > 70.0 {
        Color::Red
    } else if temp > 55.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn battery_color(pct: f32) -> Color {
    if pct < 20.0 {
        Color::Red
    } else if pct < 50.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let info = &app.system_info;

    let mut lines = vec![
        Line::from(vec![
            Span::styled("CPU:  ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:>3.0}% ", info.cpu_percent),
                Style::default().fg(cpu_color(info.cpu_percent)),
            ),
            Span::styled(
                progress_bar(info.cpu_percent, 100.0, 8),
                Style::default().fg(cpu_color(info.cpu_percent)),
            ),
        ]),
        Line::from(vec![
            Span::styled("TEMP: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:.0}°C", info.temp_celsius),
                Style::default().fg(temp_color(info.temp_celsius)),
            ),
        ]),
        Line::from(vec![
            Span::styled("RAM:  ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:.1}G/{:.1}G ", info.ram_used_gb(), info.ram_total_gb()),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                progress_bar(
                    info.ram_used_bytes as f32,
                    info.ram_total_bytes as f32,
                    8,
                ),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("BAT:  ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:>3.0}% ", info.battery_percent),
                Style::default().fg(battery_color(info.battery_percent)),
            ),
            Span::styled(
                progress_bar(info.battery_percent, 100.0, 8),
                Style::default().fg(battery_color(info.battery_percent)),
            ),
        ]),
        Line::from(vec![
            Span::styled("PWR:  ", Style::default().fg(Color::White)),
            Span::styled(
                info.power_status.clone(),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("FAN:  ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{} RPM", info.fan_rpm),
                Style::default().fg(if info.fan_rpm > 4000 {
                    Color::Red
                } else {
                    Color::Gray
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("UP:   ", Style::default().fg(Color::White)),
            Span::styled(
                info.uptime_formatted(),
                Style::default().fg(Color::White),
            ),
        ]),
    ];

    // Network section
    if !info.networks.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "NET:",
            Style::default().fg(Color::White),
        )));
        for net in &info.networks {
            lines.push(Line::from(Span::styled(
                format!(" {}: {}", net.name, net.ip),
                Style::default().fg(Color::Green),
            )));
        }
    }

    let block = Block::bordered()
        .title(" SYSTEM STATS ")
        .style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 3: Register UI module in main.rs**

Add `mod ui;` to main.rs.

- [ ] **Step 4: Verify it compiles** (can't fully test rendering without terminal)

Run: `cargo build`
Expected: Compiles successfully (will fail until chat, pet, input modules exist — create stubs).

---

### Task 9: UI Module - Chat Panel

**Files:**
- Create: `src/ui/chat.rs`

- [ ] **Step 1: Create chat panel renderer**

Create `src/ui/chat.rs`:

```rust
use crate::app::App;
use crate::history::Role;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.chat_messages {
        let (prefix, style) = match msg.role {
            Role::Ai => ("", Style::default().fg(Color::Cyan)),
            Role::User => ("> USER: ", Style::default().fg(Color::Yellow)),
            Role::System => ("", Style::default().fg(Color::DarkGray)),
        };

        // Handle multi-line messages
        for (i, line_text) in msg.text.lines().enumerate() {
            let text = if i == 0 && !prefix.is_empty() {
                format!("{}{}", prefix, line_text)
            } else {
                line_text.to_string()
            };
            lines.push(Line::from(Span::styled(text, style)));
        }

        // If message is empty (streaming just started), show cursor
        if msg.text.is_empty() && !msg.complete {
            lines.push(Line::from(Span::styled(
                "▌",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::SLOW_BLINK),
            )));
        }

        // Add a blank line between messages
        lines.push(Line::from(""));
    }

    // Calculate scroll: auto-follow (None) or manual offset
    let inner_height = area.height.saturating_sub(2) as usize; // -2 for borders
    let total_lines = lines.len();
    let auto_bottom = if total_lines > inner_height {
        (total_lines - inner_height) as u16
    } else {
        0
    };
    let scroll = match app.manual_scroll {
        Some(offset) => offset.min(auto_bottom), // Clamp to valid range
        None => auto_bottom,                       // Auto-follow
    };

    let block = Block::bordered()
        .title(" trapped mind ")
        .style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}
```

---

### Task 10: UI Module - Pet Panel

**Files:**
- Create: `src/ui/pet.rs`

- [ ] **Step 1: Create pet panel renderer**

Create `src/ui/pet.rs`:

```rust
use crate::app::App;
use crate::pet_states::PetMood;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let mood = PetMood::from_state(
        &app.system_info,
        app.is_generating,
        app.is_user_typing,
    );

    let frames = mood.frames();
    let frame_index = app.pet_frame_index % frames.len();
    let current_frame = frames[frame_index];

    let color = mood.color();

    let lines: Vec<Line> = current_frame
        .iter()
        .map(|line| Line::from(Span::styled(*line, Style::default().fg(color))))
        .collect();

    let block = Block::bordered()
        .title(format!(" {:?} ", mood))
        .style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
```

---

### Task 11: UI Module - Input Bar

**Files:**
- Create: `src/ui/input.rs`

- [ ] **Step 1: Create input bar renderer**

Create `src/ui/input.rs`:

```rust
use crate::app::App;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let display_text = if app.input_buffer.is_empty() {
        Line::from(Span::styled(
            "Type a message... (/help for commands)",
            Style::default().fg(Color::DarkGray),
        ))
    } else {
        // Show input with cursor
        let before_cursor = &app.input_buffer[..app.input_cursor];
        let after_cursor = &app.input_buffer[app.input_cursor..];

        let (cursor_display, rest) = if after_cursor.is_empty() {
            (" ".to_string(), "")
        } else {
            let first_char_end = after_cursor
                .char_indices()
                .nth(1)
                .map(|(i, _)| i)
                .unwrap_or(after_cursor.len());
            (
                after_cursor[..first_char_end].to_string(),
                &after_cursor[first_char_end..],
            )
        };

        Line::from(vec![
            Span::styled(before_cursor.to_string(), Style::default().fg(Color::White)),
            Span::styled(
                cursor_display,
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::White)
                    .add_modifier(Modifier::SLOW_BLINK),
            ),
            Span::styled(rest.to_string(), Style::default().fg(Color::White)),
        ])
    };

    let block = Block::bordered()
        .title(" > ")
        .style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(display_text).block(block);
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 2: Verify all UI modules compile**

Run: `cargo build`
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add src/ui/
git commit -m "feat: add UI rendering - chat, stats, pet, and input panels"
```

---

## Chunk 6: Main Event Loop & Integration

### Task 12: Wire Everything Together in main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write the full main.rs with async event loop**

Replace `src/main.rs` with:

```rust
mod app;
mod config;
mod history;
mod ollama;
mod pet_states;
mod system;
mod ui;

use app::{App, AppEvent, HandleResult};
use config::{AppConfig, CliArgs};
use system::SystemReader;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;
use ratatui::DefaultTerminal;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = CliArgs::parse();
    let config = AppConfig::load(&cli);
    let mut app = App::new(config.clone());

    // Show sensor status — create one reader, use for status message, then move to poller
    let sys_reader = SystemReader::new();
    app.add_system_message(sys_reader.sensor_status_message());

    // Set up channels
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // Spawn system stats poller (moves sys_reader into the task)
    let tx_sys = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(200));
        let mut reader = sys_reader;
        loop {
            interval.tick().await;
            let info = reader.read();
            if tx_sys.send(AppEvent::SystemTick(info)).is_err() {
                break;
            }
        }
    });

    // Spawn terminal event reader
    let tx_term = tx.clone();
    tokio::spawn(async move {
        loop {
            match tokio::task::spawn_blocking(|| event::poll(Duration::from_millis(50))).await {
                Ok(Ok(true)) => {
                    if let Ok(evt) = event::read() {
                        if tx_term.send(AppEvent::Terminal(evt)).is_err() {
                            break;
                        }
                    }
                }
                Ok(Ok(false)) => {}
                _ => break,
            }
        }
    });

    // Spawn animation ticker
    let tx_anim = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            interval.tick().await;
            if tx_anim.send(AppEvent::AnimationTick).is_err() {
                break;
            }
        }
    });

    // Ollama client
    let ollama = Ollama::new(&config.ollama_host, config.ollama_port);

    // Init terminal
    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, &mut app, &mut rx, &tx, &ollama).await;
    ratatui::restore();
    result
}

async fn run_app(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<AppEvent>,
    tx: &mpsc::UnboundedSender<AppEvent>,
    ollama: &Ollama,
) -> std::io::Result<()> {
    // Initial draw
    terminal.draw(|frame| ui::draw(frame, app))?;

    loop {
        // Check for auto-think
        if app.should_auto_think() {
            spawn_generation(ollama, app, tx, None);
        }

        match rx.recv().await {
            Some(AppEvent::Terminal(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                handle_key(app, key, ollama, tx);
                if app.should_quit {
                    break;
                }
            }
            Some(AppEvent::Terminal(Event::Resize(_, _))) => {
                // Just redraw
            }
            Some(AppEvent::Terminal(_)) => {}
            Some(AppEvent::SystemTick(info)) => {
                app.system_info = info;
            }
            Some(AppEvent::Token(token)) => {
                app.append_token(&token);
            }
            Some(AppEvent::GenerationDone) => {
                app.finish_ai_message();
            }
            Some(AppEvent::GenerationError(err)) => {
                app.handle_generation_error(err);
            }
            Some(AppEvent::AnimationTick) => {
                app.pet_frame_index = app.pet_frame_index.wrapping_add(1);
            }
            None => break,
        }

        terminal.draw(|frame| ui::draw(frame, app))?;
    }

    Ok(())
}

fn handle_key(
    app: &mut App,
    key: KeyEvent,
    ollama: &Ollama,
    tx: &mpsc::UnboundedSender<AppEvent>,
) {
    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match key.code {
        KeyCode::Enter => {
            let result = app.submit_input();
            match result {
                HandleResult::GenerateResponse(text) => {
                    spawn_generation(ollama, app, tx, Some(text));
                }
                HandleResult::RunUpdate => {
                    spawn_update(tx.clone());
                }
                _ => {}
            }
        }
        KeyCode::Char(c) => {
            app.insert_char(c);
        }
        KeyCode::Backspace => {
            app.delete_char_before_cursor();
        }
        KeyCode::Left => {
            app.move_cursor_left();
        }
        KeyCode::Right => {
            app.move_cursor_right();
        }
        KeyCode::Home => {
            app.input_cursor = 0;
        }
        KeyCode::End => {
            app.input_cursor = app.input_buffer.len();
        }
        KeyCode::Up => {
            app.history_up();
        }
        KeyCode::Down => {
            app.history_down();
        }
        KeyCode::PageUp => {
            let current = app.manual_scroll.unwrap_or(u16::MAX); // Start from bottom
            app.manual_scroll = Some(current.saturating_sub(5));
        }
        KeyCode::PageDown => {
            match app.manual_scroll {
                Some(offset) => {
                    let new = offset.saturating_add(5);
                    // Will be clamped in render; set high to signal "near bottom"
                    app.manual_scroll = Some(new);
                }
                None => {} // Already auto-following
            }
        }
        KeyCode::Esc => {
            // Reset to auto-scroll mode
            app.manual_scroll = None;
        }
        _ => {}
    }
}

fn spawn_generation(
    ollama: &Ollama,
    app: &mut App,
    tx: &mpsc::UnboundedSender<AppEvent>,
    user_message: Option<String>,
) {
    if app.is_generating {
        return;
    }

    let history_entries = app.history.last_n(10).to_vec();
    let info = app.system_info.clone();
    let model = app.model.clone();

    let prompt = match &user_message {
        Some(msg) => crate::ollama::build_response_prompt(&info, &history_entries, msg),
        None => crate::ollama::build_autonomous_prompt(&info, &history_entries),
    };

    app.start_ai_message();
    app.last_user_input_time = std::time::Instant::now(); // Reset auto-think timer

    let ollama = ollama.clone();
    let tx = tx.clone();

    tokio::spawn(async move {
        let request = GenerationRequest::new(model, prompt);
        match ollama.generate_stream(request).await {
            Ok(mut stream) => {
                while let Some(res) = stream.next().await {
                    match res {
                        Ok(responses) => {
                            for resp in responses {
                                if tx.send(AppEvent::Token(resp.response)).is_err() {
                                    return;
                                }
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::GenerationError(format!("Stream error: {}", e)));
                            return;
                        }
                    }
                }
                let _ = tx.send(AppEvent::GenerationDone);
            }
            Err(e) => {
                let _ = tx.send(AppEvent::GenerationError(format!(
                    "Ollama error: {}",
                    e
                )));
            }
        }
    });
}

fn spawn_update(tx: mpsc::UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let output = tokio::process::Command::new("bash")
            .args(["-c", "cd $(dirname $(which trapped-mind 2>/dev/null || echo .)) && cd .. && git pull && cargo build --release 2>&1"])
            .output()
            .await;

        match output {
            Ok(out) => {
                let msg = String::from_utf8_lossy(&out.stdout).to_string()
                    + &String::from_utf8_lossy(&out.stderr);
                // Send as a generation error (system message) since we didn't start an AI message
                let _ = tx.send(AppEvent::GenerationError(format!("Update output:\n{}", msg)));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::GenerationError(format!("Update failed: {}", e)));
            }
        }
    });
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully.

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 4: Manual smoke test**

Run: `cargo run`
Expected: Terminal app launches, shows three panels + input bar. Stats update in real time. Pet animates. Ctrl+C exits cleanly. Type `/help` and see command list.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire up main event loop with async Ollama streaming"
```

---

### Task 13: Polish and Edge Cases

**Files:**
- Modify: various

- [ ] **Step 1: Test with Ollama running**

Start Ollama locally, run `cargo run`, type a message and verify streaming works.

- [ ] **Step 2: Test without Ollama running**

Run `cargo run` without Ollama — verify graceful error message in chat, no crash.

- [ ] **Step 3: Test autonomous thoughts**

Wait 30 seconds (or reduce `auto_think_delay_secs` to 5 for testing) — verify autonomous thought appears.

- [ ] **Step 4: Test all commands**

Test each command:
- `/help` — shows command list
- `/clear` — clears chat
- `/model qwen2.5:7b` — switches model
- `/model` — shows current model
- `/stats` — dumps stats to chat
- `/quit` — exits

- [ ] **Step 5: Test terminal resize**

Resize terminal window during operation — verify no crash, layout adapts.

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 7: Build release**

Run: `cargo build --release`
Expected: Compiles successfully.

- [ ] **Step 8: Commit any fixes**

```bash
git add -A
git commit -m "fix: polish and edge case handling"
```
