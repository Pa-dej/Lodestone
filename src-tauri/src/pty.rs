#![cfg(unix)]

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use tokio::io::unix::AsyncFd;

/// Обёртка над файловым дескриптором мастера PTY.
/// Реализует AsyncRead/AsyncWrite через AsyncFd — zero-copy.
pub struct PtyMaster {
    inner: AsyncFd<OwnedFd>,
}

impl PtyMaster {
    /// Открывает новую пару PTY, возвращает (master, slave_fd).
    /// slave_fd передаётся дочернему процессу как stdin/stdout/stderr.
    pub fn open() -> std::io::Result<(Self, OwnedFd)> {
        unsafe {
            // posix_openpt: открыть мастер
            let master_fd = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC);
            if master_fd < 0 {
                return Err(std::io::Error::last_os_error());
            }

            // grantpt + unlockpt: разрешить доступ к slave
            if libc::grantpt(master_fd) < 0 || libc::unlockpt(master_fd) < 0 {
                libc::close(master_fd);
                return Err(std::io::Error::last_os_error());
            }

            // ptsname_r: имя slave-устройства (/dev/pts/N)
            let mut name_buf = [0i8; 64];
            if libc::ptsname_r(master_fd, name_buf.as_mut_ptr(), name_buf.len()) != 0 {
                libc::close(master_fd);
                return Err(std::io::Error::last_os_error());
            }

            // Открываем slave
            let slave_fd = libc::open(
                name_buf.as_ptr(),
                libc::O_RDWR | libc::O_NOCTTY | libc::O_CLOEXEC,
            );
            if slave_fd < 0 {
                libc::close(master_fd);
                return Err(std::io::Error::last_os_error());
            }

            // Неблокирующий режим на мастере для AsyncFd
            let flags = libc::fcntl(master_fd, libc::F_GETFL);
            libc::fcntl(master_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);

            let master = PtyMaster {
                inner: AsyncFd::new(OwnedFd::from_raw_fd(master_fd))?,
            };
            let slave = OwnedFd::from_raw_fd(slave_fd);

            Ok((master, slave))
        }
    }

    /// Пишем команду в мастер — она уйдёт в stdin процесса.
    pub async fn write_all(&self, buf: &[u8]) -> std::io::Result<()> {
        let mut written = 0;
        while written < buf.len() {
            let mut guard = self.inner.writable().await?;
            match guard.try_io(|fd| {
                let n = unsafe {
                    libc::write(
                        fd.get_ref().as_raw_fd(),
                        buf[written..].as_ptr() as *const libc::c_void,
                        buf.len() - written,
                    )
                };
                if n < 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            }) {
                Ok(Ok(n)) => written += n,
                Ok(Err(e)) => return Err(e),
                Err(_would_block) => continue,
            }
        }
        Ok(())
    }

    /// Читаем вывод процесса в предоставленный буфер — zero-copy.
    pub async fn read(&self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            let mut guard = self.inner.readable().await?;
            match guard.try_io(|fd| {
                let n = unsafe {
                    libc::read(
                        fd.get_ref().as_raw_fd(),
                        buf.as_mut_ptr() as *mut libc::c_void,
                        buf.len(),
                    )
                };
                if n < 0 {
                    Err(std::io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            }) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }
}
