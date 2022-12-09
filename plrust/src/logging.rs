use std::io::Write;

pub(crate) struct PgxGuestWriter<const IS_STDERR: bool>;

impl<const IS_STDERR: bool> Write for PgxGuestWriter<IS_STDERR> {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
        let content = std::str::from_utf8(data).expect("Could not interpret stdout as UTF-8");
        let prefixed = if IS_STDERR {
            String::from("stderr: ") + content
        } else {
            String::from("stdout: ") + content
        };
        pgx::log!("{}", &prefixed);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}

pub(crate) struct PgxLogWriter<const TRIM: bool = true>;

impl<const TRIM: bool> Write for PgxLogWriter<TRIM> {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
        let content = std::str::from_utf8(data).expect("Could not interpret stdout as UTF-8");
        let content = if TRIM { content.trim_start() } else { content };
        pgx::log!("{}", content);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}

pub(crate) struct PgxNoticeWriter<const TRIM: bool = true>;

impl<const TRIM: bool> Write for PgxNoticeWriter<TRIM> {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
        let content = std::str::from_utf8(data).expect("Could not interpret stdout as UTF-8");
        let content = if TRIM { content.trim_start() } else { content };
        pgx::notice!("{}", content);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}

pub(crate) struct PgxWarningWriter<const TRIM: bool = true>;

impl<const TRIM: bool> Write for PgxWarningWriter<TRIM> {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
        let content = std::str::from_utf8(data).expect("Could not interpret stdout as UTF-8");
        let content = if TRIM { content.trim_start() } else { content };
        pgx::warning!("{}", content);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}
