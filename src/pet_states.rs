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

const IDLE_FRAMES: [&[&str]; 4] = [
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
            cpu_percent: cpu, temp_celsius: temp,
            ram_used_bytes: 4_000_000_000, ram_total_bytes: 8_000_000_000,
            battery_percent: battery, power_status: power.to_string(),
            fan_rpm: 2000, uptime_secs: 3600, networks: vec![],
            cpu_real: true, temp_real: true, ram_real: true,
            battery_real: true, fan_real: true, network_real: true,
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
    fn test_hot_overrides_all() {
        let info = make_info(90.0, 75.0, 15.0, "Charging");
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
