// SPDX-License-Identifier: MIT

use std::io::Write;

#[repr(C)]
pub struct PamHandle {
    _private: [u8; 0],
}

#[repr(C)]
pub struct pam_message {
    pub msg_style: libc::c_int,
    pub msg: *const libc::c_char,
}

#[repr(C)]
pub struct pam_response {
    pub resp: *mut libc::c_char,
    pub resp_retcode: libc::c_int,
}

pub type PamConvFn = unsafe extern "C" fn(
    num_msg: libc::c_int,
    msg: *mut *mut pam_message,
    resp: *mut *mut pam_response,
    appdata_ptr: *mut libc::c_void,
) -> libc::c_int;

#[repr(C)]
pub struct pam_conv {
    pub conv: Option<PamConvFn>,
    pub appdata_ptr: *mut libc::c_void,
}

pub const PAM_SUCCESS: libc::c_int = 0;
pub const PAM_PROMPT_ECHO_OFF: libc::c_int = 1;
pub const PAM_PROMPT_ECHO_ON: libc::c_int = 2;

#[link(name = "pam")]
unsafe extern "C" {
    pub fn pam_start(
        service_name: *const libc::c_char,
        user: *const libc::c_char,
        pam_conversation: *const pam_conv,
        pamh: *mut *mut PamHandle,
    ) -> libc::c_int;

    pub fn pam_authenticate(pamh: *mut PamHandle, flags: libc::c_int) -> libc::c_int;

    pub fn pam_end(pamh: *mut PamHandle, pam_status: libc::c_int) -> libc::c_int;
}

unsafe extern "C" fn simple_pam_conv(
    num_msg: libc::c_int,
    msg: *mut *mut pam_message,
    resp: *mut *mut pam_response,
    appdata_ptr: *mut libc::c_void,
) -> libc::c_int {
    if num_msg <= 0 {
        return PAM_SUCCESS;
    }

    unsafe {
        let resp_arr = libc::malloc(num_msg as usize * std::mem::size_of::<pam_response>()) as *mut pam_response;
        if resp_arr.is_null() {
            return 5; // PAM_BUF_ERR
        }

        std::ptr::write_bytes(resp_arr, 0, num_msg as usize);
        let password_ptr = appdata_ptr as *const libc::c_char;

        for i in 0..num_msg {
            let msg_ptr = *msg.add(i as usize);
            let msg_style = (*msg_ptr).msg_style;

            if msg_style == PAM_PROMPT_ECHO_OFF || msg_style == PAM_PROMPT_ECHO_ON {
                let dup_pw = libc::strdup(password_ptr);
                (*resp_arr.add(i as usize)).resp = dup_pw;
            } else {
                (*resp_arr.add(i as usize)).resp = std::ptr::null_mut();
            }
        }

        *resp = resp_arr;
    }
    PAM_SUCCESS
}

fn authenticate(user: &str, password: &str) -> bool {
    let c_service = std::ffi::CString::new("login").unwrap();
    let c_user = std::ffi::CString::new(user).unwrap();
    let c_password = std::ffi::CString::new(password).unwrap();

    let conv = pam_conv {
        conv: Some(simple_pam_conv),
        appdata_ptr: c_password.as_ptr() as *mut libc::c_void,
    };

    let mut pamh: *mut PamHandle = std::ptr::null_mut();
    unsafe {
        let res = pam_start(c_service.as_ptr(), c_user.as_ptr(), &conv, &mut pamh);
        if res != PAM_SUCCESS {
            return false;
        }

        let auth_res = pam_authenticate(pamh, 0);
        pam_end(pamh, auth_res);
        auth_res == PAM_SUCCESS
    }
}

fn read_password() -> std::io::Result<String> {
    use std::io::BufRead;

    let mut termios = unsafe {
        let mut t = std::mem::zeroed();
        libc::tcgetattr(libc::STDIN_FILENO, &mut t);
        t
    };

    let old_termios = termios;
    termios.c_lflag &= !libc::ECHO;

    unsafe {
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &termios);
    }

    let mut line = String::new();
    let stdin = std::io::stdin();
    stdin.lock().read_line(&mut line)?;

    unsafe {
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &old_termios);
    }
    println!();

    Ok(line.trim_end_matches('\n').trim_end_matches('\r').to_string())
}

pub fn run_failsafe_lock() -> anyhow::Result<()> {
    let username = unsafe {
        let uid = libc::getuid();
        let pwd = libc::getpwuid(uid);
        if !pwd.is_null() && !(*pwd).pw_name.is_null() {
            std::ffi::CStr::from_ptr((*pwd).pw_name)
                .to_string_lossy()
                .into_owned()
        } else {
            std::env::var("USER").unwrap_or_else(|_| "user".to_string())
        }
    };
    println!("\x1b[2J\x1b[H"); // Clear screen
    println!("============================================================");
    println!("trance: SCREENSAVER RUNNER CRASHED / EXITED UNEXPECTEDLY!");
    println!("SESSION IS LOCKED FOR SECURITY.");
    println!("============================================================");
    println!();

    loop {
        print!("Password for {}: ", username);
        std::io::stdout().flush()?;

        let password = read_password().unwrap_or_default();
        if authenticate(&username, &password) {
            println!("Authentication successful. Session unlocked.");
            break;
        } else {
            println!("Authentication failed. Please try again.");
            println!();
        }
    }
    Ok(())
}

pub fn spawn_failsafe_locker() -> Result<(), String> {
    let term_emulators = [
        "xterm",
        "foot",
        "gnome-terminal",
        "konsole",
        "kitty",
        "alacritty",
        "wezterm",
        "weston-terminal",
    ];

    let current_exe = std::env::current_exe()
        .map_err(|e| format!("failed to get current exe path: {e}"))?;

    let mut term_bin = None;
    for term in &term_emulators {
        if std::process::Command::new("which").arg(term).output().is_ok() {
            term_bin = Some(*term);
            break;
        }
    }

    let Some(term) = term_bin else {
        return Err("No terminal emulator found".to_string());
    };

    loop {
        tracing::info!("Spawning failsafe locker using {}...", term);
        let mut cmd = std::process::Command::new(term);
        if term == "gnome-terminal" || term == "konsole" {
            cmd.arg("--").arg(&current_exe).arg("failsafe-lock");
        } else {
            cmd.arg("-e").arg(&current_exe).arg("failsafe-lock");
        }

        let mut child = cmd.spawn().map_err(|e| format!("failed to spawn terminal: {e}"))?;
        let status = child.wait().map_err(|e| format!("failed to wait for locker: {e}"))?;

        if status.success() {
            tracing::info!("Failsafe locker successfully authenticated user.");
            break;
        } else {
            tracing::warn!("Failsafe locker exited without success. respawning...");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }

    Ok(())
}
