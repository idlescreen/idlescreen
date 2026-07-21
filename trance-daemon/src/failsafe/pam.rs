// SPDX-License-Identifier: MIT

use libloading::Library;
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

type FnPamStart = unsafe extern "C" fn(
    *const libc::c_char,
    *const libc::c_char,
    *const pam_conv,
    *mut *mut PamHandle,
) -> libc::c_int;
type FnPamAuth = unsafe extern "C" fn(*mut PamHandle, libc::c_int) -> libc::c_int;
type FnPamEnd = unsafe extern "C" fn(*mut PamHandle, libc::c_int) -> libc::c_int;

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
        let resp_arr = libc::malloc(num_msg as usize * std::mem::size_of::<pam_response>())
            as *mut pam_response;
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

pub fn authenticate(user: &str, password: &str) -> bool {
    let lib = match unsafe { Library::new("libpam.so.1").or_else(|_| Library::new("libpam.so")) } {
        Ok(l) => l,
        Err(_) => return false,
    };

    let pam_start: libloading::Symbol<FnPamStart> = match unsafe { lib.get(b"pam_start\0") } {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pam_authenticate: libloading::Symbol<FnPamAuth> =
        match unsafe { lib.get(b"pam_authenticate\0") } {
            Ok(s) => s,
            Err(_) => return false,
        };
    let pam_end: libloading::Symbol<FnPamEnd> = match unsafe { lib.get(b"pam_end\0") } {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pam_services = [
        "trance",
        "system-auth",
        "common-auth",
        "system-lock",
        "login",
    ];
    let c_user = std::ffi::CString::new(user).unwrap();
    let c_password = std::ffi::CString::new(password).unwrap();

    let conv = pam_conv {
        conv: Some(simple_pam_conv),
        appdata_ptr: c_password.as_ptr() as *mut libc::c_void,
    };

    for service in pam_services {
        if let Ok(c_service) = std::ffi::CString::new(service) {
            let mut pamh: *mut PamHandle = std::ptr::null_mut();
            unsafe {
                let res = pam_start(c_service.as_ptr(), c_user.as_ptr(), &conv, &mut pamh);
                if res == PAM_SUCCESS {
                    let auth_res = pam_authenticate(pamh, 0);
                    pam_end(pamh, auth_res);
                    if auth_res == PAM_SUCCESS {
                        return true;
                    }
                }
            }
        }
    }
    false
}

pub fn read_password() -> std::io::Result<String> {
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

    Ok(line
        .trim_end_matches('\n')
        .trim_end_matches('\r')
        .to_string())
}
