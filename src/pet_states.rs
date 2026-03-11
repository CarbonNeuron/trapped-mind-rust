//! Pet face mood states and Unicode art animation frames.
//!
//! The pet face reacts to system conditions with 7 mood states, each with
//! its own color and set of animation frames. Moods are resolved in priority
//! order: hardware alerts (hot, high CPU, low battery) take precedence over
//! behavioral states (thinking, listening, idle).

use crate::system::SystemInfo;

/// The pet face's current mood, determined by system state and app activity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PetMood {
    /// Temperature > 70°C — sweating, distressed face.
    Hot,
    /// CPU > 80% — vibrating, overwhelmed face.
    HighCpu,
    /// Battery < 20% — drowsy, worried face.
    LowBattery,
    /// AC power connected — happy, relaxed face with lightning bolt.
    Charging,
    /// Ollama generation in progress — contemplative face with thought bubbles.
    Thinking,
    /// User is typing — attentive face with wide eyes.
    Listening,
    /// Default state — calm, blinking face.
    Idle,
}

impl PetMood {
    /// Determines the current mood from system metrics and app state.
    ///
    /// Moods are checked in priority order: hardware alerts first, then
    /// behavioral states, falling back to [`PetMood::Idle`].
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

    /// Returns the display color for this mood.
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

    /// Returns the animation frames for this mood.
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

// -- Animation frames: each mood has 2-4 frames of Unicode art --

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
