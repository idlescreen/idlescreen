// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Compact binary IPC protocol definitions and file-descriptor passing helpers.

#![allow(unsafe_code)]

use std::io::{Read, Write};
use std::os::fd::RawFd;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IpcCommand {
    Init { cols: u32, rows: u32 },
    TickAndDraw { dt_micros: u64 },
    SetSimulationRate { hz: f32 },
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IpcResponse {
    Ready,
    FrameReady { scanlines: bool, dirty: bool },
    Ack,
}

impl IpcCommand {
    pub fn write_to<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        match self {
            IpcCommand::Init { cols, rows } => {
                writer.write_all(&[0])?;
                writer.write_all(&cols.to_le_bytes())?;
                writer.write_all(&rows.to_le_bytes())?;
            }
            IpcCommand::TickAndDraw { dt_micros } => {
                writer.write_all(&[1])?;
                writer.write_all(&dt_micros.to_le_bytes())?;
            }
            IpcCommand::SetSimulationRate { hz } => {
                writer.write_all(&[2])?;
                writer.write_all(&hz.to_le_bytes())?;
            }
            IpcCommand::Stop => {
                writer.write_all(&[3])?;
            }
        }
        writer.flush()?;
        Ok(())
    }

    pub fn read_from<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let mut tag = [0u8; 1];
        reader.read_exact(&mut tag)?;
        match tag[0] {
            0 => {
                let mut cols_bytes = [0u8; 4];
                let mut rows_bytes = [0u8; 4];
                reader.read_exact(&mut cols_bytes)?;
                reader.read_exact(&mut rows_bytes)?;
                let cols = u32::from_le_bytes(cols_bytes);
                let rows = u32::from_le_bytes(rows_bytes);
                Ok(IpcCommand::Init { cols, rows })
            }
            1 => {
                let mut dt_bytes = [0u8; 8];
                reader.read_exact(&mut dt_bytes)?;
                Ok(IpcCommand::TickAndDraw {
                    dt_micros: u64::from_le_bytes(dt_bytes),
                })
            }
            2 => {
                let mut hz_bytes = [0u8; 4];
                reader.read_exact(&mut hz_bytes)?;
                Ok(IpcCommand::SetSimulationRate {
                    hz: f32::from_le_bytes(hz_bytes),
                })
            }
            3 => Ok(IpcCommand::Stop),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid command tag",
            )),
        }
    }
}

impl IpcResponse {
    pub fn write_to<W: Write>(&self, mut writer: W) -> std::io::Result<()> {
        match self {
            IpcResponse::Ready => {
                writer.write_all(&[0])?;
            }
            IpcResponse::FrameReady { scanlines, dirty } => {
                writer.write_all(&[1, u8::from(*scanlines), u8::from(*dirty)])?;
            }
            IpcResponse::Ack => {
                writer.write_all(&[2])?;
            }
        }
        writer.flush()?;
        Ok(())
    }

    pub fn read_from<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let mut tag = [0u8; 1];
        reader.read_exact(&mut tag)?;
        match tag[0] {
            0 => Ok(IpcResponse::Ready),
            1 => {
                let mut flags = [0u8; 2];
                reader.read_exact(&mut flags)?;
                Ok(IpcResponse::FrameReady {
                    scanlines: flags[0] != 0,
                    dirty: flags[1] != 0,
                })
            }
            2 => Ok(IpcResponse::Ack),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid response tag",
            )),
        }
    }
}

/// Receives an open file descriptor over a UNIX domain socket via SCM_RIGHTS.
pub fn recv_fd(sock: RawFd) -> std::io::Result<RawFd> {
    let mut dummy = [0u8; 1];
    let mut iov = libc::iovec {
        iov_base: dummy.as_mut_ptr().cast(),
        iov_len: 1,
    };
    let mut cmsg_buf = [0u8; 64];
    // SAFETY: `msghdr` is a POD C struct; zero-initialization is valid before setting fields.
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &raw mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr().cast();
    msg.msg_controllen = cmsg_buf.len() as _;

    // SAFETY: `msg` points to stack-allocated `iov` and `cmsg_buf` valid for the call.
    let res = unsafe { libc::recvmsg(sock, &raw mut msg, 0) };
    if res <= 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "failed to receive control message",
        ));
    }

    // SAFETY: `msg` was populated by `recvmsg`; `CMSG_FIRSTHDR` inspects `msg_controllen`.
    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&raw const msg) };
    if cmsg.is_null() {
        return Err(std::io::Error::other("missing cmsg in recvmsg"));
    }

    // SAFETY: `cmsg` is non-null and points within `cmsg_buf`.
    let (level, ctype, cmsg_len) =
        unsafe { ((*cmsg).cmsg_level, (*cmsg).cmsg_type, (*cmsg).cmsg_len) };
    if level != libc::SOL_SOCKET || ctype != libc::SCM_RIGHTS {
        return Err(std::io::Error::other("unexpected cmsg type"));
    }

    // SAFETY: `CMSG_LEN` is a pure size calculation macro.
    let min_len = unsafe { libc::CMSG_LEN(std::mem::size_of::<RawFd>() as u32) as usize };
    #[allow(clippy::unnecessary_cast)]
    let actual_len = cmsg_len as usize;
    if actual_len < min_len {
        return Err(std::io::Error::other("truncated cmsg for SCM_RIGHTS"));
    }

    let mut fd: RawFd = -1;
    // SAFETY: `cmsg` is validated to contain at least `sizeof(RawFd)` bytes in `CMSG_DATA`.
    unsafe {
        std::ptr::copy_nonoverlapping(
            libc::CMSG_DATA(cmsg),
            (&raw mut fd).cast::<u8>(),
            std::mem::size_of::<RawFd>(),
        );
    }

    if fd < 0 {
        return Err(std::io::Error::other("received invalid file descriptor"));
    }

    Ok(fd)
}

/// Sends an open file descriptor over a UNIX domain socket via SCM_RIGHTS.
pub fn send_fd(sock: RawFd, fd_to_send: RawFd) -> std::io::Result<()> {
    let dummy = [0u8; 1];
    let mut iov = libc::iovec {
        iov_base: dummy.as_ptr() as *mut libc::c_void,
        iov_len: 1,
    };
    let mut cmsg_buf = [0u8; 64];
    // SAFETY: `msghdr` is a POD C struct; zero-initialization is valid before setting fields.
    let mut msg: libc::msghdr = unsafe { std::mem::zeroed() };
    msg.msg_iov = &raw mut iov;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr().cast();
    msg.msg_controllen = cmsg_buf.len() as _;

    // SAFETY: `msg` has a 64-byte `cmsg_buf` which exceeds `CMSG_SPACE(sizeof(RawFd))`.
    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&raw const msg) };
    if cmsg.is_null() {
        return Err(std::io::Error::other("failed to allocate cmsg header"));
    }

    // SAFETY: `cmsg` points into `cmsg_buf` with enough space for one `RawFd`.
    unsafe {
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<RawFd>() as u32) as _;
        std::ptr::copy_nonoverlapping(
            (&raw const fd_to_send).cast::<u8>(),
            libc::CMSG_DATA(cmsg),
            std::mem::size_of::<RawFd>(),
        );
        msg.msg_controllen = (*cmsg).cmsg_len as _;
    }

    // SAFETY: `msg` and its control buffer are valid for the duration of `sendmsg`.
    let res = unsafe { libc::sendmsg(sock, &raw const msg, 0) };
    if res < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
