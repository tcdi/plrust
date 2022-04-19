use wasi_common::pipe::WritePipe;
use std::io::Write;

pub(crate) struct StdoutLogger;

impl Write for StdoutLogger {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
        pgx::log!("{}", std::str::from_utf8(data).expect("Could not interpret stdout as UTF-8"));
        Ok(data.len())
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> { 
        Ok(())
    }
}

pub(crate) struct StderrLogger;

impl Write for StderrLogger {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
        pgx::warning!("{}", std::str::from_utf8(data).expect("Could not interpret stderr as UTF-8"));
        Ok(data.len())
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> { 
        Ok(())
    }
}