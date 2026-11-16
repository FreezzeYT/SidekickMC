use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Actions;

impl Actions {
    /// Adjust system volume.
    /// Emitting multimedia keys via uinput is handled in `input.rs`.
    /// This method provides direct mixer fallback using `wpctl` (PipeWire), `pactl` (PulseAudio), or `amixer` (ALSA).
    pub fn adjust_volume_mixer(delta_steps: i32, step_pct: i32) {
        let abs_delta = (delta_steps.abs() * step_pct).to_string();
        let sign = if delta_steps > 0 { "+" } else { "-" };
        let delta_str = format!("{}{}%", abs_delta, sign);

        // Try PipeWire (Ubuntu 22.10+ / 24.04 default)
        if Command::new("wpctl")
            .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{}%", if delta_steps > 0 { format!("{}+", abs_delta) } else { format!("{}-", abs_delta) })])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return;
        }

        // Try PulseAudio
        if Command::new("pactl")
            .args(["set-sink-volume", "@DEFAULT_SINK@", &delta_str])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return;
        }

        // Try ALSA
        let _ = Command::new("amixer")
            .args(["-q", "sset", "Master", &delta_str])
            .status();
    }

    /// Find the primary laptop backlight controller in /sys/class/backlight
    fn find_backlight() -> Option<PathBuf> {
        let backlight_dir = Path::new("/sys/class/backlight");
        if let Ok(entries) = fs::read_dir(backlight_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.join("brightness").exists() && path.join("max_brightness").exists() {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Adjust laptop screen brightness in percentage increments
    pub fn adjust_brightness(delta_steps: i32, step_pct: i32) {
        if let Some(backlight) = Self::find_backlight() {
            let max_path = backlight.join("max_brightness");
            let cur_path = backlight.join("brightness");

            if let (Ok(max_str), Ok(cur_str)) = (fs::read_to_string(&max_path), fs::read_to_string(&cur_path)) {
                if let (Ok(max_val), Ok(cur_val)) = (max_str.trim().parse::<i64>(), cur_str.trim().parse::<i64>()) {
                    let step_val = (max_val * step_pct as i64) / 100;
                    let step_val = step_val.max(1); // At least 1 unit
                    let new_val = (cur_val + step_val * delta_steps as i64).clamp(0, max_val);

                    if fs::write(&cur_path, new_val.to_string()).is_ok() {
                        log::info!("Set backlight ({}) to {}/{}", backlight.display(), new_val, max_val);
                        return;
                    }
                }
            }
        }

        // Fallback: brightnessctl (standard on Ubuntu, handles permissions via suid/udev)
        let pct = delta_steps.abs() * step_pct;
        let sign = if delta_steps > 0 { "+" } else { "-" };
        let arg = format!("{}{}%", pct, sign);

        if Command::new("brightnessctl")
            .args(["set", &arg])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return;
        }

        // Fallback: light
        let flag = if delta_steps > 0 { "-A" } else { "-U" };
        let _ = Command::new("light")
            .args([flag, &pct.to_string()])
            .status();
    }

    /// Trigger screen snipping / area screenshot tool
    pub fn open_screen_snipping() {
        std::thread::spawn(|| {
            // 1. Try XDG Desktop Portal Screenshot (Interactive mode - works on GNOME, KDE, Wayland, and X11)
            let portal_res = Command::new("gdbus")
                .args([
                    "call",
                    "--session",
                    "--dest", "org.freedesktop.portal.Desktop",
                    "--object-path", "/org/freedesktop/portal/desktop",
                    "--method", "org.freedesktop.portal.Screenshot.Screenshot",
                    "",
                    "{'interactive': <true>}",
                ])
                .status();

            if portal_res.map(|s| s.success()).unwrap_or(false) {
                return;
            }

            // 2. Try GNOME Screenshot area select
            if Command::new("gnome-screenshot")
                .args(["-a"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return;
            }

            // 3. Try KDE Spectacle rectangular region
            if Command::new("spectacle")
                .args(["-r"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return;
            }

            // 4. Try Flameshot GUI
            if Command::new("flameshot")
                .args(["gui"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return;
            }

            // 5. Try wl-clipboard / grim + slurp (generic Wayland)
            let _ = Command::new("sh")
                .args(["-c", "grim -g \"$(slurp)\" - | wl-copy"])
                .status();
        });
    }
}
