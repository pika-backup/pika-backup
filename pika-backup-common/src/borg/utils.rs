use super::log_json::*;

pub fn check_line(line: &str) -> LogEntry {
    if let Ok(mut msg @ LogMessage { .. }) = serde_json::from_str(line) {
        if matches!(msg.msgid, MsgId::Undefined) {
            let msgid_helper_parsed: std::result::Result<MsgIdHelper, _> =
                serde_json::from_str(line);
            if let Ok(msgid_helper) = msgid_helper_parsed {
                msg.msgid = MsgId::Other(msgid_helper.msgid);
            }
        }
        tracing::info!("LogMessage {:?}", msg);

        LogEntry::ParsedErr(msg)
    } else {
        tracing::error!("Parse error {}", line);
        LogEntry::UnparsableErr(line.to_string())
    }
}

fn is_sha256_faster() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::arch::is_x86_feature_detected!("sha")
    }
    #[cfg(target_arch = "aarch64")]
    {
        std::arch::is_aarch64_feature_detected!("sha2")
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        false
    }
}

pub fn fasted_hash_algorithm() -> &'static str {
    if is_sha256_faster() { "" } else { "-blake2" }
}

pub fn mount_base_dir() -> std::path::PathBuf {
    crate::utils::host::user_runtime_dir()
        .join(env!("CARGO_PKG_NAME"))
        .join("mount")
}
