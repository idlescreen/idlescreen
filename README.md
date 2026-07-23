<h1 align="center">
  <img src="assets/icon.png?v=1.0.31" width="48" height="48" valign="middle"> Trance
</h1>

<p align="center">
  <b>Modern modular Wayland screensaver and ambient display daemon for Linux written in Rust.</b>
</p>

---

### Instant One-Line Install (Native Package Manager)

On Debian, Ubuntu, Fedora, or RHEL:

```bash
curl -fsSL https://studio2201.github.io/packages/install.sh | sudo bash
```

---

### Unraid NAS & Linux Container Deployment

Run the official zero-dependency container:

```bash
docker run -d --name trance --net=host -v /tmp/.X11-unix:/tmp/.X11-unix ghcr.io/studio2201/trance:latest
```

---

### Environment Configuration

The daemon service can be customized using the following environment variables:

| Variable | Description | Default |
| :--- | :--- | :---: |
| `TRANCE_IDLE_TIMEOUT_MINS` | Minutes of inactivity before screensaver activates | `10` |
| `TRANCE_ACTIVE_SAVER` | Active plugin name (e.g. `beams`, `matrix`, `flurry`) | `beams` |
| `TRANCE_SHOW_FPS` | Display real-time FPS overlay | `false` |
| `LOG_LEVEL` | Tracing filter (`error`, `warn`, `info`, `debug`) | `info` |

---

### Administration CLI & Control Utility

Every installation includes the `trance-cli` control binary.

CLI Command Reference:
- `trance-cli status` — Displays screensaver state and active plugin.
- `trance-cli enable` — Enables automatic idle screensaver.
- `trance-cli disable` — Disables automatic idle screensaver.
- `trance-cli preview <plugin>` — Runs a full-screen preview of a specific screensaver.

---

### Architecture & Security

- **Native Wayland Integration**: Built on `ext-idle-notify-v1` and `ext-session-lock-v1` protocols.
- **GPU Accelerated Cell Renderer**: High-efficiency wgpu rendering pipeline for cell-based visualizers.
- **Fail-Safe PAM Authentication**: Secure screen lock integration with local PAM fallback authentication.

---

### License

Distributed under the Apache 2.0 License. See [LICENSE](LICENSE) for details.
