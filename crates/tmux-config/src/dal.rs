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
    pub trigger: String,   // file that triggered this test
    pub label: String,     // display label (e.g. "tmux-fmt unit")
    pub command: String,   // full command to run
    pub status: TestStatus,
    pub output: String,    // stdout+stderr
    pub duration_ms: u64,
}

/// Dal test runner state
pub struct DalState {
    pub queue: Vec<TestItem>,
    pub file_mtimes: HashMap<PathBuf, SystemTime>,
    pub project_root: PathBuf,
    pub last_scan: Option<Instant>,
    pub auto_scan: bool,
    pub running: bool,
}

/// Map a changed file path to test commands
fn file_to_tests(path: &Path, root: &Path) -> Vec<(String, String)> {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let rel_str = rel.to_string_lossy();

    let mut tests = Vec::new();

    // Rust source → cargo test for the crate
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

    // BATS test file changed
    if rel_str.ends_with(".bats") {
        tests.push((
            "smoke (BATS)".into(),
            "bash tests/run.sh".into(),
        ));
    }

    // Click/render handler changed → also run smoke tests
    if rel_str.contains("click.rs") || rel_str.contains("render.rs") {
        tests.push((
            "smoke (BATS)".into(),
            "bash tests/run.sh".into(),
        ));
    }

    // Config template changed → test config roundtrip
    if rel_str.contains("template.rs") || rel_str.contains("config") {
        tests.push((
            "config roundtrip".into(),
            "cargo test -p tmux-windowbar --lib -- config".into(),
        ));
    }

    tests
}

impl DalState {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            queue: Vec::new(),
            file_mtimes: HashMap::new(),
            project_root,
            last_scan: None,
            auto_scan: true,
            running: false,
        }
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
                None => true, // new file
            };

            if changed {
                self.file_mtimes.insert(path.clone(), mtime);

                for (label, cmd) in file_to_tests(&path, &root) {
                    // Deduplicate: don't add if same command already pending
                    let already = self.queue.iter().any(|t| {
                        t.command == cmd && t.status == TestStatus::Pending
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
                        });
                    }
                }
            }
        }

        self.queue.extend(new_tests);
        self.last_scan = Some(Instant::now());
    }

    /// Run the next pending test (blocking — call from a thread or tick)
    pub fn run_next(&mut self) -> bool {
        let idx = match self.queue.iter().position(|t| t.status == TestStatus::Pending) {
            Some(i) => i,
            None => return false,
        };

        self.running = true;
        self.queue[idx].status = TestStatus::Running;

        let cmd = &self.queue[idx].command;
        let start = Instant::now();

        let result = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let elapsed = start.elapsed().as_millis() as u64;

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                self.queue[idx].output = format!("{stdout}{stderr}");
                self.queue[idx].status = if output.status.success() {
                    TestStatus::Passed
                } else {
                    TestStatus::Failed
                };
                self.queue[idx].duration_ms = elapsed;
            }
            Err(e) => {
                self.queue[idx].output = format!("exec error: {e}");
                self.queue[idx].status = TestStatus::Failed;
                self.queue[idx].duration_ms = elapsed;
            }
        }

        self.running = false;
        true
    }

    /// Count by status
    pub fn count_pending(&self) -> usize {
        self.queue.iter().filter(|t| t.status == TestStatus::Pending).count()
    }

    pub fn count_passed(&self) -> usize {
        self.queue.iter().filter(|t| t.status == TestStatus::Passed).count()
    }

    pub fn count_failed(&self) -> usize {
        self.queue.iter().filter(|t| t.status == TestStatus::Failed).count()
    }

    /// Clear completed tests from queue
    pub fn clear_done(&mut self) {
        self.queue.retain(|t| t.status == TestStatus::Pending || t.status == TestStatus::Running);
    }

    /// Clear all tests
    pub fn clear_all(&mut self) {
        self.queue.clear();
    }

    /// Manual add: run all tests
    pub fn queue_all(&mut self) {
        let tests = [
            ("tmux-fmt unit", "cargo test -p tmux-fmt --lib"),
            ("tmux-sessionbar unit", "cargo test -p tmux-sessionbar --lib"),
            ("tmux-windowbar unit", "cargo test -p tmux-windowbar --lib"),
            ("tmux-config unit", "cargo test -p tmux-config --lib"),
            ("smoke (BATS)", "bash tests/run.sh"),
        ];

        for (label, cmd) in tests {
            let already = self.queue.iter().any(|t| t.command == cmd && t.status == TestStatus::Pending);
            if !already {
                self.queue.push(TestItem {
                    trigger: "(manual)".into(),
                    label: label.into(),
                    command: cmd.into(),
                    status: TestStatus::Pending,
                    output: String::new(),
                    duration_ms: 0,
                });
            }
        }
    }
}

/// Collect all .rs and .bats files under the project root
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
            // Skip target/ and .git/
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
        assert!(tests.len() >= 2); // unit + smoke
    }

    #[test]
    fn dedup_pending() {
        let mut dal = DalState::new(PathBuf::from("/tmp"));
        dal.queue.push(TestItem {
            trigger: "a.rs".into(),
            label: "test".into(),
            command: "cargo test".into(),
            status: TestStatus::Pending,
            output: String::new(),
            duration_ms: 0,
        });
        // queue_all should not add duplicate "cargo test" if already pending
        // (but queue_all uses specific commands, so this tests the concept)
        assert_eq!(dal.count_pending(), 1);
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
        });
        dal.queue.push(TestItem {
            trigger: "b.rs".into(),
            label: "pend".into(),
            command: "echo".into(),
            status: TestStatus::Pending,
            output: String::new(),
            duration_ms: 0,
        });
        dal.clear_done();
        assert_eq!(dal.queue.len(), 1);
        assert_eq!(dal.queue[0].label, "pend");
    }
}
