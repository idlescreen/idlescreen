// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 crateria

//! Shared memory layout and control protocol for out-of-process screensaver execution.

pub mod protocol;
pub mod shm;

pub use protocol::{IpcCommand, IpcResponse};
pub use shm::{FfiTerminalCell, SHM_MAGIC, SharedMemory, SharedMemoryHeader, compute_shm_size};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_commands() {
        let cmds = vec![
            IpcCommand::Init {
                cols: 120,
                rows: 40,
            },
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

    #[test]
    fn test_compute_shm_size_zero_dimensions() {
        let size = compute_shm_size(0, 0);
        assert_eq!(size, std::mem::size_of::<SharedMemoryHeader>());
    }

    #[test]
    fn test_ffi_terminal_cell_conversion() {
        use trance_api::TerminalCell;
        let cell = TerminalCell {
            ch: '★',
            fg: (255, 128, 64),
            bg: (10, 20, 30),
            bold: true,
        };
        let ffi = FfiTerminalCell::from(cell);
        assert_eq!(ffi.ch, '★' as u32);
        assert_eq!(ffi.fg_r, 255);
        assert_eq!(ffi.fg_g, 128);
        assert_eq!(ffi.fg_b, 64);
        assert_eq!(ffi.bold, 1);

        let roundtrip = TerminalCell::from(ffi);
        assert_eq!(cell, roundtrip);
    }

    #[test]
    fn test_invalid_ipc_command_tag() {
        let bad_bytes = [99u8];
        assert!(IpcCommand::read_from(&bad_bytes[..]).is_err());
    }

    #[test]
    fn test_invalid_ipc_response_tag() {
        let bad_bytes = [255u8];
        assert!(IpcResponse::read_from(&bad_bytes[..]).is_err());
    }

    #[test]
    fn test_truncated_command_read() {
        let truncated = [0u8, 120]; // Tag 0 requires 8 bytes payload (cols:4, rows:4)
        assert!(IpcCommand::read_from(&truncated[..]).is_err());
    }
}
