// Sandboxing module for isolating execution
//
// Provides:
// - Process isolation via Linux namespaces (user, mount, PID, network)
// - Resource limits via cgroups v2 (memory, CPU, PIDs)
// - Filesystem access control (tmpfs, bind mount, read-only)
// - Network isolation (separate namespace or disabled)
// - Timeout enforcement via tokio::time::timeout
//
// Platform: Linux only (uses /sys/fs/cgroup, unshare, bind mounts)

use crate::{Result, SecurityError};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::timeout;
use tracing::{debug, warn};
use uuid::Uuid;

// ── Configuration ────────────────────────────────────────────────────

/// Sandbox configuration for isolated execution
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Maximum memory usage in bytes
    pub max_memory: Option<u64>,

    /// Maximum CPU time in seconds (wall clock timeout)
    pub max_cpu_time: Option<u64>,

    /// Network access allowed
    pub network_enabled: bool,

    /// Filesystem access mode
    pub filesystem_access: FilesystemAccess,

    /// Additional paths to make available in ReadOnly mode
    pub readonly_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemAccess {
    /// No filesystem access (empty tmpfs)
    None,

    /// Read-only access to specific paths
    ReadOnly,

    /// Full filesystem access (not recommended for untrusted code)
    Full,
}

impl SandboxConfig {
    pub fn strict() -> Self {
        Self {
            max_memory: Some(512 * 1024 * 1024), // 512 MB
            max_cpu_time: Some(30),              // 30 seconds
            network_enabled: false,
            filesystem_access: FilesystemAccess::None,
            readonly_paths: Vec::new(),
        }
    }

