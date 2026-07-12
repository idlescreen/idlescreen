// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 crateria

//! Shared memory layout and control protocol for out-of-process screensaver execution.

use std::ffi::CString;
use std::ptr;
use trance_api::TerminalCell;

/// FFI-safe representation of `TerminalCell` for shared memory communication.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FfiTerminalCell {
    pub ch: u32,
    pub fg_r: u8,
    pub fg_g: u8,
    pub fg_b: u8,
    pub bg_r: u8,
    pub bg_g: u8,
    pub bg_b: u8,
    pub bold: u8,
}

impl From<TerminalCell> for FfiTerminalCell {
    fn from(cell: TerminalCell) -> Self {
        Self {
            ch: cell.ch as u32,
            fg_r: cell.fg.0,
            fg_g: cell.fg.1,
            fg_b: cell.fg.2,
            bg_r: cell.bg.0,
            bg_g: cell.bg.1,
            bg_b: cell.bg.2,
            bold: if cell.bold { 1 } else { 0 },
        }
    }
}

impl From<FfiTerminalCell> for TerminalCell {
    fn from(ffi: FfiTerminalCell) -> Self {
        Self {
            ch: std::char::from_u32(ffi.ch).unwrap_or(' '),
            fg: (ffi.fg_r, ffi.fg_g, ffi.fg_b),
            bg: (ffi.bg_r, ffi.bg_g, ffi.bg_b),
            bold: ffi.bold != 0,
        }
    }
}

#[repr(C)]
pub struct SharedMemoryHeader {
    pub magic: u32,
    pub cols: u32,
    pub rows: u32,
    pub frame_counter: u64,
}

pub const SHM_MAGIC: u32 = 0x54524e43; // 'TRNC'

/// Returns the size in bytes required for a shared memory segment of the given grid size.
pub fn compute_shm_size(cols: usize, rows: usize) -> usize {
    std::mem::size_of::<SharedMemoryHeader>() + cols * rows * std::mem::size_of::<FfiTerminalCell>()
}

/// A wrapper around a POSIX shared memory segment.
pub struct SharedMemory {
    name: String,
    fd: libc::c_int,
    ptr: *mut libc::c_void,
    size: usize,
    is_owner: bool,
}

impl SharedMemory {
    /// Creates a new shared memory segment.
    pub fn create(name: &str, size: usize) -> Result<Self, String> {
        let c_name = CString::new(name).map_err(|e| e.to_string())?;

        unsafe {
            libc::shm_unlink(c_name.as_ptr());
        }

        let fd = unsafe {
            libc::shm_open(
                c_name.as_ptr(),
                libc::O_CREAT | libc::O_RDWR | libc::O_EXCL,
                0o600,
            )
        };
        if fd < 0 {
            return Err(format!(
                "shm_open (create) failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        if unsafe { libc::ftruncate(fd, size as libc::off_t) } < 0 {
            let err = std::io::Error::last_os_error();
            unsafe {
                libc::close(fd);
                libc::shm_unlink(c_name.as_ptr());
            }
            return Err(format!("ftruncate failed: {err}"));
        }

        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            let err = std::io::Error::last_os_error();
            unsafe {
                libc::close(fd);
                libc::shm_unlink(c_name.as_ptr());
            }
            return Err(format!("mmap failed: {err}"));
        }

        Ok(Self {
            name: name.to_string(),
            fd,
            ptr,
            size,
            is_owner: true,
        })
    }

    /// Opens an existing shared memory segment.
    pub fn open(name: &str, size: usize) -> Result<Self, String> {
        let c_name = CString::new(name).map_err(|e| e.to_string())?;

        let fd = unsafe { libc::shm_open(c_name.as_ptr(), libc::O_RDWR, 0) };
        if fd < 0 {
            return Err(format!(
                "shm_open (open) failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            let err = std::io::Error::last_os_error();
            unsafe {
                libc::close(fd);
            }
            return Err(format!("mmap failed: {err}"));
        }

        Ok(Self {
            name: name.to_string(),
            fd,
            ptr,
            size,
            is_owner: false,
        })
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub unsafe fn header_mut(&self) -> &mut SharedMemoryHeader {
        unsafe { &mut *(self.ptr as *mut SharedMemoryHeader) }
    }

    pub unsafe fn cells_mut(&self) -> &mut [FfiTerminalCell] {
        let header = unsafe { self.header_mut() };
        let count = (header.cols * header.rows) as usize;
        let cells_ptr = unsafe {
            (self.ptr as *mut u8).add(std::mem::size_of::<SharedMemoryHeader>())
                as *mut FfiTerminalCell
        };
        unsafe { std::slice::from_raw_parts_mut(cells_ptr, count) }
    }
}

impl Drop for SharedMemory {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() && self.ptr != libc::MAP_FAILED {
                libc::munmap(self.ptr, self.size);
            }
            if self.fd >= 0 {
                libc::close(self.fd);
            }
            if self.is_owner {
                if let Ok(c_name) = CString::new(self.name.clone()) {
                    libc::shm_unlink(c_name.as_ptr());
                }
            }
        }
    }
}

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
    FrameReady { scanlines: bool },
    Ack,
}

impl IpcCommand {
    pub fn write_to<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
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
        Ok(())
    }

