use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::FileExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::names::numbered;
use crate::{EngineError, JobSpec, OnExists};

const MAX_NUMBER: u32 = 10_000;

fn open_new(path: &Path) -> io::Result<File> {
    OpenOptions::new().read(true).write(true).create_new(true).open(path)
}

fn open_truncate(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
}

pub(crate) fn open_existing(path: &Path) -> io::Result<File> {
    OpenOptions::new().read(true).write(true).open(path)
}

pub(crate) fn disk_err(e: io::Error) -> EngineError {
    EngineError::Disk(e.to_string())
}

pub(crate) struct Claimed {
    pub working_path: PathBuf,
    pub file_name: String,
    pub file: File,
}

/// Create the folder and claim a working file for `name`, applying the exists policy.
/// The name is checked against both the final path and the temp path.
pub(crate) fn claim(spec: &JobSpec, name: &str) -> Result<Claimed, EngineError> {
    std::fs::create_dir_all(&spec.dest_dir).map_err(disk_err)?;
    let suffix = spec.temp_suffix.as_deref().unwrap_or("");
    for i in 0..=MAX_NUMBER {
        let candidate = if i == 0 { name.to_string() } else { numbered(name, i) };
        let final_path = spec.dest_dir.join(&candidate);
        let working_path = spec.dest_dir.join(format!("{candidate}{suffix}"));
        let taken = final_path.exists() || (!suffix.is_empty() && working_path.exists());
        let opened = match spec.on_exists {
            OnExists::Overwrite => open_truncate(&working_path).map_err(disk_err)?,
            OnExists::Fail => {
                if taken {
                    return Err(EngineError::Exists);
                }
                match open_new(&working_path) {
                    Ok(f) => f,
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Err(EngineError::Exists),
                    Err(e) => return Err(disk_err(e)),
                }
            }
            OnExists::Number => {
                if taken {
                    continue;
                }
                match open_new(&working_path) {
                    Ok(f) => f,
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(e) => return Err(disk_err(e)),
                }
            }
        };
        return Ok(Claimed { working_path, file_name: candidate, file: opened });
    }
    Err(EngineError::Exists)
}

/// Reopen a known working path from scratch (restart or a lost partial file).
pub(crate) fn reclaim(working_path: &Path, file_name: &str) -> Result<Claimed, EngineError> {
    if let Some(dir) = working_path.parent() {
        std::fs::create_dir_all(dir).map_err(disk_err)?;
    }
    let file = open_truncate(working_path).map_err(disk_err)?;
    Ok(Claimed { working_path: working_path.to_path_buf(), file_name: file_name.to_string(), file })
}

pub(crate) fn mark_sparse(file: &File) {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::IO::DeviceIoControl;
    use windows::Win32::System::Ioctl::FSCTL_SET_SPARSE;

    let mut returned = 0u32;
    // Sparse is only an optimisation, so a failure here is ignored.
    unsafe {
        let _ = DeviceIoControl(
            HANDLE(file.as_raw_handle()),
            FSCTL_SET_SPARSE,
            None,
            0,
            None,
            0,
            Some(&mut returned),
            None,
        );
    }
}

pub(crate) fn write_all_at(file: &File, mut data: &[u8], mut pos: u64) -> io::Result<()> {
    while !data.is_empty() {
        let n = file.seek_write(data, pos)?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "the disk accepted no data"));
        }
        data = &data[n..];
        pos += n as u64;
    }
    Ok(())
}

/// Move the working file to its final name. A name that appeared meanwhile is numbered
/// unless the job overwrites.
pub(crate) fn finalize(
    working_path: &Path,
    file_name: &str,
    on_exists: OnExists,
) -> Result<(PathBuf, String), EngineError> {
    let dir = working_path.parent().unwrap_or(Path::new("."));
    let target = dir.join(file_name);
    if target == working_path {
        return Ok((target, file_name.to_string()));
    }
    if on_exists == OnExists::Overwrite || !target.exists() {
        std::fs::rename(working_path, &target).map_err(disk_err)?;
        return Ok((target, file_name.to_string()));
    }
    for i in 1..=MAX_NUMBER {
        let candidate = numbered(file_name, i);
        let path = dir.join(&candidate);
        if path.exists() {
            continue;
        }
        std::fs::rename(working_path, &path).map_err(disk_err)?;
        return Ok((path, candidate));
    }
    Err(EngineError::Exists)
}

/// Parse an HTTP date (`Sun, 06 Nov 1994 08:49:37 GMT`).
pub(crate) fn parse_http_date(text: &str) -> Option<SystemTime> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }
    let day: i64 = parts[1].parse().ok()?;
    let month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        .iter()
        .position(|m| m.eq_ignore_ascii_case(parts[2]))? as i64
        + 1;
    let year: i64 = parts[3].parse().ok()?;
    let mut hms = parts[4].split(':').map(|x| x.parse::<i64>());
    let (h, m, s) = (hms.next()?.ok()?, hms.next()?.ok()?, hms.next()?.ok()?);
    if !(1..=31).contains(&day) || h > 23 || m > 59 || s > 60 {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + h * 3600 + m * 60 + s;
    let secs = u64::try_from(secs).ok()?;
    SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_dates() {
        let t = parse_http_date("Sun, 06 Nov 1994 08:49:37 GMT").unwrap();
        assert_eq!(t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(), 784_111_777);
        let t = parse_http_date("Thu, 01 Jan 1970 00:00:00 GMT").unwrap();
        assert_eq!(t, SystemTime::UNIX_EPOCH);
        assert!(parse_http_date("garbage").is_none());
    }
}
