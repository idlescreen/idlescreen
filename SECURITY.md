# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.3.x   | :white_check_mark: |
| < 0.3   | :x:                |

## Hardening notes (plugin load & D-Bus)

* Screensaver plugins are **allowlisted** by basename and loaded only from
  trusted directory trees (system paths preferred over `~/.local`).
* World-writable plugin libraries (`o+w`) are refused at resolve time.
* D-Bus control methods prefer a trusted peer basename
  (`trance`, `trance-applet`, `trance-tui`, `trance-cli`) under
  `/usr/bin` or `/usr/local/bin` (debug builds allow same-directory peers).
* When `/proc/<peer>/exe` is unreadable (common under systemd hardening such as
  `ProtectSystem=strict`), control is allowed if the D-Bus peer’s Unix UID
  matches the daemon’s (session-bus threat model).
* `TRANCE_DBUS_TRUST_ALL=1` is honored only in **debug** builds.

## Reporting a Vulnerability

Please report security vulnerabilities via GitHub Issues or email the maintainer
directly. Do not disclose vulnerabilities publicly until a fix is available.

We aim to acknowledge reports within 72 hours and provide a fix or mitigation
within 30 days for critical issues.