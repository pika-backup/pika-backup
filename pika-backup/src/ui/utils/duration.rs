use crate::ui::prelude::*;

pub fn from_std(duration: std::time::Duration) -> chrono::Duration {
    chrono::Duration::from_std(duration)
        .ok()
        .unwrap_or(chrono::TimeDelta::MAX)
}

pub fn plain(d: &chrono::Duration) -> String {
    if d.num_minutes() < 1 {
        ngettextf_("One second", "{} seconds", d.num_seconds() as u32)
    } else if d.num_hours() < 1 {
        ngettextf_("One minute", "{} minutes", d.num_minutes() as u32)
    } else if d.num_days() < 2 {
        ngettextf_("One hour", "{} hours", d.num_hours() as u32)
    } else {
        ngettextf_("One day", "{} days", d.num_days() as u32)
    }
}

pub fn plain_lowercase(d: &chrono::Duration) -> String {
    if d.num_minutes() < 1 {
        ngettextf_("one second", "{} seconds", d.num_seconds() as u32)
    } else if d.num_hours() < 1 {
        ngettextf_("one minute", "{} minutes", d.num_minutes() as u32)
    } else if d.num_days() < 2 {
        ngettextf_("one hour", "{} hours", d.num_hours() as u32)
    } else {
        ngettextf_("one day", "{} days", d.num_days() as u32)
    }
}

pub fn left(d: &chrono::Duration) -> String {
    if d.num_hours() < 1 {
        ngettextf_(
            "One minute left",
            "{} minutes left",
            (d.num_minutes() + 1) as u32,
        )
    } else if d.num_days() < 2 {
        ngettextf_("One hour left", "{} hours left", (d.num_hours() + 1) as u32)
    } else {
        ngettextf_("One day left", "{} days left", (d.num_days() + 1) as u32)
    }
}

pub fn ago(d: &chrono::Duration) -> String {
    if d.num_minutes() < 1 {
        gettext("Now")
    } else if d.num_hours() < 1 {
        ngettextf_("One minute ago", "{} minutes ago", d.num_minutes() as u32)
    } else if d.num_days() < 1 {
        ngettextf_("One hour ago", "{} hours ago", d.num_hours() as u32)
    } else if d.num_weeks() < 1 {
        ngettextf_("One day ago", "{} days ago", d.num_days() as u32)
    } else if d.num_days() < 30 {
        ngettextf_("One week ago", "{} weeks ago", d.num_weeks() as u32)
    } else if d.num_weeks() < 52 {
        ngettextf_("One month ago", "{} months ago", (d.num_days() / 30) as u32)
    } else {
        ngettextf_("One year ago", "{} years ago", (d.num_weeks() / 52) as u32)
    }
}