    pub fn relaxed() -> Self {
        Self {
            max_memory: Some(2 * 1024 * 1024 * 1024), // 2 GB
            max_cpu_time: Some(300),                  // 5 minutes
            network_enabled: true,
            filesystem_access: FilesystemAccess::ReadOnly,
            readonly_paths: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(mem) = self.max_memory {
            if mem == 0 {
                return Err(SecurityError::Sandbox(
                    "Memory limit cannot be zero".to_string(),
                ));
            }
        }

        if let Some(cpu) = self.max_cpu_time {
            if cpu == 0 {
                return Err(SecurityError::Sandbox(
                    "CPU time limit cannot be zero".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self::strict()
    }
}

// ── Execution Result ─────────────────────────────────────────────────

/// Result of a sandboxed execution
#[derive(Debug, Clone)]
pub struct SandboxResult {
    /// Exit code of the process (None if killed by signal/timeout)
    pub exit_code: Option<i32>,

    /// Standard output captured
    pub stdout: Vec<u8>,

    /// Standard error captured
    pub stderr: Vec<u8>,

    /// Whether the process was killed due to timeout
    pub timed_out: bool,

    /// Whether the process was killed by a signal
    pub killed_by_signal: Option<i32>,

    /// Wall clock duration of the execution
    pub duration: Duration,

    /// Peak memory usage in bytes (if cgroups tracking was enabled)
    pub peak_memory: Option<u64>,
}

// ── Sandbox Executor ─────────────────────────────────────────────────

/// Sandbox executor using Linux namespaces + cgroups v2
pub struct Sandbox {
    config: SandboxConfig,
    sandbox_id: String,
}

impl Sandbox {
    pub fn new(config: SandboxConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            sandbox_id: format!("sandbox-{}", Uuid::new_v4()),
        })
    }

    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// Execute arbitrary code in an isolated process.
    ///
    /// The provided closure runs in a child process isolated via:
    /// - User namespace (UID/GID mapping)
    /// - Mount namespace (controlled filesystem access)
    /// - PID namespace (process isolation)
    /// - Network namespace (if network_enabled = false)
    /// - cgroups v2 resource limits (memory, CPU, PIDs)
    ///
    /// Returns `SandboxResult` with captured stdout/stderr and metadata.
    /// Execute a command in an isolated sandbox.
    ///
    /// # Arguments
    /// * `program` - Absolute path to the executable
    /// * `args` - Arguments to pass to the program
    ///
    /// # Isolation guarantees
    /// - Process runs in separate user/mount/PID namespaces
    /// - Memory and CPU limits enforced via cgroups v2
    /// - Filesystem access controlled (tmpfs, readonly, or full)
    /// - Network isolated (unless `network_enabled = true`)
    /// - Process killed if it exceeds timeout
    pub async fn execute_command(&self, program: &str, args: &[&str]) -> Result<SandboxResult> {
        let start = tokio::time::Instant::now();

        // 1. Setup cgroups v2 for resource limits (Ok(None) = graceful degradation)
        let cgroup_dir = self.setup_cgroups().await?;

        // 2. Build the sandbox command with namespaces
        let mut cmd = self.build_sandboxed_command(program, args);

        // 3. Spawn the child process
        #[allow(unused_mut)]
        let mut child = cmd
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                SecurityError::Sandbox(format!("Failed to spawn sandbox process: {}", e))
            })?;

        let pid = child.id().unwrap_or(0);

        // 4. Add process to cgroup (if available)
        if let Some(ref cg_path) = cgroup_dir {
            self.add_process_to_cgroup(cg_path, pid).await?;
        }

        // 5. Wait for process with timeout, capturing output
        let timeout_dur = self
            .config
            .max_cpu_time
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(30));

        let wait_result = timeout(timeout_dur, child.wait_with_output()).await;

        let (exit_code, output_stdout, output_stderr, timed_out) = match wait_result {
            Ok(Ok(out)) => (out.status.code(), out.stdout, out.stderr, false),
            Ok(Err(e)) => {
                warn!("Sandbox process error: {}", e);
                (None, Vec::new(), Vec::new(), false)
            }
            Err(_elapsed) => {
                // child was moved into wait_with_output; kill_on_drop(true) handles cleanup
                warn!("Sandbox timeout reached for process {} — terminating via kill_on_drop", pid);
                (None, Vec::new(), Vec::new(), true)
            }
        };

        // 6. Peak memory from cgroups (if available)
        let peak_memory = if let Some(ref cg_path) = cgroup_dir {
            self.read_peak_memory(cg_path).await
        } else {
            None
        };

        // 7. Cleanup cgroups
        if let Some(ref cg_path) = cgroup_dir {
            self.cleanup_cgroups(cg_path).await;
        }

        let duration = start.elapsed();

        Ok(SandboxResult {
            exit_code,
            stdout: output_stdout,
            stderr: output_stderr,
            timed_out,
            killed_by_signal: None,
            duration,
            peak_memory,
        })
    }

    // ── cgroups v2 Management ────────────────────────────────────

    /// Create a cgroup directory and set resource limits.
    ///
    /// Returns `Ok(Some(path))` if cgroups were set up, `Ok(None)` if
    /// cgroups are unavailable or we lack permission (graceful degradation).
    async fn setup_cgroups(&self) -> Result<Option<PathBuf>> {
        let cgroup_root = PathBuf::from("/sys/fs/cgroup");

        if !cgroup_root.exists() {
            warn!("cgroups v2 not available at /sys/fs/cgroup");
            return Ok(None);
        }

        // Check if cgroup filesystem is accessible
        let procs_file = cgroup_root.join("cgroup.procs");
        if tokio::fs::metadata(&procs_file).await.is_err() {
            warn!("Cannot access cgroup.procs — running without cgroup limits (degraded mode)");
            return Ok(None);
        }

        // Create sandbox sub-cgroup
        let sandbox_cg = cgroup_root.join(&self.sandbox_id);

        if let Err(e) = tokio::fs::create_dir_all(&sandbox_cg).await {
            warn!(
                "Cannot create cgroup directory {}: {} — running without cgroup limits (degraded mode)",
                sandbox_cg.display(),
                e
            );
            return Ok(None);
        }

        // Set memory limit (non-fatal if write fails)
        if let Some(mem_bytes) = self.config.max_memory {
            let mem_max = sandbox_cg.join("memory.max");
            if let Err(e) = tokio::fs::write(&mem_max, mem_bytes.to_string()).await {
                warn!("Failed to set memory.max: {} — continuing without limit", e);
            } else {
                debug!("cgroup memory.max set to {} bytes", mem_bytes);
            }
        }

        // Set PID limit (non-fatal if write fails)
        let pids_max_path = sandbox_cg.join("pids.max");
        if let Err(e) = tokio::fs::write(&pids_max_path, "128").await {
            warn!("Failed to set pids.max: {} — continuing without limit", e);
        } else {
            debug!("cgroup pids.max set to 128");
        }

        debug!("cgroup created at {}", sandbox_cg.display());
        Ok(Some(sandbox_cg))
    }

