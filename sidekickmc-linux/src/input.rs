use std::collections::HashSet;
use std::path::{Path, PathBuf};
use evdev::{
    uinput::{VirtualDevice, VirtualDeviceBuilder},
    AttributeSet, Device, EventType, InputEvent, Key, RelativeAxisType,
};
use crate::actions::Actions;
use crate::config::{Action, Config};
use crate::state::{GestureEngine, GestureResult, SideButton};

pub struct MouseManager {
    config: Config,
    engine: GestureEngine,
}

pub struct DeviceInfo {
    pub path: PathBuf,
    pub name: String,
    pub has_side_buttons: bool,
    pub has_wheel: bool,
}

impl MouseManager {
    pub fn new(config: Config) -> Self {
        let engine = GestureEngine::new(config.clone());
        Self { config, engine }
    }

    /// List all input devices and identify compatible mice
    pub fn list_devices() -> Vec<DeviceInfo> {
        let mut list = Vec::new();
        for (path, dev) in evdev::enumerate() {
            let name = dev.name().unwrap_or("Unknown Device").to_string();
            let keys = dev.supported_keys();
            let axes = dev.supported_relative_axes();

            let has_side_buttons = keys.map(|k| {
                k.contains(Key::BTN_SIDE)
                    || k.contains(Key::BTN_EXTRA)
                    || k.contains(Key::BTN_BACK)
                    || k.contains(Key::BTN_FORWARD)
            }).unwrap_or(false);

            let has_wheel = axes.map(|a| {
                a.contains(RelativeAxisType::REL_WHEEL)
                    || a.contains(RelativeAxisType::REL_WHEEL_HI_RES)
            }).unwrap_or(false);

            list.push(DeviceInfo {
                path,
                name,
                has_side_buttons,
                has_wheel,
            });
        }
        list
    }

    /// Auto-detect the best matching mouse with side buttons and a scroll wheel
    pub fn find_target_device(&self) -> Option<PathBuf> {
        if let Some(target) = &self.config.target_device {
            let path = PathBuf::from(target);
            if path.exists() {
                return Some(path);
            }
            // Match by name
            for (p, dev) in evdev::enumerate() {
                if let Some(name) = dev.name() {
                    if name.to_lowercase().contains(&target.to_lowercase()) {
                        return Some(p);
                    }
                }
            }
        }

        // Auto-detect: Pick first device that has both side buttons and a wheel
        for (path, dev) in evdev::enumerate() {
            let keys = dev.supported_keys();
            let axes = dev.supported_relative_axes();

            let has_side = keys.map(|k| {
                k.contains(Key::BTN_SIDE) || k.contains(Key::BTN_EXTRA) || k.contains(Key::BTN_BACK) || k.contains(Key::BTN_FORWARD)
            }).unwrap_or(false);

            let has_wheel = axes.map(|a| {
                a.contains(RelativeAxisType::REL_WHEEL) || a.contains(RelativeAxisType::REL_WHEEL_HI_RES)
            }).unwrap_or(false);

            if has_side && has_wheel {
                return Some(path);
            }
        }
        None
    }

    /// Create the virtual device that replicates mouse clicks/movements and adds multimedia keys
    fn create_virtual_device(source: &Device) -> std::io::Result<VirtualDevice> {
        let mut keys = AttributeSet::<Key>::new();
        // Clone source keys if available, or add standard mouse keys
        if let Some(src_keys) = source.supported_keys() {
            for k in src_keys.iter() {
                keys.insert(k);
            }
        }
        keys.insert(Key::BTN_LEFT);
        keys.insert(Key::BTN_RIGHT);
        keys.insert(Key::BTN_MIDDLE);
        keys.insert(Key::BTN_SIDE);
        keys.insert(Key::BTN_EXTRA);
        keys.insert(Key::BTN_BACK);
        keys.insert(Key::BTN_FORWARD);
        // Multimedia keys for native OSD volume control
        keys.insert(Key::KEY_VOLUMEUP);
        keys.insert(Key::KEY_VOLUMEDOWN);
        keys.insert(Key::KEY_MUTE);

        let mut rel_axes = AttributeSet::<RelativeAxisType>::new();
        if let Some(src_axes) = source.supported_relative_axes() {
            for a in src_axes.iter() {
                rel_axes.insert(a);
            }
        }
        rel_axes.insert(RelativeAxisType::REL_X);
        rel_axes.insert(RelativeAxisType::REL_Y);
        rel_axes.insert(RelativeAxisType::REL_WHEEL);
        rel_axes.insert(RelativeAxisType::REL_HWHEEL);
        rel_axes.insert(RelativeAxisType::REL_WHEEL_HI_RES);

        VirtualDeviceBuilder::new()?
            .name("SidekickMC Virtual Input")
            .with_keys(&keys)?
            .with_relative_axes(&rel_axes)?
            .build()
    }

