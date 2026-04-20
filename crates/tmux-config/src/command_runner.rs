//! 원격/로컬 명령 실행 추상화.
//!
//! `ssh_run(user, host, cmd)` 에 숨어 있던 "localhost 면 `sh -c`, 아니면 `ssh`"
//! 분기를 trait 으로 명시화. 구현체가 늘어나도 사용부는 바뀌지 않는다.
//!
//! - [`LocalRunner`]: `sh -c <cmd>` 로 현재 프로세스에서 실행
//! - [`SshRunner`]: `ssh user@host cmd`. host key 미등록 시 1회 자동 등록 후 재시도
//! - [`runner_for`]: host 문자열이 로컬이면 Local, 아니면 Ssh 를 선택

use crate::proxmox::{ensure_host_key, is_localhost};
use std::process::Command;

const SSH_OPTS: [&str; 4] = ["-o", "ConnectTimeout=5", "-o", "BatchMode=yes"];

pub trait CommandRunner {
    /// 명령을 실행하고 성공 시 stdout 을, 실패하면 `None` 을 돌려준다.
    fn run(&self, cmd: &str) -> Option<String>;
}

pub struct LocalRunner;

impl CommandRunner for LocalRunner {
    fn run(&self, cmd: &str) -> Option<String> {
        let output = Command::new("sh").arg("-c").arg(cmd).output().ok()?;
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }
        None
    }
}

pub struct SshRunner {
    pub user: String,
    pub host: String,
}

impl CommandRunner for SshRunner {
    fn run(&self, cmd: &str) -> Option<String> {
        let target = format!("{}@{}", self.user, self.host);
        let try_once = || {
            let output = Command::new("ssh")
                .args(SSH_OPTS)
                .arg(&target)
                .arg(cmd)
                .output()
                .ok()?;
            Some(output)
        };

        let output = try_once()?;
        if output.status.success() {
            return Some(String::from_utf8_lossy(&output.stdout).to_string());
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Host key verification failed") && ensure_host_key(&self.host) {
            let retry = try_once()?;
            if retry.status.success() {
                return Some(String::from_utf8_lossy(&retry.stdout).to_string());
            }
        }
        None
    }
}

/// `host` 가 현재 머신을 가리키면 [`LocalRunner`], 아니면 [`SshRunner`].
pub fn runner_for(user: &str, host: &str) -> Box<dyn CommandRunner> {
    if is_localhost(host) {
        Box::new(LocalRunner)
    } else {
        Box::new(SshRunner {
            user: user.to_string(),
            host: host.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 파서·캐시 테스트에서 주입할 용도. 고정 응답만 돌려주는 스텁.
    pub struct StubRunner {
        pub response: Option<String>,
    }
    impl CommandRunner for StubRunner {
        fn run(&self, _cmd: &str) -> Option<String> {
            self.response.clone()
        }
    }

    #[test]
    fn local_runner_echoes() {
        let out = LocalRunner.run("echo ok").unwrap();
        assert_eq!(out.trim(), "ok");
    }

    #[test]
    fn local_runner_nonzero_returns_none() {
        assert!(LocalRunner.run("false").is_none());
    }

    #[test]
    fn runner_for_localhost_is_local() {
        // host = 127.0.0.1 → LocalRunner 선택 — echo 가 ssh 없이 돈다
        let r = runner_for("root", "127.0.0.1");
        assert_eq!(r.run("echo hello").unwrap().trim(), "hello");
    }

    #[test]
    fn stub_runner_returns_fixed() {
        let s = StubRunner { response: Some("fixed".into()) };
        assert_eq!(s.run("anything").as_deref(), Some("fixed"));
    }
}
