mod actions;
mod config;
mod input;
mod state;

use clap::{Parser, Subcommand};
use config::Config;
use input::MouseManager;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "sidekickmc", author = "SidekickMC Contributors", version = "1.0.0", about = "Linux mouse gesture daemon: Volume, brightness, and screen snipping with mouse side buttons.")]
struct Cli {
    /// Specific /dev/input/event* path to use
    #[arg(short, long)]
    device: Option<PathBuf>,

    /// List all input devices and identify compatible mice
    #[arg(short, long)]
    list_devices: bool,

    /// Install udev rules for unprivileged access to /dev/uinput and /dev/input/event*
    #[arg(long)]
    install_udev: bool,

    /// Install and enable systemd user service for auto-start
    #[arg(long)]
    install_service: bool,

    /// Run daemon in background (fork/detach)
    #[arg(short, long)]
    daemon: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show or update configuration
    Config {
        /// Set back button action (volume, brightness, none)
        #[arg(long)]
        back: Option<String>,
        /// Set forward button action (volume, brightness, none)
        #[arg(long)]
        forward: Option<String>,
        /// Set chord action (snipping, none)
        #[arg(long)]
        chord: Option<String>,
        /// Display current configuration file path and contents
        #[arg(long)]
        show: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    if cli.list_devices {
        println!("\n=== SidekickMC Input Device Scanner ===");
        let devices = MouseManager::list_devices();
        if devices.is_empty() {
            println!("No input devices detected or permission denied (are you in the 'input' group?).");
        } else {
            for dev in devices {
                let status = if dev.has_side_buttons && dev.has_wheel {
                    "[COMPATIBLE MOUSE (Side buttons + Wheel)]"
                } else if dev.has_wheel {
                    "[Standard Mouse (Wheel only)]"
                } else {
                    "[Other Device]"
                };
                println!("- {:<22} {:<35} {}", dev.path.display(), dev.name, status);
            }
        }
        println!();
        return Ok(());
    }

    if cli.install_udev {
        install_udev_rules()?;
        return Ok(());
    }

    if cli.install_service {
        install_systemd_service()?;
        return Ok(());
    }

    let mut config = Config::load();

    if let Some(Commands::Config { back, forward, chord, show }) = cli.command {
        let mut modified = false;
        if let Some(b) = back {
            match b.to_lowercase().as_str() {
                "volume" => { config.back_action = config::Action::Volume; modified = true; }
                "brightness" => { config.back_action = config::Action::Brightness; modified = true; }
                "none" => { config.back_action = config::Action::None; modified = true; }
                _ => eprintln!("Invalid action '{}'. Valid: volume, brightness, none", b),
            }
        }
        if let Some(f) = forward {
            match f.to_lowercase().as_str() {
                "volume" => { config.forward_action = config::Action::Volume; modified = true; }
                "brightness" => { config.forward_action = config::Action::Brightness; modified = true; }
                "none" => { config.forward_action = config::Action::None; modified = true; }
                _ => eprintln!("Invalid action '{}'. Valid: volume, brightness, none", f),
            }
        }
        if let Some(c) = chord {
            match c.to_lowercase().as_str() {
                "snipping" | "snippingtool" | "snip" => { config.chord_action = config::ChordAction::SnippingTool; modified = true; }
                "none" | "off" => { config.chord_action = config::ChordAction::None; modified = true; }
                _ => eprintln!("Invalid chord action '{}'. Valid: snipping, none", c),
            }
        }
        if modified {
            config.save()?;
            println!("Configuration updated successfully.");
        }
        if show || !modified {
            println!("\nConfiguration file: {}", Config::config_path().display());
            println!("{}", toml::to_string_pretty(&config)?);
        }
        return Ok(());
    }

    if cli.daemon {
        daemonize()?;
    }

    let mut manager = MouseManager::new(config.clone());
    let target = match cli.device {
        Some(p) => p,
        None => match manager.find_target_device() {
            Some(p) => p,
            None => {
                eprintln!("\nError: No compatible mouse with side buttons was detected.");
                eprintln!("Run 'sidekickmc --list-devices' to inspect available devices, or specify '--device /dev/input/eventX'.\n");
                std::process::exit(1);
            }
        },
    };

    println!("Starting SidekickMC on device: {}", target.display());
    println!("Back button action:    {:?}", config.back_action);
    println!("Forward button action: {:?}", config.forward_action);
    println!("Chording action:       {:?}", config.chord_action);

    if let Err(e) = manager.run_loop(&target) {
        eprintln!("SidekickMC terminated with error: {}", e);
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            eprintln!("\nPermission denied accessing /dev/input or /dev/uinput.");
            eprintln!("Run 'sudo sidekickmc --install-udev' to set up required permissions.\n");
        }
        std::process::exit(1);
    }

    Ok(())
}

fn install_udev_rules() -> std::io::Result<()> {
    let rules = r#"# Udev rules for SidekickMC
# Allows access to uinput and input event devices for logged-in desktop users
KERNEL=="uinput", SUBSYSTEM=="misc", TAG+="uaccess", MODE="0660", GROUP="input"
KERNEL=="event*", SUBSYSTEM=="input", TAG+="uaccess", MODE="0660", GROUP="input"
"#;
    let target_path = "/etc/udev/rules.d/99-sidekickmc.rules";
    println!("Installing udev rules to {}...", target_path);

    if let Err(e) = fs::write(target_path, rules) {
        eprintln!("Failed to write {}: {}. Did you run with 'sudo'?", target_path, e);
        return Err(e);
    }

    let _ = Command::new("udevadm").args(["control", "--reload-rules"]).status();
    let _ = Command::new("udevadm").args(["trigger"]).status();
    let _ = Command::new("modprobe").args(["uinput"]).status();

    println!("Udev rules installed successfully.");
    println!("Note: If your user is not in the 'input' group, run: sudo usermod -aG input $USER");
    Ok(())
}

fn install_systemd_service() -> std::io::Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let service_dir = PathBuf::from(&home).join(".config/systemd/user");
    fs::create_dir_all(&service_dir)?;

    let exe_path = std::env::current_exe()?;
    let exe_str = exe_path.to_string_lossy();

    let service_content = format!(
        r#"[Unit]
Description=SidekickMC Mouse Gestures Daemon
After=graphical-session.target

[Service]
Type=simple
ExecStart={}
Restart=always
RestartSec=3

[Install]
WantedBy=default.target
"#,
        exe_str
    );

    let service_file = service_dir.join("sidekickmc.service");
    fs::write(&service_file, service_content)?;
    println!("Installed systemd user service to {}", service_file.display());

    let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();
    let _ = Command::new("systemctl").args(["--user", "enable", "--now", "sidekickmc"]).status();
    println!("Service enabled and started! Check status with: systemctl --user status sidekickmc");

    Ok(())
}

fn daemonize() -> Result<(), Box<dyn std::error::Error>> {
    use nix::unistd::{fork, ForkResult};
    match unsafe { fork() }? {
        ForkResult::Parent { .. } => {
            std::process::exit(0);
        }
        ForkResult::Child => {
            // Child process continues in background
            Ok(())
        }
    }
}
