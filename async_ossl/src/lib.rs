#[cfg(unix)]
pub trait AsRawDesc: std::os::unix::io::AsRawFd {}
#[cfg(windows)]
pub trait AsRawDesc: std::os::windows::io::AsRawSocket {}

#[cfg(not(windows))]
mod real {
    use openssl::ssl::SslStream;
    use std::net::TcpStream;

    #[derive(Debug)]
    pub struct AsyncSslStream {
        s: SslStream<TcpStream>,
    }

    unsafe impl async_io::IoSafe for AsyncSslStream {}

    impl AsyncSslStream {
        pub fn new(s: SslStream<TcpStream>) -> Self {
            Self { s }
        }
    }

    impl std::os::fd::AsFd for AsyncSslStream {
        fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
            self.s.get_ref().as_fd()
        }
    }

    impl std::os::unix::io::AsRawFd for AsyncSslStream {
        fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
            self.s.get_ref().as_raw_fd()
        }
    }

    impl super::AsRawDesc for AsyncSslStream {}

    impl std::io::Read for AsyncSslStream {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
            self.s.read(buf)
        }
    }

    impl std::io::Write for AsyncSslStream {
        fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
            self.s.write(buf)
        }
        fn flush(&mut self) -> Result<(), std::io::Error> {
            self.s.flush()
        }
    }
}

#[cfg(not(windows))]
pub use real::AsyncSslStream;

// Stub for Windows ? TLS mux domain connections are not supported in this build.
// Callers that construct AsyncSslStream are also cfg-gated and will never reach here.
#[cfg(windows)]
mod stub {
    #[derive(Debug)]
    pub struct AsyncSslStream;

    unsafe impl async_io::IoSafe for AsyncSslStream {}

    impl AsyncSslStream {
        #[allow(dead_code)]
        pub fn new<T>(_: T) -> Self {
            unimplemented!("TLS mux connections are not supported on Windows in this build")
        }
    }

    impl std::os::windows::io::AsRawSocket for AsyncSslStream {
        fn as_raw_socket(&self) -> std::os::windows::io::RawSocket {
            unimplemented!()
        }
    }

    impl std::os::windows::io::AsSocket for AsyncSslStream {
        fn as_socket(&self) -> std::os::windows::io::BorrowedSocket<'_> {
            unimplemented!()
        }
    }

    impl super::AsRawDesc for AsyncSslStream {}

    impl std::io::Read for AsyncSslStream {
        fn read(&mut self, _buf: &mut [u8]) -> Result<usize, std::io::Error> {
            unimplemented!()
        }
    }

    impl std::io::Write for AsyncSslStream {
        fn write(&mut self, _buf: &[u8]) -> Result<usize, std::io::Error> {
            unimplemented!()
        }
        fn flush(&mut self) -> Result<(), std::io::Error> {
            unimplemented!()
        }
    }
}

#[cfg(windows)]
pub use stub::AsyncSslStream;
