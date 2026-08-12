use crate::ir::Analysis;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub const SIMULATION_SCHEMA_VERSION: &str = "netlang.simulation.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationRequest {
    pub schema_version: String,
    pub netlist: String,
    pub analyses: Vec<Analysis>,
    pub timeout_ms: u64,
    pub artifact_policy: ArtifactPolicy,
}

impl SimulationRequest {
    pub fn new(netlist: impl Into<String>, analyses: Vec<Analysis>) -> Self {
        Self {
            schema_version: SIMULATION_SCHEMA_VERSION.to_string(),
            netlist: netlist.into(),
            analyses,
            timeout_ms: 30_000,
            artifact_policy: ArtifactPolicy::CleanupAlways,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactPolicy {
    CleanupAlways,
    RetainOnFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationStatus {
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulatorProcessStatus {
    pub exit_code: Option<i32>,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulatorInfo {
    pub executable: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationResult {
    pub schema_version: String,
    pub status: SimulationStatus,
    pub analyses: Vec<Analysis>,
    pub simulator: SimulatorInfo,
    pub process: SimulatorProcessStatus,
    pub measurements: BTreeMap<String, f64>,
    pub datasets: Vec<AnalysisDataset>,
    pub diagnostics: Vec<SimulatorDiagnostic>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub raw_log: SimulatorLog,
    pub artifacts: Vec<SimulationArtifact>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisDataset {
    pub index: usize,
    pub analysis: Analysis,
    pub data: Dataset,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Dataset {
    OperatingPoint { values: BTreeMap<String, f64> },
    Transient(RealSeriesDataset),
    Ac(ComplexSeriesDataset),
    DcSweep(RealSeriesDataset),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealSeriesDataset {
    pub axis: SeriesAxis,
    pub signals: BTreeMap<String, Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplexSeriesDataset {
    pub frequency_hz: Vec<f64>,
    pub signals: BTreeMap<String, ComplexSeries>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComplexSeries {
    pub real: Vec<f64>,
    pub imaginary: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeriesAxis {
    pub name: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulatorDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulatorDiagnosticKind {
    Warning,
    Convergence,
    Fatal,
    ResultParse,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulatorDiagnostic {
    pub code: String,
    pub severity: SimulatorDiagnosticSeverity,
    pub kind: SimulatorDiagnosticKind,
    pub message: String,
}

impl SimulationResult {
    pub fn succeeded(&self) -> bool {
        self.status == SimulationStatus::Succeeded
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulatorLog {
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationArtifact {
    pub kind: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationRunErrorKind {
    Io,
    Launch,
    VersionCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationRunError {
    pub kind: SimulationRunErrorKind,
    pub message: String,
}

impl std::fmt::Display for SimulationRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SimulationRunError {}

#[derive(Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

pub trait SimulationRunner {
    fn run(
        &self,
        request: &SimulationRequest,
        cancellation: &CancellationToken,
    ) -> Result<SimulationResult, SimulationRunError>;
}

pub fn analysis_data_filename(index: usize, analysis: &Analysis) -> String {
    format!("netlang-analysis-{index:03}-{}.data", analysis.kind_name())
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use crate::simulation_parser::{
        classify_simulator_log, parse_measurements, parse_wrdata, result_parse_diagnostic,
    };
    use std::env;
    use std::fs;
    use std::io::Read;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    static NEXT_RUN_ID: AtomicU64 = AtomicU64::new(0);

    #[derive(Debug, Clone)]
    pub struct NgspiceRunner {
        executable: PathBuf,
    }

    impl NgspiceRunner {
        pub fn discover() -> Self {
            Self::new(discover_ngspice_path())
        }

        pub fn new(executable: impl Into<PathBuf>) -> Self {
            Self {
                executable: executable.into(),
            }
        }

        pub fn executable(&self) -> &Path {
            &self.executable
        }

        fn probe_version(&self) -> Result<String, SimulationRunError> {
            let cancellation = CancellationToken::new();
            let captured = execute_process(
                &self.executable,
                &["-v"],
                None,
                Duration::from_secs(5),
                &cancellation,
            )?;
            if !captured.success {
                return Err(run_error(
                    SimulationRunErrorKind::VersionCheck,
                    format!(
                        "Ngspice version check failed for '{}' with process status {}.",
                        self.executable.display(),
                        display_exit_code(captured.exit_code)
                    ),
                ));
            }
            let output = if captured.stdout.trim().is_empty() {
                captured.stderr.as_str()
            } else {
                captured.stdout.as_str()
            };
            let version = output.lines().find_map(|line| {
                line.split_whitespace()
                    .map(|token| token.trim_matches('*'))
                    .find(|token| token.to_ascii_lowercase().starts_with("ngspice"))
            });
            let Some(version) = version else {
                return Err(run_error(
                    SimulationRunErrorKind::VersionCheck,
                    format!(
                        "Ngspice version check at '{}' produced no recognizable version.",
                        self.executable.display()
                    ),
                ));
            };
            Ok(version.to_string())
        }
    }

    impl SimulationRunner for NgspiceRunner {
        fn run(
            &self,
            request: &SimulationRequest,
            cancellation: &CancellationToken,
        ) -> Result<SimulationResult, SimulationRunError> {
            if request.schema_version != SIMULATION_SCHEMA_VERSION {
                return Err(run_error(
                    SimulationRunErrorKind::Io,
                    format!(
                        "Unsupported simulation request schema '{}'; expected '{}'.",
                        request.schema_version, SIMULATION_SCHEMA_VERSION
                    ),
                ));
            }

            if cancellation.is_cancelled() {
                return Ok(cancelled_before_launch(request, &self.executable));
            }
            let version = self.probe_version()?;
            let mut run_directory = RunDirectory::create()?;
            let netlist_path = run_directory.path().join("circuit.spice");
            fs::write(&netlist_path, &request.netlist).map_err(|error| {
                run_error(
                    SimulationRunErrorKind::Io,
                    format!(
                        "Failed to write temporary SPICE netlist '{}': {error}",
                        netlist_path.display()
                    ),
                )
            })?;

            let timeout = Duration::from_millis(request.timeout_ms.max(1));
            let captured = execute_process(
                &self.executable,
                &["-b", "circuit.spice"],
                Some(run_directory.path()),
                timeout,
                cancellation,
            )?;

            let mut diagnostics = classify_simulator_log(&captured.stdout, &captured.stderr);
            let measurements = match parse_measurements(&captured.stdout) {
                Ok(measurements) => measurements,
                Err(parse_errors) => {
                    diagnostics.extend(parse_errors.into_iter().map(|error| {
                        result_parse_diagnostic(format!("Could not parse measurement: {error}"))
                    }));
                    BTreeMap::new()
                }
            };
            let mut datasets = Vec::new();
            for (index, analysis) in request.analyses.iter().enumerate() {
                let filename = analysis_data_filename(index, analysis);
                let path = run_directory.path().join(&filename);
                if path.is_file() {
                    match fs::read_to_string(&path) {
                        Ok(contents) => match parse_wrdata(analysis, &contents) {
                            Ok(data) => datasets.push(AnalysisDataset {
                                index,
                                analysis: analysis.clone(),
                                data,
                            }),
                            Err(error) => diagnostics.push(result_parse_diagnostic(format!(
                                "Could not parse analysis dataset '{filename}': {error}"
                            ))),
                        },
                        Err(error) => diagnostics.push(result_parse_diagnostic(format!(
                            "Could not read analysis dataset '{filename}': {error}"
                        ))),
                    }
                } else if request.netlist.contains(&format!("wrdata {filename} ")) {
                    diagnostics.push(result_parse_diagnostic(format!(
                        "Ngspice did not produce expected analysis dataset '{filename}'."
                    )));
                }
            }

            let has_diagnostic_errors = diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == SimulatorDiagnosticSeverity::Error);
            let status = if captured.cancelled {
                SimulationStatus::Cancelled
            } else if captured.timed_out {
                SimulationStatus::TimedOut
            } else if captured.success && !has_diagnostic_errors {
                SimulationStatus::Succeeded
            } else {
                SimulationStatus::Failed
            };

            let mut warnings = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == SimulatorDiagnosticSeverity::Warning)
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>();
            let errors = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == SimulatorDiagnosticSeverity::Error)
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>();

            let mut artifacts = Vec::new();
            if request.artifact_policy == ArtifactPolicy::RetainOnFailure
                && status != SimulationStatus::Succeeded
            {
                run_directory.retain();
                artifacts.push(SimulationArtifact {
                    kind: "run_directory".to_string(),
                    path: run_directory.path().to_string_lossy().into_owned(),
                });
            } else if let Err(error) = run_directory.cleanup() {
                let message = format!(
                    "Could not remove temporary simulation directory '{}': {error}",
                    run_directory.path().display()
                );
                warnings.push(message.clone());
                diagnostics.push(SimulatorDiagnostic {
                    code: "NL-S003".to_string(),
                    severity: SimulatorDiagnosticSeverity::Warning,
                    kind: SimulatorDiagnosticKind::Warning,
                    message,
                });
            }

            Ok(SimulationResult {
                schema_version: SIMULATION_SCHEMA_VERSION.to_string(),
                status,
                analyses: request.analyses.clone(),
                simulator: SimulatorInfo {
                    executable: self.executable.to_string_lossy().into_owned(),
                    version,
                },
                process: SimulatorProcessStatus {
                    exit_code: captured.exit_code,
                    success: captured.success,
                },
                measurements,
                datasets,
                diagnostics,
                warnings,
                errors,
                raw_log: SimulatorLog {
                    stdout: captured.stdout,
                    stderr: captured.stderr,
                },
                artifacts,
            })
        }
    }

    pub(super) struct RunDirectory {
        path: PathBuf,
        cleanup: bool,
    }

    impl RunDirectory {
        pub(super) fn create() -> Result<Self, SimulationRunError> {
            let root = env::temp_dir();
            for _ in 0..16 {
                let sequence = NEXT_RUN_ID.fetch_add(1, AtomicOrdering::Relaxed);
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let path = root.join(format!(
                    "netlang-sim-{}-{timestamp}-{sequence}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => {
                        return Ok(Self {
                            path,
                            cleanup: true,
                        });
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => {
                        return Err(run_error(
                            SimulationRunErrorKind::Io,
                            format!(
                                "Failed to create temporary simulation directory '{}': {error}",
                                path.display()
                            ),
                        ));
                    }
                }
            }
            Err(run_error(
                SimulationRunErrorKind::Io,
                "Failed to allocate a unique temporary simulation directory.".to_string(),
            ))
        }

        pub(super) fn path(&self) -> &Path {
            &self.path
        }

        fn retain(&mut self) {
            self.cleanup = false;
        }

        fn cleanup(&mut self) -> std::io::Result<()> {
            if self.cleanup && self.path.exists() {
                fs::remove_dir_all(&self.path)?;
                self.cleanup = false;
            }
            Ok(())
        }
    }

    impl Drop for RunDirectory {
        fn drop(&mut self) {
            let _ = self.cleanup();
        }
    }

    struct CapturedProcess {
        exit_code: Option<i32>,
        success: bool,
        timed_out: bool,
        cancelled: bool,
        stdout: String,
        stderr: String,
    }

    fn execute_process(
        executable: &Path,
        args: &[&str],
        current_dir: Option<&Path>,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<CapturedProcess, SimulationRunError> {
        let mut command = Command::new(executable);
        command
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(current_dir) = current_dir {
            command.current_dir(current_dir);
        }
        let mut child = command.spawn().map_err(|error| {
            run_error(
                SimulationRunErrorKind::Launch,
                format!(
                    "Failed to execute ngspice at '{}': {error}",
                    executable.display()
                ),
            )
        })?;

        let stdout = child
            .stdout
            .take()
            .expect("piped simulator stdout must be available");
        let stderr = child
            .stderr
            .take()
            .expect("piped simulator stderr must be available");
        let stdout_reader = thread::spawn(move || read_stream(stdout));
        let stderr_reader = thread::spawn(move || read_stream(stderr));

        let started = Instant::now();
        let mut timed_out = false;
        let mut cancelled = false;
        let exit_status = loop {
            if let Some(status) = child.try_wait().map_err(|error| {
                run_error(
                    SimulationRunErrorKind::Io,
                    format!("Failed while waiting for ngspice: {error}"),
                )
            })? {
                break status;
            }
            if cancellation.is_cancelled() {
                cancelled = true;
                let _ = child.kill();
                break child.wait().map_err(|error| {
                    run_error(
                        SimulationRunErrorKind::Io,
                        format!("Failed to reap cancelled ngspice process: {error}"),
                    )
                })?;
            }
            if started.elapsed() >= timeout {
                timed_out = true;
                let _ = child.kill();
                break child.wait().map_err(|error| {
                    run_error(
                        SimulationRunErrorKind::Io,
                        format!("Failed to reap timed-out ngspice process: {error}"),
                    )
                })?;
            }
            thread::sleep(Duration::from_millis(10));
        };

        let stdout = stdout_reader.join().map_err(|_| {
            run_error(
                SimulationRunErrorKind::Io,
                "Ngspice stdout reader panicked.",
            )
        })??;
        let stderr = stderr_reader.join().map_err(|_| {
            run_error(
                SimulationRunErrorKind::Io,
                "Ngspice stderr reader panicked.",
            )
        })??;

        Ok(CapturedProcess {
            exit_code: exit_status.code(),
            success: exit_status.success() && !timed_out && !cancelled,
            timed_out,
            cancelled,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }

    fn read_stream(mut stream: impl Read) -> Result<Vec<u8>, SimulationRunError> {
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).map_err(|error| {
            run_error(
                SimulationRunErrorKind::Io,
                format!("Failed to read ngspice output: {error}"),
            )
        })?;
        Ok(bytes)
    }

    fn discover_ngspice_path() -> PathBuf {
        if let Some(configured_path) = env::var_os("NETLANG_NGSPICE")
            && !configured_path.is_empty()
        {
            return PathBuf::from(configured_path);
        }

        let binary_name = if cfg!(windows) {
            "ngspice_con.exe"
        } else {
            "ngspice"
        };
        let mut candidates = vec![
            PathBuf::from("tools/ngspice/bin").join(binary_name),
            PathBuf::from("core/tools/ngspice/bin").join(binary_name),
            PathBuf::from("../core/tools/ngspice/bin").join(binary_name),
        ];
        if let Ok(executable) = env::current_exe()
            && let Some(parent) = executable.parent()
        {
            candidates.push(parent.join("tools/ngspice/bin").join(binary_name));
            for ancestor in parent.ancestors().take(5) {
                candidates.push(ancestor.join("core/tools/ngspice/bin").join(binary_name));
            }
        }
        candidates
            .into_iter()
            .find(|candidate| candidate.is_file())
            .unwrap_or_else(|| PathBuf::from(binary_name))
    }

    fn cancelled_before_launch(request: &SimulationRequest, executable: &Path) -> SimulationResult {
        SimulationResult {
            schema_version: SIMULATION_SCHEMA_VERSION.to_string(),
            status: SimulationStatus::Cancelled,
            analyses: request.analyses.clone(),
            simulator: SimulatorInfo {
                executable: executable.to_string_lossy().into_owned(),
                version: "not_probed".to_string(),
            },
            process: SimulatorProcessStatus {
                exit_code: None,
                success: false,
            },
            measurements: BTreeMap::new(),
            datasets: Vec::new(),
            diagnostics: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            raw_log: SimulatorLog {
                stdout: String::new(),
                stderr: String::new(),
            },
            artifacts: Vec::new(),
        }
    }

    fn display_exit_code(code: Option<i32>) -> String {
        code.map_or_else(|| "terminated".to_string(), |code| code.to_string())
    }

    fn run_error(kind: SimulationRunErrorKind, message: impl Into<String>) -> SimulationRunError {
        SimulationRunError {
            kind,
            message: message.into(),
        }
    }

    pub use self::NgspiceRunner as NativeNgspiceRunner;
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::NativeNgspiceRunner as NgspiceRunner;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_and_result_domain_is_versioned_and_serializable() {
        let request = SimulationRequest::new("* test\n.end\n", vec![Analysis::OperatingPoint]);
        let value = serde_json::to_value(&request).expect("request should serialize");
        assert_eq!(value["schema_version"], SIMULATION_SCHEMA_VERSION);
        assert_eq!(value["analyses"][0]["kind"], "operating_point");
        assert_eq!(value["artifact_policy"], "cleanup_always");
    }

    #[test]
    fn cancellation_token_is_shared_between_runner_and_caller() {
        let caller = CancellationToken::new();
        let runner = caller.clone();
        assert!(!runner.is_cancelled());
        caller.cancel();
        assert!(runner.is_cancelled());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn run_directories_are_unique_and_cleaned_by_default() {
        use super::native::RunDirectory;

        let first = RunDirectory::create().expect("first run directory should be created");
        let second = RunDirectory::create().expect("second run directory should be created");
        let first_path = first.path().to_path_buf();
        let second_path = second.path().to_path_buf();
        assert_ne!(first_path, second_path);
        assert!(first_path.is_dir());
        assert!(second_path.is_dir());
        drop(first);
        drop(second);
        assert!(!first_path.exists());
        assert!(!second_path.exists());
    }
}
