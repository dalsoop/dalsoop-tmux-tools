//! SSH 관련 헬퍼.
//!
//! - [`is_localhost`] : 호스트 문자열이 현재 머신을 가리키는지 (프로세스 당 1회 캐시)
//! - [`ssh_run`] / [`remote_or_local`] : command_runner 위에 얹은 얇은 래퍼
//! - [`host_key_exists`] / [`register_host_key`] / [`ensure_host_key`] : known_hosts 관리

use std::collections::HashSet;
use std::process::Command;
use std::sync::OnceLock;

fn local_aliases() -> &'static HashSet<String> {
    static ALIASES: OnceLock<HashSet<String>> = OnceLock::new();
    ALIASES.get_or_init(|| {
        let mut set: HashSet<String> = ["127.0.0.1", "localhost", "::1"]
            .iter().map(|s| s.to_string()).collect();
        if let Ok(out) = Command::new("hostname").output() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                set.insert(s.trim().to_string());
            }
        }
        if let Ok(out) = Command::new("hostname").arg("-I").output() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                for ip in s.split_whitespace() { set.insert(ip.to_string()); }
            }
        }
        set
    })
}

/// 루프백·localhost·현재 hostname·머신의 모든 로컬 IP 중 하나면 true.
pub fn is_localhost(host: &str) -> bool {
    local_aliases().contains(host)
}

/// 하위 호환 얇은 래퍼. 새 코드에서는 [`crate::command_runner::runner_for`] 직접 사용 권장.
pub(crate) fn ssh_run(user: &str, host: &str, cmd: &str) -> Option<String> {
    crate::command_runner::runner_for(user, host).run(cmd)
}

/// 상호작용형 명령(`ssh -t ...`) 을 로컬·원격 상황에 맞게 조립.
pub(crate) fn remote_or_local(user: &str, host: &str, remote_cmd: &str) -> String {
    if is_localhost(host) {
        remote_cmd.to_string()
    } else {
        format!("ssh -t {user}@{host} {remote_cmd}")
    }
}

pub fn host_key_exists(host: &str) -> bool {
    Command::new("ssh-keygen")
        .args(["-F", host])
        .output()
        .map(|o| o.status.success() && !o.stdout.is_empty())
        .unwrap_or(false)
}

pub fn register_host_key(host: &str) -> bool {
    let output = Command::new("ssh-keyscan").args(["-H", host]).output();
    match output {
        Ok(o) if !o.stdout.is_empty() => {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into()); // LINT_ALLOW: last-resort fallback when $HOME is unset
            let known_hosts = std::path::PathBuf::from(home).join(".ssh/known_hosts");
            std::fs::OpenOptions::new()
                .create(true).append(true).open(&known_hosts)
                .and_then(|mut f| { use std::io::Write; f.write_all(&o.stdout) })
                .is_ok()
        }
        _ => false,
    }
}

/// known_hosts 에 key 있으면 즉시 true, 없으면 interactive prompt 로 등록 시도.
pub fn ensure_host_key(host: &str) -> bool {
    if host_key_exists(host) { return true; }
    eprint!("Host key not found for '{host}'. Register it now? [y/N] ");
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() { return false; }
    if !input.trim().eq_ignore_ascii_case("y") { return false; }
    if register_host_key(host) {
        eprintln!("✓ Host key registered for '{host}'");
        true
    } else {
        eprintln!("✗ Failed to register host key for '{host}'");
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_localhost_static_aliases() {
        assert!(is_localhost("127.0.0.1"));
        assert!(is_localhost("localhost"));
        assert!(is_localhost("::1"));
        assert!(!is_localhost("192.0.2.99")); // TEST-NET-1 (RFC 5737)
    }
}
