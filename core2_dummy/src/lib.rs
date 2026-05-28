#![no_std]
pub mod io {
    pub trait Read {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()>;
    }
    pub trait Write {
        fn write(&mut self, buf: &[u8]) -> Result<usize, ()>;
        fn flush(&mut self) -> Result<(), ()>;
    }
    pub type Error = ();
}
