//! Linux raw-syscall wrappers through the public API.
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[cfg(target_os = "linux")]
mod linux_only {
    use std::fs;
    use std::io::Write;

    use rust_kernel_game_engine_kit::platform::linux::io_uring::IoUring;
    use rust_kernel_game_engine_kit::platform::linux::syscalls::{
        read_file, Fd, MappedFile, O_CLOEXEC, O_RDONLY,
    };

    fn temp_path(tag: &str) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!(
            "{}/rust_kernel_test_{tag}_{}_{}.tmp",
            std::env::temp_dir().display(),
            std::process::id(),
            nanos
        )
    }

    #[test]
    fn read_file_reads_embedded_nul_bytes() {
        let path = temp_path("read");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(b"abc\x00def")
            .unwrap();
        assert_eq!(read_file(&path).unwrap(), b"abc\x00def");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn mmap_presents_identical_bytes() {
        let path = temp_path("mmap");
        let bytes: Vec<u8> = (0..=255u8).collect();
        std::fs::File::create(&path)
            .unwrap()
            .write_all(&bytes)
            .unwrap();

        let fd = Fd::open(&path, O_RDONLY | O_CLOEXEC, 0).unwrap();
        let map = MappedFile::map_readonly(&fd, bytes.len() as u64, &path).unwrap();
        assert_eq!(map.as_slice(), &bytes[..]);
        drop(map);
        drop(fd); // munmap ran before close; both happen in this scope.
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_file_surfaces_errno_not_panic() {
        let err = read_file("/definitely/not/a/real/path").unwrap_err();
        assert!(err.to_string().contains("open"));
    }

    #[test]
    fn io_uring_reads_owned_buffer_to_completion() {
        let path = temp_path("uring");
        let payload = b"io_uring-end-to-end\0payload";
        fs::File::create(&path).unwrap().write_all(payload).unwrap();
        let fd = Fd::open(&path, O_RDONLY | O_CLOEXEC, 0).unwrap();
        let mut ring = IoUring::new(8).expect("Linux CI must expose io_uring");

        let request = ring
            .submit_read(fd.raw_fd(), vec![0; payload.len()], 0, 0xfeed)
            .unwrap();
        ring.enter(1, 1).unwrap();
        let mut completed = None;
        for _ in 0..8 {
            if ring.try_complete().is_some() {
                completed = Some(ring.take_completed(request.user_data()).unwrap());
                break;
            }
        }
        assert_eq!(completed.as_deref(), Some(payload.as_slice()));
        drop(fd);
        let _ = fs::remove_file(path);
    }
}
