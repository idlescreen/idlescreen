# trance-applet

A native COSMIC Desktop panel applet for the [Trance screensaver suite](https://github.com/UberMetroid/trance).

## What it does

- Toggle screensaver idle activation on/off
- Adjust idle timeout
- Select active screensaver
- Quick preview of any installed saver
- Falls back to direct config file when the daemon is offline

## Build

```
cargo build --release -p trance-applet
```

The binary is installed at `/usr/bin/trance-applet` by the deb/rpm packages.

## Configuration

`~/.config/ubermetroid/theme.yaml` (shared with the daemon).

## See also

- [Main README](https://github.com/UberMetroid/trance)
- [D-Bus API](https://github.com/UberMetroid/trance#d-bus-api)