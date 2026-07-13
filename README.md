<p align="center">
  <a href="https://crateria.github.io/">
    <img src="assets/crateria-header.jpg" alt="Crateria" width="100%">
  </a>
</p>

# Trance

[![CI](https://github.com/crateria/trance/actions/workflows/ci.yml/badge.svg)](https://github.com/crateria/trance/actions/workflows/ci.yml)

Trance is a modular, Wayland-native screensaver system for modern Linux desktops, with first-class integration for the COSMIC Desktop environment.

## Showcase

### Ripple
![Ripple Showcase](assets/ripple.webp)

### Beams
![Beams Showcase](assets/beams.webp)

## Packages Produced
This repository builds official `.deb` (Debian/Ubuntu/Pop!_OS) and `.rpm` (Fedora) packages:
*   `trance-daemon` (The core background screensaver daemon)
*   `trance-cli` (Control and command-line user interface - CUI)
*   `trance-tui` (Terminal-based status and configuration monitor - TUI)
*   `trance-applet` (Optional graphical desktop panel applet - GUI)
*   `trance-plugins-all` (Standard animation effects pack)

## Installation
Add the official Crateria repository and install the packages:

### Debian / Ubuntu / Pop!_OS
```bash
sudo mkdir -p /etc/apt/keyrings
sudo curl -fsSL https://crateria.github.io/packages/apt/crateria-keyring.gpg -o /etc/apt/keyrings/crateria.gpg
echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/crateria.gpg] https://crateria.github.io/packages/apt stable main" | sudo tee /etc/apt/sources.list.d/crateria.list
sudo apt update
sudo apt install trance-daemon trance-cli trance-tui trance-plugins-all
```

### Fedora
```bash
sudo curl -fsSL https://crateria.github.io/packages/rpm/crateria.repo -o /etc/yum.repos.d/crateria.repo
sudo dnf install trance-daemon trance-cli trance-tui trance-plugins-all
```

## How to Use
*   **Daemon (Background Service)**: Run `trance-daemon` directly or enable its systemd user service.
*   **CLI (Command Line)**:
    ```bash
    trance-cli start
    trance-cli stop
    trance-cli next
    ```
*   **TUI (Terminal Monitor)**: Run `trance-tui` to view active status.
*   **GUI (COSMIC Desktop Applet)**: Enable the Trance applet under Desktop Panel applets settings.
