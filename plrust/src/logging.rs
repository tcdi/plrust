/*
Portions Copyright 2021-2025 Technology Concepts & Design, Inc.

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/
/*
Copyright 2021-2023 Technology Concepts & Design, Inc. <support@tcdi.com>

All rights reserved.

Use of this source code is governed by the PostgreSQL license that can be found in the LICENSE.md file.
*/

use std::io::Write;

pub(crate) struct PgrxGuestWriter<const IS_STDERR: bool>;

impl<const IS_STDERR: bool> Write for PgrxGuestWriter<IS_STDERR> {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
        let content = std::str::from_utf8(data).expect("Could not interpret stdout as UTF-8");
        let prefixed = if IS_STDERR {
            String::from("stderr: ") + content
        } else {
            String::from("stdout: ") + content
        };
        pgrx::log!("{}", &prefixed);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}

pub(crate) struct PgrxLogWriter<const TRIM: bool = true>;

impl<const TRIM: bool> Write for PgrxLogWriter<TRIM> {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
        let content = std::str::from_utf8(data).expect("Could not interpret stdout as UTF-8");
        let content = if TRIM { content.trim_start() } else { content };
        pgrx::log!("{}", content);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}

pub(crate) struct PgrxNoticeWriter<const TRIM: bool = true>;

impl<const TRIM: bool> Write for PgrxNoticeWriter<TRIM> {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
        let content = std::str::from_utf8(data).expect("Could not interpret stdout as UTF-8");
        let content = if TRIM { content.trim_start() } else { content };
        pgrx::notice!("{}", content);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}

pub(crate) struct PgrxWarningWriter<const TRIM: bool = true>;

impl<const TRIM: bool> Write for PgrxWarningWriter<TRIM> {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, std::io::Error> {
        let content = std::str::from_utf8(data).expect("Could not interpret stdout as UTF-8");
        let content = if TRIM { content.trim_start() } else { content };
        pgrx::warning!("{}", content);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        Ok(())
    }
}
