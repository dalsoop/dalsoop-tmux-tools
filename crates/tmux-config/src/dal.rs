use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime};

/// Test status
#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Pending,
    Running,
    Passed,
    Failed,
}

/// A single test item in the queue
#[derive(Debug, Clone)]
pub struct TestItem {
    pub trigger: String,
    pub label: String,
    pub command: String,
    pub status: TestStatus,
    pub output: String,
    pub duration_ms: u64,
    pub submitted_at: Option<Instant>,
}

/// Dal test runner state — delegates to dalcenter tester
pub struct DalState {
    pub queue: Vec<TestItem>,
    pub file_mtimes: HashMap<PathBuf, SystemTime>,
    pub project_root: PathBuf,
    pub last_scan: Option<Instant>,
    pub last_poll: Option<Instant>,
    pub auto_scan: bool,
    pub dal_target: String,
    pub workspace_name: String,
    pub tester_alive: bool,
}

/// Map a changed file path to test commands
fn file_to_tests(path: &Path, root: &Path) -> Vec<(String, String)> {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let rel_str = rel.to_string_lossy();

    let mut tests = Vec::new();

    if rel_str.ends_with(".rs") {
        let crate_name = if rel_str.starts_with("crates/") {
            rel.components().nth(1).map(|c| c.as_os_str().to_string_lossy().to_string())
        } else {
            None
        };

        if let Some(name) = crate_name {
            tests.push((
                format!("{name} unit"),
                format!("cargo test -p {name} --lib"),
            ));
        } else {
            tests.push((
                "workspace unit".into(),
                "cargo test --lib".into(),
            ));
        }
    }

    if rel_str.ends_with(".bats") {
        tests.push((
            "smoke (BATS)".into(),
            "bash tests/run.sh".into(),
        ));
    }

    if rel_str.contains("click.rs") || rel_str.contains("render.rs") {
        tests.push((
            "smoke (BATS)".into(),
            "bash tests/run.sh".into(),
        ));
    }

    if rel_str.contains("template.rs") || rel_str.contains("config") {
        tests.push((
            "config roundtrip".into(),
            "cargo test -p tmux-windowbar --lib -- config".into(),
        ));
    }

    tests
}

