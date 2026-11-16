#!/usr/bin/env bash
set -e

echo "=============================================="
echo "      SidekickMC Linux Automated Installer    "
echo "=============================================="

# Check for root/sudo
if [ "$EUID" -eq 0 ]; then
    echo "Please do not run this script directly as root. Run as your regular desktop user."
    echo "The script will invoke sudo when needed."
    exit 1
fi

echo "[1/5] Checking dependencies..."
if command -v apt-get &>/dev/null; then
    echo "Detected Debian/Ubuntu system."
    sudo apt-get update -y
    sudo apt-get install -y build-essential cargo rustc pkg-config libudev-dev brightnessctl
elif command -v dnf &>/dev/null; then
    echo "Detected Fedora/RHEL system."
    sudo dnf install -y gcc cargo rust systemd-devel brightnessctl
elif command -v pacman &>/dev/null; then
    echo "Detected Arch Linux system."
    sudo pacman -S --needed --noconfirm base-devel rust brightnessctl
else
    echo "Please ensure cargo, rustc, and brightnessctl are installed."
fi

echo "[2/5] Building high-performance release binary..."
cd "$(dirname "$0")"
cargo build --release

echo "[3/5] Installing binary to /usr/local/bin..."
sudo install -m 755 target/release/sidekickmc /usr/local/bin/sidekickmc

echo "[4/5] Setting up udev permissions for mouse access..."
sudo install -m 644 udev/99-sidekickmc.rules /etc/udev/rules.d/99-sidekickmc.rules
sudo udevadm control --reload-rules
sudo udevadm trigger
sudo modprobe uinput || true

# Add user to input group if present
if getent group input >/dev/null; then
    sudo usermod -aG input "$USER"
fi

echo "[5/5] Configuring systemd user service..."
mkdir -p "$HOME/.config/systemd/user"
install -m 644 systemd/sidekickmc.service "$HOME/.config/systemd/user/sidekickmc.service"
systemctl --user daemon-reload
systemctl --user enable --now sidekickmc

echo ""
echo "=============================================="
echo "    SidekickMC Successfully Installed!        "
echo "=============================================="
echo "Status check:  systemctl --user status sidekickmc"
echo "Device list:   sidekickmc --list-devices"
echo "Configuration: sidekickmc config --show"
echo ""
echo "Note: If mouse events are not captured immediately,"
echo "log out and log back in for the 'input' group to take effect."
