/// Trait to implement a more human format for various types
pub trait Readable {
    /// Return a more readable string for a type for user facing components
    fn readable(&self) -> String;
}

impl Readable for chrono::Duration {
    fn readable(&self) -> String {
        format_from_seconds(self.num_seconds() as u64)
    }
}

impl Readable for std::time::Duration {
    fn readable(&self) -> String {
        format_from_seconds(self.as_secs())
    }
}

impl Readable for chrono::DateTime<chrono::Utc> {
    fn readable(&self) -> String {
        format!(
            "{} ({})",
            self.format("%d.%m.%Y at %H:%M:%S"),
            self.timezone()
        )
    }
}

fn format_from_seconds(mut raw: u64) -> String {
    let mut result = String::new();
    let seconds = raw % 60;
    raw /= 60;
    let minutes = raw % 60;
    raw /= 60;
    let hours = raw % 24;
    raw /= 24;
    let days = raw;
    match days {
        0 => {}
        1 => result.push_str(&format!("{} day ", days)),
        _ => result.push_str(&format!("{} days ", days)),
    }
    match hours {
        0 => {}
        1 => result.push_str(&format!("{} hour ", hours)),
        _ => result.push_str(&format!("{} hours ", hours)),
    }
    match minutes {
        0 => {}
        1 => result.push_str(&format!("{} minute ", minutes)),
        _ => result.push_str(&format!("{} minutes ", minutes)),
    }
    match seconds {
        0 => {}
        1 => result.push_str(&format!("{} second ", seconds)),
        _ => result.push_str(&format!("{} seconds ", seconds)),
    }
    if result.len() == 0 {
        result = format!("0 Seconds");
    }
    result.trim_end().to_owned()
}
