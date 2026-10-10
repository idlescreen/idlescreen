// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Socketpair tick receiver and sealed memfd frame renderer.

use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use crate::protocol::{IpcCommand, IpcResponse, recv_fd};
use crate::runner::TerminalCell;

pub fn run(saver_name: &str, mut socket: UnixStream) -> Result<(), Box<dyn std::error::Error>> {
    let mut saver = match crate::savers::create_saver(saver_name) {
        Some(s) => s,
        None => {
            return Err(format!("unknown screensaver: {saver_name}").into());
        }
    };

    // First, receive the sealed shared memory file descriptor from Core
    let raw_shm_fd = recv_fd(std::os::fd::AsRawFd::as_raw_fd(&socket))?;
    let shm_fd = unsafe { OwnedFd::from_raw_fd(raw_shm_fd) };

    // Signal readiness
    IpcResponse::Ready.write_to(&mut socket)?;

    let mut cols: usize = 0;
    let mut rows: usize = 0;
    let mut mapped_ptr: *mut TerminalCell = std::ptr::null_mut();
    let mut mapped_len: usize = 0;

    loop {
        let cmd = match IpcCommand::read_from(&mut socket) {
            Ok(c) => c,
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(e) => return Err(e.into()),
        };

        match cmd {
            IpcCommand::Init { cols: c, rows: r } => {
                let c = c as usize;
                let r = r as usize;
                if c == 0 || r == 0 || c > 1000 || r > 1000 {
                    return Err(format!("invalid grid dimensions: {c}x{r}").into());
                }

                if !mapped_ptr.is_null() && mapped_len > 0 {
                    unsafe {
                        libc::munmap(mapped_ptr.cast(), mapped_len);
                    }
                }

                cols = c;
                rows = r;
                let num_cells = cols.saturating_mul(rows);
                let needed_bytes = num_cells.saturating_mul(std::mem::size_of::<TerminalCell>());

                let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
                if unsafe {
                    libc::fstat(std::os::fd::AsRawFd::as_raw_fd(&shm_fd), stat.as_mut_ptr())
                } < 0
                {
                    return Err("fstat failed on shared memory".into());
                }
                let file_size = unsafe { stat.assume_init() }.st_size as usize;
                if needed_bytes > file_size {
                    return Err(format!(
                        "grid dimensions require {needed_bytes} bytes, but shm is only {file_size} bytes"
                    )
                    .into());
                }

                let ptr = unsafe {
                    libc::mmap(
                        std::ptr::null_mut(),
                        needed_bytes,
                        libc::PROT_READ | libc::PROT_WRITE,
                        libc::MAP_SHARED,
                        std::os::fd::AsRawFd::as_raw_fd(&shm_fd),
                        0,
                    )
                };

                if ptr == libc::MAP_FAILED {
                    return Err("mmap failed on shared memory".into());
                }

                mapped_ptr = ptr.cast();
                mapped_len = needed_bytes;

                IpcResponse::Ack.write_to(&mut socket)?;
            }
            IpcCommand::TickAndDraw { dt_micros } => {
                if mapped_ptr.is_null() || cols == 0 || rows == 0 {
                    IpcResponse::Ack.write_to(&mut socket)?;
                    continue;
                }

                let dt = Duration::from_micros(dt_micros);
                saver.update(dt, cols, rows);

                let num_cells = cols.saturating_mul(rows);
                let slice = unsafe { std::slice::from_raw_parts_mut(mapped_ptr, num_cells) };

                // Clear/reset cells to default space before drawing
                for cell in slice.iter_mut() {
                    *cell = TerminalCell::default();
                }

                saver.draw(slice, cols, rows);

                IpcResponse::FrameReady {
                    scanlines: false,
                    dirty: true,
                }
                .write_to(&mut socket)?;
            }
            IpcCommand::SetSimulationRate { .. } => {
                IpcResponse::Ack.write_to(&mut socket)?;
            }
            IpcCommand::Stop => {
                break;
            }
        }
    }

    if !mapped_ptr.is_null() && mapped_len > 0 {
        unsafe {
            libc::munmap(mapped_ptr.cast(), mapped_len);
        }
    }

    Ok(())
}
