# Creating Custom Screensavers for Trance

Trance supports dynamic screensaver plugins compiled as Rust shared libraries (`.so` on Linux). This guide walks you through setting up, implementing, compiling, and previewing your own custom screensaver.

---

## 1. Project Setup

Create a new library project using cargo:

```bash
cargo new --lib screensaver_myplugin
```

Modify the project's `Cargo.toml` to configure it as a dynamic C-compatible library and add the `trance-api` dependency:

```toml
[package]
name = "screensaver_myplugin"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
name = "screensaver_myplugin"
crate-type = ["cdylib"]

[dependencies]
# Link to the trance-api dependency
trance-api = { git = "https://github.com/crateria/trance.git", branch = "main" }
```

---

## 2. Implementing the screensaver

In your `src/lib.rs`, implement the `trance_api::Screensaver` trait for your screensaver structure.

### Example Code:

```rust
use std::time::Duration;
use trance_api::{Screensaver, ScreensaverInstance, TerminalCell};

struct MyEffect {
    time: f32,
}

impl Screensaver for MyEffect {
    // Optional: Initialize state when screensaver starts
    fn init(&mut self, _cols: usize, _rows: usize) {
        self.time = 0.0;
    }

    // Update animation state over time
    fn update(&mut self, dt: Duration, _cols: usize, _rows: usize) {
        self.time += dt.as_secs_f32();
    }

    // Render cells to the terminal grid
    fn draw(&self, grid: &mut [TerminalCell], cols: usize, rows: usize) {
        if cols == 0 || rows == 0 || grid.is_empty() {
            return;
        }

        // Draw a sweeping wave of stars
        let wave_center = (self.time * 2.0).sin() * 0.5 + 0.5; // normalized 0.0 - 1.0
        let target_col = (wave_center * (cols as f32)) as usize;

        for y in 0..rows {
            for x in 0..cols {
                let cell = &mut grid[y * cols + x];
                if x == target_col {
                    cell.ch = '█';
                    cell.fg = (0, 255, 128); // Green
                } else if x == target_col.saturating_sub(1) || x == target_col + 1 {
                    cell.ch = '▒';
                    cell.fg = (0, 180, 100);
                } else {
                    cell.ch = ' ';
                    cell.fg = (0, 0, 0);
                }
                cell.bg = (0, 0, 0);
                cell.bold = false;
            }
        }
    }
}
```

---

## 3. Registering FFI Entrypoints

The Trance daemon loads plugins dynamically using FFI (Foreign Function Interface). You must export the following unmangled FFI functions at the bottom of your `src/lib.rs` to allow creation and destruction:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn create_screensaver() -> *mut ScreensaverInstance {
    let effect = MyEffect { time: 0.0 };
    Box::into_raw(Box::new(ScreensaverInstance {
        inner: Box::new(effect),
    }))
}

/// # Safety
/// The pointer `ptr` must be a valid pointer allocated by `create_screensaver` and not previously dropped.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_screensaver(ptr: *mut ScreensaverInstance) {
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);
        }
    }
}
```

---

## 4. Building Your Plugin

Compile the library in release mode:

```bash
cargo build --release
```

This will produce the compiled shared library:
`target/release/libscreensaver_myplugin.so`

---

## 5. Previewing and Running

### Development Preview
You can run and test your compiled `.so` plugin directly using the `trance` CLI command without installing it:

```bash
trance preview target/release/libscreensaver_myplugin.so
```

### Production Installation
To make the plugin permanently available to your `trance-daemon` session:
1. Copy the `.so` file to the user screensavers directory:
   ```bash
   mkdir -p ~/.local/share/trance/screensavers
   cp target/release/libscreensaver_myplugin.so ~/.local/share/trance/screensavers/
   ```
2. The daemon will automatically discover the new screensaver on next start or configuration reload.
