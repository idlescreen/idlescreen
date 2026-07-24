// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

//! Shared memory layout and control protocol for out-of-process screensaver execution.

pub mod ffi_cell;
pub mod path_safety;
pub mod protocol;
pub mod shm;

pub use ffi_cell::{
    FfiTerminalCell, MAX_GRID_CELLS, MAX_GRID_DIM, SHM_MAGIC, SharedMemoryHeader, compute_shm_size,
    validate_grid_dims,
};
pub use path_safety::{is_plausible_socket_path, is_valid_shm_name};
pub use protocol::{IpcCommand, IpcResponse};
pub use shm::SharedMemory;

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
            cmd.write_to(&mut buf).expect("encode command");
            let decoded = IpcCommand::read_from(&buf[..]).expect("decode command");
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
            resp.write_to(&mut buf).expect("encode response");
            let decoded = IpcResponse::read_from(&buf[..]).expect("decode response");
            assert_eq!(resp, decoded);
        }
    }

    #[test]
    fn test_shm_size() {
        let size = compute_shm_size(80, 24).expect("size");
        let header_sz = std::mem::size_of::<SharedMemoryHeader>();
        let cell_sz = std::mem::size_of::<FfiTerminalCell>();
        assert_eq!(size, header_sz + 80 * 24 * cell_sz);
    }

    #[test]
    fn test_compute_shm_size_zero_dimensions() {
        let size = compute_shm_size(0, 0).expect("zero grid is just header");
        assert_eq!(size, std::mem::size_of::<SharedMemoryHeader>());
    }

    #[test]
    fn test_compute_shm_size_overflow_is_none() {
        assert!(compute_shm_size(usize::MAX, 2).is_none());
        assert!(compute_shm_size(usize::MAX / 2, usize::MAX / 2).is_none());
    }

    #[test]
    fn test_validate_grid_dims() {
        assert!(validate_grid_dims(80, 24).is_ok());
        assert!(validate_grid_dims(0, 24).is_err());
        assert!(validate_grid_dims(80, 0).is_err());
        assert!(validate_grid_dims(0, 0).is_err());
        assert!(validate_grid_dims(MAX_GRID_DIM + 1, 1).is_err());
        assert!(validate_grid_dims(1, MAX_GRID_DIM + 1).is_err());
        assert!(validate_grid_dims(MAX_GRID_CELLS, 2).is_err());
    }

    #[test]
    fn test_validate_grid_dims_boundaries() {
        // Exact axis max is allowed (`>` must not become `>=`).
        assert!(validate_grid_dims(MAX_GRID_DIM, 1).is_ok());
        assert!(validate_grid_dims(1, MAX_GRID_DIM).is_ok());
        // Exact cell-count max is allowed (`<=` must not become `<` or always-true).
        assert!(validate_grid_dims(512, 512).is_ok());
        assert_eq!(512 * 512, MAX_GRID_CELLS);
        // One cell over the cap with both axes still under MAX_GRID_DIM.
        assert!(validate_grid_dims(512, 513).is_err());
        assert!(validate_grid_dims(513, 512).is_err());
        // Both axes max → product far over MAX_GRID_CELLS.
        assert!(validate_grid_dims(MAX_GRID_DIM, MAX_GRID_DIM).is_err());
    }

    #[test]
    fn test_ffi_terminal_cell_conversion() {
        use idle_api::TerminalCell;
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

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn arb_command() -> impl Strategy<Value = IpcCommand> {
        prop_oneof![
            (any::<u32>(), any::<u32>()).prop_map(|(cols, rows)| IpcCommand::Init { cols, rows }),
            any::<u64>().prop_map(|dt_micros| IpcCommand::TickAndDraw { dt_micros }),
            any::<f32>().prop_filter_map("finite hz", |hz| {
                hz.is_finite()
                    .then_some(IpcCommand::SetSimulationRate { hz })
            }),
            Just(IpcCommand::Stop),
        ]
    }

    fn arb_response() -> impl Strategy<Value = IpcResponse> {
        prop_oneof![
            Just(IpcResponse::Ready),
            any::<bool>().prop_map(|scanlines| IpcResponse::FrameReady { scanlines }),
            Just(IpcResponse::Ack),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// Every command encodes and decodes to an equal value.
        #[test]
        fn command_roundtrip(cmd in arb_command()) {
            let mut buf = Vec::new();
            cmd.write_to(&mut buf).expect("write");
            let decoded = IpcCommand::read_from(&buf[..]).expect("read");
            prop_assert_eq!(cmd, decoded);
        }

        /// Every response encodes and decodes to an equal value.
        #[test]
        fn response_roundtrip(resp in arb_response()) {
            let mut buf = Vec::new();
            resp.write_to(&mut buf).expect("write");
            let decoded = IpcResponse::read_from(&buf[..]).expect("read");
            prop_assert_eq!(resp, decoded);
        }

        /// SHM size is at least the header and grows linearly with cells.
        #[test]
        fn shm_size_monotonic(cols in 0usize..512, rows in 0usize..512) {
            let size = compute_shm_size(cols, rows).expect("no overflow in range");
            let header = std::mem::size_of::<SharedMemoryHeader>();
            let cell = std::mem::size_of::<FfiTerminalCell>();
            prop_assert!(size >= header);
            prop_assert_eq!(size, header + cols * rows * cell);
            if cols > 0 && rows > 0 {
                let smaller = compute_shm_size(cols - 1, rows).expect("smaller");
                prop_assert!(smaller < size || cols == 1);
            }
        }

        /// Adversarial dims either validate cleanly or are rejected; size never panics.
        #[test]
        fn adversarial_dims_never_panic(cols in any::<u32>(), rows in any::<u32>()) {
            let c = cols as usize;
            let r = rows as usize;
            let _ = validate_grid_dims(c, r);
            let _ = compute_shm_size(c, r);
        }

        /// Unknown command tags are rejected.
        #[test]
        fn invalid_command_tags_fail(tag in 4u8..=255) {
            prop_assert!(IpcCommand::read_from(&[tag][..]).is_err());
        }

        /// Unknown response tags are rejected.
        #[test]
        fn invalid_response_tags_fail(tag in 3u8..=255) {
            prop_assert!(IpcResponse::read_from(&[tag][..]).is_err());
        }
    }
}
