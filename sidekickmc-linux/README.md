# SidekickMC for Linux (Ubuntu & other distros)

A high-performance, ultra-low-footprint Linux daemon written in **Rust** that ports the Windows [SidekickMC](../SidekickMC/README.md) gesture system to modern Linux (Ubuntu 22.04/24.04+, Fedora, Arch, etc.).

Hold your mouse's side buttons (Back / Forward) and scroll to control system volume or laptop screen brightness. Press both together to trigger screen snipping. Tap either button to send normal browser navigation.

---

## Gestures

| Gesture | Default Action | Technical Mechanism |
| :--- | :--- | :--- |
| **Hold Back + Scroll Wheel** | Adjust system volume | Emits `KEY_VOLUMEUP` / `KEY_VOLUMEDOWN` via virtual `uinput` keyboard (triggers native GNOME / KDE / Ubuntu on-screen display flyout) + PipeWire/Pulse fallback |
| **Hold Forward + Scroll Wheel** | Adjust laptop brightness in steps of 5% | Direct write to `/sys/class/backlight/` with automatic fallback to `brightnessctl` |
| **Press both buttons within 300 ms** | Open screen snipping tool | Invokes XDG Desktop Portal `org.freedesktop.portal.Screenshot` (native interactive capture on Wayland & X11) or `gnome-screenshot` / `spectacle` |
| **Tap either button on its own** | Normal browser Back / Forward click | Emits `BTN_SIDE` / `BTN_EXTRA` cleanly via virtual mouse |

The scroll wheel is consumed during gestures so underlying applications (browsers, documents) do not scroll while you adjust volume or brightness.

---

## Architecture & Performance Comparison

| Metric / Layer | Original Windows App | SidekickMC for Linux (Rust) |
| :--- | :--- | :--- |
| **Language & Runtime** | C# (.NET Framework 4.x) | Rust (Zero runtime dependencies, standalone native ELF) |
| **RAM Usage** | ~15 – 25 MB | **~3 – 6 MB** |
| **Binary Size** | ~380 KB | **~1.5 MB** (fully stripped, statically optimized) |
| **Input Hooking** | Win32 `SetWindowsHookEx(WH_MOUSE_LL)` | Linux Kernel `evdev` (`EVIOCGRAB`) + `/dev/uinput` passthrough |
| **Display Server** | Win32 Desktop Window Manager | **Compatible with both Wayland and X11** |
| **Startup / Lifecycle** | Windows Registry `Run` key | Systemd User Service (`systemd --user`) & XDG Autostart |

---

## Quick Install (Ubuntu & Debian)

Run the automated installer script:

```bash
chmod +x install.sh
./install.sh
```

The script will:
1. Install build tools (`cargo`, `rustc`, `libudev-dev`, `brightnessctl`).
2. Build the optimized release binary with Link-Time Optimization (`lto = true`).
3. Install the binary to `/usr/local/bin/sidekickmc`.
4. Install `/etc/udev/rules.d/99-sidekickmc.rules` so you can run without root.
5. Enable and start the systemd user service (`systemctl --user enable --now sidekickmc`).

---

## Manual Build & Run

### Prerequisites (Ubuntu / Debian)
```bash
sudo apt update
sudo apt install build-essential cargo rustc pkg-config libudev-dev brightnessctl
```

### Build
```bash
cargo build --release
```
The optimized standalone binary will be at `target/release/sidekickmc`.

### Test Run
Inspect detected mice:
```bash
./target/release/sidekickmc --list-devices
```

Run in foreground:
```bash
./target/release/sidekickmc
```

---

## Configuration & Customization

Settings are stored in `~/.config/sidekickmc/config.toml`. You can configure actions via CLI:

```bash
# View current configuration
sidekickmc config --show

# Swap buttons: Back for Brightness, Forward for Volume
sidekickmc config --back brightness --forward volume

# Disable screen snipping chord
sidekickmc config --chord none
```

Example `config.toml`:
```toml
back_action = "Volume"
forward_action = "Brightness"
chord_action = "SnippingTool"
chord_window_ms = 300
brightness_step_pct = 5
volume_step_pct = 5
target_device = ""
```

---

## Troubleshooting

1. **Permission denied on `/dev/input/event*` or `/dev/uinput`**:
   Install the udev rules and add your user to the `input` group:
   ```bash
   sudo sidekickmc --install-udev
   sudo usermod -aG input $USER
   ```
   Then log out and log back in.

2. **Check daemon logs**:
   ```bash
   systemctl --user status sidekickmc
   journalctl --user -u sidekickmc -f
   ```