/// Detect workspace name from project root directory name
fn detect_workspace_name(root: &Path) -> String {
    root.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

impl DalState {
    pub fn new(project_root: PathBuf) -> Self {
        let workspace_name = detect_workspace_name(&project_root);
        let dal_target = format!("{workspace_name}--tester");
        Self {
            queue: Vec::new(),
            file_mtimes: HashMap::new(),
            project_root,
            last_scan: None,
            last_poll: None,
            auto_scan: true,
            dal_target,
            workspace_name,
            tester_alive: false,
        }
    }

    /// Check if tester dal is alive
    pub fn check_tester_status(&mut self) {
        let output = Command::new("dalcenter")
            .args(["status"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Look for our tester line and check if "running"
            for line in stdout.lines() {
                if line.contains(&format!("{}/tester", self.workspace_name)) {
                    self.tester_alive = line.contains("running");
                    return;
                }
            }
        }
        self.tester_alive = false;
    }

    /// Wake the tester dal
    pub fn wake_tester(&mut self) {
        let target = format!("{}/tester", self.workspace_name);
        let _ = Command::new("dalcenter")
            .args(["wake", &target])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        self.tester_alive = true;
    }

    /// Sleep the tester dal
    pub fn sleep_tester(&mut self) {
        let target = format!("{}/tester", self.workspace_name);
        let _ = Command::new("dalcenter")
            .args(["sleep", &target])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        self.tester_alive = false;
    }

    /// Scan project for changed files and queue tests
    pub fn scan(&mut self) {
        let root = self.project_root.clone();
        let files = collect_source_files(&root);
        let mut new_tests: Vec<TestItem> = Vec::new();

        for path in files {
            let mtime = match std::fs::metadata(&path).and_then(|m| m.modified()) {
                Ok(t) => t,
                Err(_) => continue,
            };

            let changed = match self.file_mtimes.get(&path) {
                Some(old) => *old != mtime,
                None => true,
            };

            if changed {
                self.file_mtimes.insert(path.clone(), mtime);

                for (label, cmd) in file_to_tests(&path, &root) {
                    let already = self.queue.iter().any(|t| {
                        t.command == cmd && (t.status == TestStatus::Pending || t.status == TestStatus::Running)
                    }) || new_tests.iter().any(|t| t.command == cmd);

                    if !already {
                        let rel = path.strip_prefix(&root).unwrap_or(&path);
                        new_tests.push(TestItem {
                            trigger: rel.to_string_lossy().to_string(),
                            label,
                            command: cmd,
                            status: TestStatus::Pending,
                            output: String::new(),
                            duration_ms: 0,
                            submitted_at: None,
                        });
                    }
                }
            }
        }

        self.queue.extend(new_tests);
        self.last_scan = Some(Instant::now());
    }

    /// Submit the next pending test to dalcenter (non-blocking)
    pub fn submit_next(&mut self) -> bool {
        let idx = match self.queue.iter().position(|t| t.status == TestStatus::Pending) {
            Some(i) => i,
            None => return false,
        };

        let task_msg = format!(
            "프로젝트 루트에서 다음 테스트를 실행하고 결과를 보고해줘: {}",
            self.queue[idx].command
        );

        let result = Command::new("dalcenter")
            .args([
                "send",
                "--msg-type", "task",
                &self.dal_target,
                &task_msg,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match result {
            Ok(out) if out.status.success() => {
                self.queue[idx].status = TestStatus::Running;
                self.queue[idx].submitted_at = Some(Instant::now());
                true
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.queue[idx].status = TestStatus::Failed;
                self.queue[idx].output = format!("dalcenter send failed: {stderr}");
                false
            }
            Err(e) => {
                self.queue[idx].status = TestStatus::Failed;
                self.queue[idx].output = format!("dalcenter not found: {e}");
                false
            }
        }
    }

    /// Submit all pending tests
    pub fn submit_all(&mut self) {
        loop {
            if !self.submit_next() {
                break;
            }
        }
    }

    /// Poll dalcenter for test results from tester dal
    pub fn poll_results(&mut self) {
        if !self.has_running() {
            return;
        }

        let output = Command::new("dalcenter")
            .args(["logs", &format!("{}/tester", self.workspace_name), "-n", "30"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let logs = match output {
            Ok(out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout).to_string()
            }
            _ => return,
        };

        // Parse logs for test results
        // Look for patterns like "cargo test ... ok" or "FAILED"
        for item in self.queue.iter_mut().filter(|t| t.status == TestStatus::Running) {
            // Check if the test command appears in logs with a result
            let cmd_keyword = item.command
                .split_whitespace()
                .last()
                .unwrap_or(&item.command);

            for line in logs.lines() {
                if !line.contains(cmd_keyword) {
                    continue;
                }
                if line.contains("test result: ok") || line.contains("PASSED") || line.contains("passed") {
                    item.status = TestStatus::Passed;
                    item.output = line.to_string();
                    if let Some(start) = item.submitted_at {
                        item.duration_ms = start.elapsed().as_millis() as u64;
                    }
                    break;
                }
                if line.contains("FAILED") || line.contains("failures") || line.contains("실패") {
                    item.status = TestStatus::Failed;
                    item.output = line.to_string();
                    if let Some(start) = item.submitted_at {
                        item.duration_ms = start.elapsed().as_millis() as u64;
                    }
                    break;
                }
            }

            // Timeout: 5 minutes
            if item.status == TestStatus::Running {
                if let Some(start) = item.submitted_at {
                    if start.elapsed().as_secs() > 300 {
                        item.status = TestStatus::Failed;
                        item.output = "timeout (5m)".into();
                        item.duration_ms = start.elapsed().as_millis() as u64;
                    }
                }
            }
        }

        self.last_poll = Some(Instant::now());
    }

    pub fn has_running(&self) -> bool {
        self.queue.iter().any(|t| t.status == TestStatus::Running)
    }

    pub fn count_pending(&self) -> usize {
        self.queue.iter().filter(|t| t.status == TestStatus::Pending).count()
    }

    pub fn count_passed(&self) -> usize {
        self.queue.iter().filter(|t| t.status == TestStatus::Passed).count()
    }

    pub fn count_failed(&self) -> usize {
        self.queue.iter().filter(|t| t.status == TestStatus::Failed).count()
    }

    pub fn count_running(&self) -> usize {
        self.queue.iter().filter(|t| t.status == TestStatus::Running).count()
    }

    pub fn clear_done(&mut self) {
        self.queue.retain(|t| t.status == TestStatus::Pending || t.status == TestStatus::Running);
    }

    pub fn clear_all(&mut self) {
        self.queue.clear();
    }

    /// Queue all tests (without submitting)
    pub fn queue_all(&mut self) {
        let tests = [
            ("tmux-fmt unit", "cargo test -p tmux-fmt --lib"),
            ("tmux-sessionbar unit", "cargo test -p tmux-sessionbar --lib"),
            ("tmux-windowbar unit", "cargo test -p tmux-windowbar --lib"),
            ("tmux-config unit", "cargo test -p tmux-config"),
            ("smoke (BATS)", "bash tests/run.sh"),
        ];

        for (label, cmd) in tests {
            let already = self.queue.iter().any(|t| {
                t.command == cmd && (t.status == TestStatus::Pending || t.status == TestStatus::Running)
            });
            if !already {
                self.queue.push(TestItem {
                    trigger: "(manual)".into(),
                    label: label.into(),
                    command: cmd.into(),
                    status: TestStatus::Pending,
                    output: String::new(),
                    duration_ms: 0,
                    submitted_at: None,
                });
            }
        }
    }
}

fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let dirs = ["crates", "tests"];
    for dir in dirs {
        let target = root.join(dir);
        if target.is_dir() {
            walk_dir(&target, &mut files);
        }
    }
    files
}

fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name != "target" && name != ".git" {
                walk_dir(&path, files);
            }
        } else {
            let ext = path.extension().unwrap_or_default().to_string_lossy();
            if ext == "rs" || ext == "bats" || ext == "toml" {
                files.push(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn file_to_tests_rs_crate() {
        let root = PathBuf::from("/project");
        let path = PathBuf::from("/project/crates/tmux-fmt/src/lib.rs");
        let tests = file_to_tests(&path, &root);
        assert!(!tests.is_empty());
        assert!(tests.iter().any(|(l, _)| l == "tmux-fmt unit"));
    }

    #[test]
    fn file_to_tests_bats() {
        let root = PathBuf::from("/project");
        let path = PathBuf::from("/project/tests/smoke.bats");
        let tests = file_to_tests(&path, &root);
        assert!(tests.iter().any(|(l, _)| l.contains("BATS")));
    }

    #[test]
    fn file_to_tests_click_adds_smoke() {
        let root = PathBuf::from("/project");
        let path = PathBuf::from("/project/crates/tmux-windowbar/src/commands/click.rs");
        let tests = file_to_tests(&path, &root);
        assert!(tests.len() >= 2);
    }

    #[test]
    fn detect_workspace() {
        let name = detect_workspace_name(Path::new("/root/workspace/dalsoop-tmux-tools"));
        assert_eq!(name, "dalsoop-tmux-tools");
    }

    #[test]
    fn dal_target_format() {
        let dal = DalState::new(PathBuf::from("/root/workspace/dalsoop-tmux-tools"));
        assert_eq!(dal.dal_target, "dalsoop-tmux-tools--tester");
        assert_eq!(dal.workspace_name, "dalsoop-tmux-tools");
    }

    #[test]
    fn clear_done_keeps_pending() {
        let mut dal = DalState::new(PathBuf::from("/tmp"));
        dal.queue.push(TestItem {
            trigger: "a.rs".into(),
            label: "pass".into(),
            command: "true".into(),
            status: TestStatus::Passed,
            output: String::new(),
            duration_ms: 0,
            submitted_at: None,
        });
        dal.queue.push(TestItem {
            trigger: "b.rs".into(),
            label: "pend".into(),
            command: "echo".into(),
            status: TestStatus::Pending,
            output: String::new(),
            duration_ms: 0,
            submitted_at: None,
        });
        dal.clear_done();
        assert_eq!(dal.queue.len(), 1);
        assert_eq!(dal.queue[0].label, "pend");
    }

    #[test]
    fn has_running_detection() {
        let mut dal = DalState::new(PathBuf::from("/tmp"));
        assert!(!dal.has_running());
        dal.queue.push(TestItem {
            trigger: "a.rs".into(),
            label: "test".into(),
            command: "cargo test".into(),
            status: TestStatus::Running,
            output: String::new(),
            duration_ms: 0,
            submitted_at: Some(Instant::now()),
        });
        assert!(dal.has_running());
    }
}
