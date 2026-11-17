//! Retain only CAP_NET_RAW. UID 0 is retained for root-owned UCI/ubus access;
//! child processes cannot reacquire root capabilities or gain privilege on exec.
#[cfg(target_os = "linux")]
pub fn restrict() -> std::io::Result<()> {
    #[repr(C)]
    struct Header {
        version: u32,
        pid: i32,
    }
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Data {
        effective: u32,
        permitted: u32,
        inheritable: u32,
    }
    fn checked(code: i32) -> std::io::Result<()> {
        if code < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    // SECBIT_NOROOT | SECBIT_NOROOT_LOCKED | SECBIT_NO_SETUID_FIXUP |
    // SECBIT_NO_SETUID_FIXUP_LOCKED: do not regain capabilities as UID 0 on exec.
    unsafe {
        if libc::geteuid() == 0 {
            checked(libc::prctl(libc::PR_SET_SECUREBITS, 15, 0, 0, 0))?;
        }
        checked(libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0))?;
        let header = Header {
            version: 0x20080522,
            pid: 0,
        };
        let data = [
            Data {
                effective: 1 << 13,
                permitted: 1 << 13,
                inheritable: 0,
            },
            Data {
                effective: 0,
                permitted: 0,
                inheritable: 0,
            },
        ];
        checked(libc::syscall(libc::SYS_capset, &header as *const _, data.as_ptr()) as i32)?;
    }
    Ok(())
}