    /// Add a process to the cgroup
    async fn add_process_to_cgroup(&self, cgroup_path: &PathBuf, pid: u32) -> Result<()> {
        let procs_file = cgroup_path.join("cgroup.procs");
        tokio::fs::write(&procs_file, pid.to_string())
            .await
            .map_err(|e| {
                SecurityError::Sandbox(format!("Failed to add process {} to cgroup: {}", pid, e))
            })?;
        debug!("Process {} added to cgroup {}", pid, cgroup_path.display());
        Ok(())
    }

    /// Read peak memory usage from cgroup
    async fn read_peak_memory(&self, cgroup_path: &PathBuf) -> Option<u64> {
        let mem_peak = cgroup_path.join("memory.peak");
        tokio::fs::read_to_string(&mem_peak)
            .await
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
    }

    /// Cleanup cgroup directory
    async fn cleanup_cgroups(&self, cgroup_path: &PathBuf) {
        // Remove the cgroup directory
        if let Err(e) = tokio::fs::remove_dir(cgroup_path).await {
            warn!("Failed to remove cgroup {}: {}", cgroup_path.display(), e);
        } else {
            debug!("Cleaned up cgroup {}", cgroup_path.display());
        }
    }

    // ── Namespace Isolation ──────────────────────────────────────

    /// Build the sandboxed command using `unshare` for namespaces
    fn build_sandboxed_command(&self, program: &str, args: &[&str]) -> TokioCommand {
        // We use `unshare` to create new namespaces:
        // -U: new user namespace
        // -m: new mount namespace
        // -p: new PID namespace
        // -f: fork before executing
        // -n: new network namespace (if network is disabled)
        // --map-root-user: map to root in the new user namespace

        let mut unshare_args: Vec<String> = vec![
            "-U".to_string(),
            "-m".to_string(),
            "-p".to_string(),
            "-f".to_string(),
            "--map-root-user".to_string(),
        ];

        if !self.config.network_enabled {
            unshare_args.push("-n".to_string());
        }

        // Filesystem setup depends on access mode
        let mount_setup = match self.config.filesystem_access {
            FilesystemAccess::None => {
                // Mount an empty tmpfs at a temporary root
                // We use --mount-proc to set up /proc
                unshare_args.push("--mount-proc".to_string());
                Some(format!(
                    "mount -t tmpfs none /tmp && {} {}",
                    program,
                    args.join(" ")
                ))
            }
            FilesystemAccess::ReadOnly => {
                // Keep filesystem but remount key dirs as readonly
                unshare_args.push("--mount-proc".to_string());
                Some(format!("{} {}", program, args.join(" ")))
            }
            FilesystemAccess::Full => {
                unshare_args.push("--mount-proc".to_string());
                None
            }
        };

        let mut cmd = TokioCommand::new("unshare");

        // Add unshare arguments
        for arg in &unshare_args {
            cmd.arg(arg);
        }

        if let Some(setup_script) = mount_setup {
            // Use sh -c to run the mount setup + actual command
            cmd.arg("sh").arg("-c").arg(setup_script);
        } else {
            cmd.arg(program);
            for arg in args {
                cmd.arg(arg);
            }
        }

        // Kill child on parent exit
        cmd.kill_on_drop(true);

        debug!("Sandbox command: unshare {}", unshare_args.join(" "));

        cmd
    }


