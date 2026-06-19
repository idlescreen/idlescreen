pub mod app;
pub mod ui;

/// Returns the list of detected screensavers
pub fn list_screensavers() -> Vec<String> {
    trance_runner::discovery::detect_screensavers()
}

/// Spawns a screensaver using the launcher in Preview mode
pub fn start_screensaver(name: &str) -> std::io::Result<()> {
    use trance_runner::launcher::{launch_screensaver, LaunchMode};
    match launch_screensaver(name, LaunchMode::Preview) {
        Ok(mut child) => {
            let _ = child.wait();
            Ok(())
        }
        Err(e) => {
            eprintln!("failed to launch screensaver '{}': {}", name, e);
            Err(e)
        }
    }
}

/// Helper stub to stop active screensavers by killing their instances
pub fn stop_screensavers() -> std::io::Result<()> {
    println!("stop screensavers: not yet implemented.");
    Ok(())
}

/// Self-Repair Diagnostics
pub fn run_diagnostics(do_fix: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("running doctor checks on local76 screensavers...");

    // 1. Check daemon config
    let config_path = std::env::var("HOME")
        .map(|h| std::path::PathBuf::from(h).join(".config").join("local76").join("theme.yaml"))
        .ok();
    
    if let Some(ref path) = config_path {
        if path.exists() {
            println!("- checking daemon config: OK ({})", path.display());
        } else {
            println!("- checking daemon config: WARNING (Config file does not exist at {})", path.display());
            if do_fix {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let default_cfg = "idle_enabled: true\nidle_timeout_mins: 5\nactive_saver: \"beams\"\n";
                if std::fs::write(path, default_cfg).is_ok() {
                    println!("  -> [FIX] Created default config at theme.yaml");
                }
            }
        }
    } else {
        println!("- checking daemon config: FAILED (Could not determine HOME directory)");
    }

    // 2. Check xterm
    let xterm_check = std::process::Command::new("xterm")
        .arg("-version")
        .output();
    
    match xterm_check {
        Ok(output) if output.status.success() => {
            println!("- checking xterm installation: OK");
        }
        _ => {
            println!("- checking xterm installation: FAILED (xterm is not installed but is required for displaying screensavers)");
            if do_fix {
                println!("  -> [FIX] Please run 'sudo apt install xterm' on Debian/Ubuntu systems to install xterm.");
            }
        }
    }

    // 3. Check busctl / logind access
    let busctl_check = std::process::Command::new("busctl")
        .arg("status")
        .output();
    
    match busctl_check {
        Ok(output) if output.status.success() => {
            println!("- checking systemd-logind bus accessibility: OK");
        }
        _ => {
            println!("- checking systemd-logind bus accessibility: WARNING (busctl command failed or system DBus is unreachable)");
            println!("  -> Idle detection might not function correctly if systemd-logind is inaccessible.");
        }
    }

    // 4. Check screensaver binaries
    let detected = list_screensavers();
    if detected.is_empty() {
        println!("- checking screensaver binaries: WARNING (No screensaver binaries found in search paths)");
    } else {
        println!("- checking screensaver binaries: OK (Found {} screensavers: {:?})", detected.len(), detected);
    }

    if do_fix {
        println!("all repairs completed.");
    }
    Ok(())
}