    pub fn read_from<R: std::io::Read>(mut reader: R) -> std::io::Result<Self> {
        let mut tag = [0u8; 1];
        reader.read_exact(&mut tag)?;
        match tag[0] {
            0 => {
                let mut cols_bytes = [0u8; 4];
                let mut rows_bytes = [0u8; 4];
                reader.read_exact(&mut cols_bytes)?;
                reader.read_exact(&mut rows_bytes)?;
                Ok(IpcCommand::Init {
                    cols: u32::from_le_bytes(cols_bytes),
                    rows: u32::from_le_bytes(rows_bytes),
                })
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
    pub fn write_to<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
        match self {
            IpcResponse::Ready => {
                writer.write_all(&[0])?;
            }
            IpcResponse::FrameReady { scanlines } => {
                writer.write_all(&[1, if *scanlines { 1 } else { 0 }])?;
            }
            IpcResponse::Ack => {
                writer.write_all(&[2])?;
            }
        }
        Ok(())
    }

    pub fn read_from<R: std::io::Read>(mut reader: R) -> std::io::Result<Self> {
        let mut tag = [0u8; 1];
        reader.read_exact(&mut tag)?;
        match tag[0] {
            0 => Ok(IpcResponse::Ready),
            1 => {
                let mut scan_byte = [0u8; 1];
                reader.read_exact(&mut scan_byte)?;
                Ok(IpcResponse::FrameReady {
                    scanlines: scan_byte[0] != 0,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_commands() {
        let cmds = vec![
            IpcCommand::Init { cols: 120, rows: 40 },
            IpcCommand::TickAndDraw { dt_micros: 16666 },
            IpcCommand::SetSimulationRate { hz: 60.0 },
            IpcCommand::Stop,
        ];

        for cmd in cmds {
            let mut buf = Vec::new();
            cmd.write_to(&mut buf).unwrap();
            let decoded = IpcCommand::read_from(&buf[..]).unwrap();
            assert_eq!(cmd, decoded);
        }
    }

    #[test]
    fn test_ipc_responses() {
        let resps = vec![
            IpcResponse::Ready,
            IpcResponse::FrameReady { scanlines: true },
            IpcResponse::FrameReady { scanlines: false },
            IpcResponse::Ack,
        ];

        for resp in resps {
            let mut buf = Vec::new();
            resp.write_to(&mut buf).unwrap();
            let decoded = IpcResponse::read_from(&buf[..]).unwrap();
            assert_eq!(resp, decoded);
        }
    }

    #[test]
    fn test_shm_size() {
        let size = compute_shm_size(80, 24);
        let header_sz = std::mem::size_of::<SharedMemoryHeader>();
        let cell_sz = std::mem::size_of::<FfiTerminalCell>();
        assert_eq!(size, header_sz + 80 * 24 * cell_sz);
    }
}