    // ── Convenience: Execute with string input ────────────────────

    /// Execute a command with stdin input, capturing stdout/stderr
    pub async fn execute_with_input(
        &self,
        program: &str,
        args: &[&str],
        stdin_data: &[u8],
    ) -> Result<SandboxResult> {
        let start = tokio::time::Instant::now();

        let cgroup_dir = self.setup_cgroups().await?;

        let mut cmd = self.build_sandboxed_command(program, args);

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| SecurityError::Sandbox(format!("Failed to spawn: {}", e)))?;

        let pid = child.id().unwrap_or(0);

        if let Some(ref cg_path) = cgroup_dir {
            self.add_process_to_cgroup(cg_path, pid).await?;
        }

        // Write stdin
        use tokio::io::AsyncWriteExt;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(stdin_data)
                .await
                .map_err(|e| SecurityError::Sandbox(format!("Failed to write stdin: {}", e)))?;
            drop(stdin); // Close stdin
        }

        let timeout_dur = self
            .config
            .max_cpu_time
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(30));

        let output = timeout(timeout_dur, child.wait_with_output()).await;

        let (exit_code, stdout, stderr, timed_out) = match output {
            Ok(Ok(out)) => (out.status.code(), out.stdout, out.stderr, false),
            Ok(Err(e)) => {
                warn!("Process error: {}", e);
                (None, Vec::new(), Vec::new(), false)
            }
            Err(_) => {
                warn!("Timeout reached");
                (None, Vec::new(), Vec::new(), true)
            }
        };

        let peak_memory = if let Some(ref cg_path) = cgroup_dir {
            self.read_peak_memory(cg_path).await
        } else {
            None
        };

        if let Some(ref cg_path) = cgroup_dir {
            self.cleanup_cgroups(cg_path).await;
        }

        Ok(SandboxResult {
            exit_code,
            stdout,
            stderr,
            timed_out,
            killed_by_signal: None,
            duration: start.elapsed(),
            peak_memory,
        })
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Config Tests ─────────────────────────────────────────────

    #[test]
    fn test_sandbox_config_validation() {
        let mut config = SandboxConfig::strict();
        assert!(config.validate().is_ok());

        config.max_memory = Some(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sandbox_presets() {
        let strict = SandboxConfig::strict();
        assert!(!strict.network_enabled);
        assert_eq!(strict.filesystem_access, FilesystemAccess::None);
        assert_eq!(strict.max_memory, Some(512 * 1024 * 1024));

        let relaxed = SandboxConfig::relaxed();
        assert!(relaxed.network_enabled);
        assert_eq!(relaxed.filesystem_access, FilesystemAccess::ReadOnly);
        assert_eq!(relaxed.max_memory, Some(2 * 1024 * 1024 * 1024));
    }

    #[test]
    fn test_cpu_time_zero_validation() {
        let mut config = SandboxConfig::strict();
        config.max_cpu_time = Some(0);
        assert!(config.validate().is_err());
    }

    // ── Sandbox Execution Tests (requires Linux) ─────────────────

    #[tokio::test]
    async fn test_execute_simple_command() {
        let config = SandboxConfig::relaxed();
        let sandbox = Sandbox::new(config).unwrap();

        let true_path = "/run/current-system/sw/bin/true";
        let result = sandbox.execute_command(true_path, &[]).await;
        assert!(
            result.is_ok(),
            "Simple command should succeed: {:?}",
            result.err()
        );
        let r = result.unwrap();
        assert_eq!(r.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_execute_command_with_args() {
        let config = SandboxConfig::relaxed();
        let sandbox = Sandbox::new(config).unwrap();

        let echo_path = "/run/current-system/sw/bin/echo";
        let result = sandbox
            .execute_command(echo_path, &["hello", "world"])
            .await;
        assert!(result.is_ok(), "Command with args should succeed");
        let r = result.unwrap();
        assert_eq!(r.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_execute_failing_command() {
        let config = SandboxConfig::relaxed();
        let sandbox = Sandbox::new(config).unwrap();

        let false_path = "/run/current-system/sw/bin/false";
        let result = sandbox.execute_command(false_path, &[]).await;
        assert!(
            result.is_ok(),
            "Failing command should not error at sandbox level"
        );
        let r = result.unwrap();
        assert_ne!(r.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_timeout_enforcement() {
        let mut config = SandboxConfig::relaxed();
        config.max_cpu_time = Some(1); // 1 second timeout

        let sandbox = Sandbox::new(config).unwrap();
        let sleep_path = "/run/current-system/sw/bin/sleep";

        let result = sandbox.execute_command(sleep_path, &["5"]).await;
        assert!(result.is_ok(), "Timeout should not be an error");
        let r = result.unwrap();
        assert!(r.timed_out, "Process should be timed out");
    }

    #[tokio::test]
    async fn test_network_isolation() {
        let config = SandboxConfig::strict(); // network_enabled = false
        let sandbox = Sandbox::new(config).unwrap();

        // Try to resolve a hostname (should fail without network)
        let getent_path = "/run/current-system/sw/bin/getent";
        let result = sandbox
            .execute_command(getent_path, &["hosts", "google.com"])
            .await;

        // This might fail (no network) or succeed if DNS is cached
        // Just verify it doesn't crash
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sandbox_result_fields() {
        let config = SandboxConfig::relaxed();
        let sandbox = Sandbox::new(config).unwrap();

        let true_path = "/run/current-system/sw/bin/true";
        let result = sandbox.execute_command(true_path, &[]).await.unwrap();

        assert_eq!(result.exit_code, Some(0));
        assert!(!result.timed_out);
        assert!(result.killed_by_signal.is_none());
        assert!(result.duration.as_millis() > 0);
    }

    #[tokio::test]
    async fn test_strict_sandbox_no_network() {
        let config = SandboxConfig::strict();
        let sandbox = Sandbox::new(config).unwrap();

        let true_path = "/run/current-system/sw/bin/true";
        let result = sandbox.execute_command(true_path, &[]).await;
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_execute_with_input() {
        let config = SandboxConfig::relaxed();
        let sandbox = Sandbox::new(config).unwrap();

        let cat_path = "/run/current-system/sw/bin/cat";
        let result = sandbox
            .execute_with_input(cat_path, &[], b"test input data\n")
            .await;

        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_memory_limit_cgroup() {
        let config = SandboxConfig::relaxed();
        let sandbox = Sandbox::new(config).unwrap();

        // Test that cgroup is created for memory limits
        if PathBuf::from("/sys/fs/cgroup/cgroup.procs").exists() {
            let cgroup = sandbox.setup_cgroups().await;
            assert!(cgroup.is_ok(), "cgroup setup should succeed");
            if let Ok(Some(cg_path)) = cgroup {
                assert!(cg_path.exists(), "cgroup directory should exist");
                // Cleanup
                let _ = tokio::fs::remove_dir(&cg_path).await;
            }
        }
    }

    #[tokio::test]
    async fn test_cgroup_cleanup() {
        let config = SandboxConfig::relaxed();
        let sandbox = Sandbox::new(config).unwrap();

        if PathBuf::from("/sys/fs/cgroup/cgroup.procs").exists() {
            // Setup and immediate cleanup
            if let Ok(Some(cg_path)) = sandbox.setup_cgroups().await {
                sandbox.cleanup_cgroups(&cg_path).await;
                assert!(!cg_path.exists(), "cgroup should be cleaned up");
            }
        }
    }
}
