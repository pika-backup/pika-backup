#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => (
        glib::g_log!(module_path!(), glib::LogLevel::Error, $($arg)+)
    )
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => (
        glib::g_log!(module_path!(), glib::LogLevel::Warning, $($arg)+)
    )
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => (
        glib::g_log!(module_path!(), glib::LogLevel::Info, $($arg)+)
    )
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => (
        glib::g_log!(module_path!(), glib::LogLevel::Debug, $($arg)+)
    )
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => (
        glib::g_log!(module_path!(), glib::LogLevel::Debug, $($arg)+)
    )
}
