use chrono::{DateTime, Local, TimeZone};

pub fn size(bytes: u64, decimal: bool) -> String {
    let (step, units): (f64, [&str; 5]) =
        if decimal { (1000.0, ["B", "KB", "MB", "GB", "TB"]) } else { (1024.0, ["B", "KiB", "MiB", "GiB", "TiB"]) };
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= step && unit < units.len() - 1 {
        value /= step;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else if value >= 100.0 {
        format!("{value:.0} {}", units[unit])
    } else {
        format!("{value:.1} {}", units[unit])
    }
}

pub fn speed(bytes_per_sec: u64, decimal: bool) -> String {
    format!("{}/s", size(bytes_per_sec, decimal))
}

pub fn eta(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m {:02}s", secs / 60, secs % 60),
        3600..=86_399 => format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60),
        _ => format!("{}d {}h", secs / 86_400, (secs % 86_400) / 3600),
    }
}

fn clock(at: &DateTime<Local>) -> String {
    at.format("%-I:%M %P").to_string()
}

/// "Today 4:05 pm", "Yesterday", "12 Nov", "12 Nov 2025".
pub fn when(ms: i64) -> String {
    let Some(at) = Local.timestamp_millis_opt(ms).single() else { return String::new() };
    let now = Local::now();
    let days = now.date_naive().signed_duration_since(at.date_naive()).num_days();
    match days {
        0 => format!("Today {}", clock(&at)),
        1 => "Yesterday".into(),
        _ if at.format("%Y").to_string() == now.format("%Y").to_string() => at.format("%-d %b").to_string(),
        _ => at.format("%-d %b %Y").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_read_well() {
        assert_eq!(size(512, true), "512 B");
        assert_eq!(size(1_500, true), "1.5 KB");
        assert_eq!(size(250_000_000, true), "250 MB");
        assert_eq!(size(1_048_576, false), "1.0 MiB");
    }

    #[test]
    fn times_read_well() {
        assert_eq!(eta(42), "42s");
        assert_eq!(eta(125), "2m 05s");
        assert_eq!(eta(7_260), "2h 01m");
    }
}