    /// Main event loop: grab mouse, process gestures, and pass through other events
    pub fn run_loop(&mut self, device_path: &Path) -> std::io::Result<()> {
        let mut physical = Device::open(device_path)?;
        let name = physical.name().unwrap_or("Unknown Mouse").to_string();
        log::info!("Opened mouse: {} ({})", name, device_path.display());

        // Grab device so we have exclusive interception rights
        physical.grab()?;
        log::info!("Device grabbed exclusively. Creating virtual passthrough device...");

        let mut virtual_dev = Self::create_virtual_device(&physical)?;
        log::info!("SidekickMC is active! Gestures are running.");

        loop {
            let events = match physical.fetch_events() {
                Ok(evts) => evts,
                Err(e) => {
                    log::error!("Error reading events: {}. Mouse may have been unplugged.", e);
                    break;
                }
            };

            for ev in events {
                match ev.event_type() {
                    EventType::KEY => {
                        let code = ev.code();
                        let value = ev.value(); // 0 = release, 1 = press, 2 = repeat
                        let is_press = value == 1;
                        let is_release = value == 0;

                        let side_btn = if code == Key::BTN_SIDE.0 || code == Key::BTN_BACK.0 {
                            Some(SideButton::Back)
                        } else if code == Key::BTN_EXTRA.0 || code == Key::BTN_FORWARD.0 {
                            Some(SideButton::Forward)
                        } else {
                            None
                        };

                        if let Some(btn) = side_btn {
                            if is_press || is_release {
                                match self.engine.handle_button(btn, is_press) {
                                    GestureResult::Consumed => {
                                        // Absorbed by gesture engine
                                    }
                                    GestureResult::PassThrough => {
                                        let _ = virtual_dev.emit(&[ev]);
                                    }
                                    GestureResult::EmitTap(tapped_btn) => {
                                        let key = match tapped_btn {
                                            SideButton::Back => Key::BTN_SIDE,
                                            SideButton::Forward => Key::BTN_EXTRA,
                                        };
                                        // Synthetic press + syn + release + syn
                                        let press = InputEvent::new(EventType::KEY, key.0, 1);
                                        let release = InputEvent::new(EventType::KEY, key.0, 0);
                                        let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
                                        let _ = virtual_dev.emit(&[press, syn, release, syn]);
                                    }
                                    GestureResult::TriggerChord => {
                                        log::info!("Chording recognized: Triggering screen snip.");
                                        Actions::open_screen_snipping();
                                    }
                                    _ => {}
                                }
                                continue;
                            }
                        }

                        // Pass other keys/buttons through
                        let _ = virtual_dev.emit(&[ev]);
                    }
                    EventType::RELATIVE => {
                        let code = ev.code();
                        let value = ev.value();

                        if code == RelativeAxisType::REL_WHEEL.0 || code == RelativeAxisType::REL_WHEEL_HI_RES.0 {
                            match self.engine.handle_wheel(value) {
                                GestureResult::Consumed => {
                                    // Wheel tick consumed during gesture! Do not forward to virtual mouse.
                                }
                                GestureResult::TriggerAction(action, steps) => {
                                    match action {
                                        Action::Volume => {
                                            // Emit native Linux volume key to trigger desktop OSD
                                            let key = if steps > 0 { Key::KEY_VOLUMEUP } else { Key::KEY_VOLUMEDOWN };
                                            for _ in 0..steps.abs() {
                                                let press = InputEvent::new(EventType::KEY, key.0, 1);
                                                let release = InputEvent::new(EventType::KEY, key.0, 0);
                                                let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
                                                let _ = virtual_dev.emit(&[press, syn, release, syn]);
                                            }
                                            // Also invoke mixer adjustment for environments without OSD
                                            Actions::adjust_volume_mixer(steps, self.config.volume_step_pct);
                                        }
                                        Action::Brightness => {
                                            Actions::adjust_brightness(steps, self.config.brightness_step_pct);
                                        }
                                        Action::None => {}
                                    }
                                }
                                GestureResult::PassThrough => {
                                    let _ = virtual_dev.emit(&[ev]);
                                }
                                _ => {
                                    let _ = virtual_dev.emit(&[ev]);
                                }
                            }
                            continue;
                        }

                        // Pass other axes (mouse movement X/Y) through instantly
                        let _ = virtual_dev.emit(&[ev]);
                    }
                    _ => {
                        // Pass synchronization and other events
                        let _ = virtual_dev.emit(&[ev]);
                    }
                }
            }
        }

        let _ = physical.ungrab();
        Ok(())
    }
}
