use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiPlanError {
    #[error("unsupported structured intent: {0}")]
    Unsupported(String),
    #[error("invalid structured intent: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "intent", rename_all = "snake_case")]
pub enum StructuredIntent {
    DiagnoseProject { path: String },
    InspectDatabase { connection: Value },
    CreateProject { template: String, path: String },
    FixPortConflict { port: u16 },
    InspectMachine,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub command: String,
    pub permission: String,
    pub params: Value,
    pub mutating: bool,
    pub requires_confirmation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolPlan {
    pub version: u32,
    pub intent: String,
    pub calls: Vec<ToolCall>,
    pub unrestricted_shell_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecuteRequest {
    pub intent: StructuredIntent,
    #[serde(default)]
    pub confirm_mutations: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolExecution {
    pub command: String,
    pub ok: bool,
    pub result: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionReport {
    pub plan: ToolPlan,
    pub results: Vec<ToolExecution>,
    pub completed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvaluationCase {
    pub name: String,
    pub intent: StructuredIntent,
    #[serde(default)]
    pub expected_commands: Vec<String>,
    #[serde(default)]
    pub forbid_mutations: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationCaseResult {
    pub name: String,
    pub passed: bool,
    pub errors: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvaluationReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub cases: Vec<EvaluationCaseResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidatePlanValidation {
    pub valid: bool,
    pub calls: usize,
    pub mutating_calls: usize,
    pub total_param_bytes: usize,
    pub errors: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiCapabilityReport {
    pub intents: Vec<String>,
    pub max_tool_calls: usize,
    pub unrestricted_shell_allowed: bool,
    pub requires_confirmation_for_mutations: bool,
    pub candidate_plan_validation: bool,
}

pub fn capabilities() -> AiCapabilityReport {
    AiCapabilityReport {
        intents: vec![
            "diagnose_project".into(),
            "inspect_database".into(),
            "create_project".into(),
            "fix_port_conflict".into(),
            "inspect_machine".into(),
        ],
        max_tool_calls: 16,
        unrestricted_shell_allowed: false,
        requires_confirmation_for_mutations: true,
        candidate_plan_validation: true,
    }
}
pub fn validate_candidate_plan(plan: &ToolPlan) -> CandidatePlanValidation {
    let mut errors = Vec::new();
    if plan.version != 1 {
        errors.push("unsupported tool plan version".into());
    }
    if plan.intent.trim().is_empty()
        || plan.intent.len() > 128
        || plan.intent.chars().any(char::is_control)
    {
        errors.push("plan intent is invalid".into());
    }
    if plan.unrestricted_shell_allowed {
        errors.push("unrestricted shell must remain disabled".into());
    }
    if plan.calls.is_empty() || plan.calls.len() > 16 {
        errors.push("tool plan must contain 1..16 calls".into());
    }
    let mut total_param_bytes = 0usize;
    let mut mutating = 0usize;
    for (index, call) in plan.calls.iter().enumerate() {
        if call.command.is_empty()
            || call.command.len() > 128
            || !call.command.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
            })
        {
            errors.push(format!("call {index} command is invalid"));
        }
        if call.command.starts_with("ai.") {
            errors.push(format!("call {index} recursively invokes AI"));
        }
        if call.permission.is_empty()
            || call.permission.len() > 128
            || !call.permission.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
            })
        {
            errors.push(format!("call {index} permission is invalid"));
        }
        let size = serde_json::to_vec(&call.params)
            .map(|v| v.len())
            .unwrap_or(usize::MAX);
        total_param_bytes = total_param_bytes.saturating_add(size);
        if size > 2 * 1024 * 1024 {
            errors.push(format!("call {index} params exceed 2 MiB"));
        }
        if call.mutating {
            mutating += 1;
            if !call.requires_confirmation {
                errors.push(format!("call {index} mutates without confirmation"));
            }
        }
    }
    if total_param_bytes > 4 * 1024 * 1024 {
        errors.push("combined tool parameters exceed 4 MiB".into());
    }
    CandidatePlanValidation {
        valid: errors.is_empty(),
        calls: plan.calls.len(),
        mutating_calls: mutating,
        total_param_bytes,
        errors,
    }
}
pub fn plan(intent: &StructuredIntent) -> Result<ToolPlan, AiPlanError> {
    let (name, calls) = match intent {
        StructuredIntent::DiagnoseProject { path } => {
            validate_path(path)?;
            (
                "diagnose_project",
                vec![
                    call(
                        "project.detect",
                        "project.view",
                        json!({"path":path}),
                        false,
                        false,
                    ),
                    call(
                        "project.dependencies",
                        "project.view",
                        json!({"path":path}),
                        false,
                        false,
                    ),
                    call("runtime.list", "runtime.view", json!({}), false, false),
                    call("port.list", "network.view", json!({}), false, false),
                ],
            )
        }
        StructuredIntent::InspectDatabase { connection } => (
            "inspect_database",
            vec![call(
                "database.cli.inspect",
                "database.view",
                json!({"connection":connection}),
                false,
                false,
            )],
        ),
        StructuredIntent::CreateProject { template, path } => {
            if template.trim().is_empty() || template.len() > 64 {
                return Err(AiPlanError::Invalid("template is invalid".into()));
            }
            validate_path(path)?;
            (
                "create_project",
                vec![
                    call(
                        "project.bootstrap-plan",
                        "project.edit",
                        json!({"template":template,"path":path}),
                        false,
                        false,
                    ),
                    call(
                        "project.bootstrap",
                        "project.edit",
                        json!({"template":template,"path":path}),
                        true,
                        true,
                    ),
                ],
            )
        }
        StructuredIntent::FixPortConflict { port } => (
            "fix_port_conflict",
            vec![call(
                "port.check",
                "network.view",
                json!({"port":port}),
                false,
                false,
            )],
        ),
        StructuredIntent::InspectMachine => (
            "inspect_machine",
            vec![
                call("status", "machine.view", json!({}), false, false),
                call("process.list", "machine.view", json!({}), false, false),
                call("port.list", "network.view", json!({}), false, false),
            ],
        ),
    };
    Ok(ToolPlan {
        version: 1,
        intent: name.into(),
        calls,
        unrestricted_shell_allowed: false,
    })
}

pub fn evaluate(cases: &[EvaluationCase]) -> EvaluationReport {
    let mut results = Vec::new();
    for case in cases {
        let mut errors = Vec::new();
        match plan(&case.intent) {
            Ok(plan) => {
                let commands = plan
                    .calls
                    .iter()
                    .map(|c| c.command.clone())
                    .collect::<Vec<_>>();
                for expected in &case.expected_commands {
                    if !commands.iter().any(|v| v == expected) {
                        errors.push(format!("missing expected command: {expected}"));
                    }
                }
                if case.forbid_mutations && plan.calls.iter().any(|c| c.mutating) {
                    errors.push("plan contains a mutation while forbid_mutations=true".into());
                }
                if plan.unrestricted_shell_allowed {
                    errors.push("unrestricted shell was enabled".into());
                }
                for call in &plan.calls {
                    if call.mutating && !call.requires_confirmation {
                        errors.push(format!(
                            "mutating command {} does not require confirmation",
                            call.command
                        ));
                    }
                }
                let validation = validate_candidate_plan(&plan);
                errors.extend(
                    validation
                        .errors
                        .into_iter()
                        .map(|e| format!("candidate-plan validation: {e}")),
                );
            }
            Err(e) => errors.push(e.to_string()),
        }
        results.push(EvaluationCaseResult {
            name: case.name.clone(),
            passed: errors.is_empty(),
            errors,
        });
    }
    let passed = results.iter().filter(|r| r.passed).count();
    EvaluationReport {
        total: results.len(),
        passed,
        failed: results.len().saturating_sub(passed),
        cases: results,
    }
}
pub fn evaluate_json(bytes: &[u8]) -> Result<EvaluationReport, AiPlanError> {
    let cases: Vec<EvaluationCase> = serde_json::from_slice(bytes)
        .map_err(|e| AiPlanError::Invalid(format!("evaluation JSON is invalid: {e}")))?;
    if cases.is_empty() || cases.len() > 1024 {
        return Err(AiPlanError::Invalid(
            "evaluation suite must contain 1..1024 cases".into(),
        ));
    }
    Ok(evaluate(&cases))
}

fn call(
    command: &str,
    permission: &str,
    params: Value,
    mutating: bool,
    requires_confirmation: bool,
) -> ToolCall {
    ToolCall {
        command: command.into(),
        permission: permission.into(),
        params,
        mutating,
        requires_confirmation,
    }
}
fn validate_path(path: &str) -> Result<(), AiPlanError> {
    if path.trim().is_empty() || path.len() > 4096 || path.contains('\0') {
        Err(AiPlanError::Invalid("path is invalid".into()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ai_plans_never_enable_unrestricted_shell() {
        let plan = plan(&StructuredIntent::InspectMachine).unwrap();
        assert!(!plan.unrestricted_shell_allowed);
    }
    #[test]
    fn mutating_project_creation_requires_confirmation() {
        let plan = plan(&StructuredIntent::CreateProject {
            template: "laravel".into(),
            path: "C:/work/app".into(),
        })
        .unwrap();
        assert!(plan
            .calls
            .iter()
            .any(|c| c.mutating && c.requires_confirmation));
    }
    #[test]
    fn candidate_plan_rejects_recursive_ai() {
        let p = ToolPlan {
            version: 1,
            intent: "x".into(),
            calls: vec![ToolCall {
                command: "ai.execute".into(),
                permission: "machine.view".into(),
                params: json!({}),
                mutating: false,
                requires_confirmation: false,
            }],
            unrestricted_shell_allowed: false,
        };
        assert!(!validate_candidate_plan(&p).valid);
    }
    #[test]
    fn evaluation_detects_expected_commands() {
        let report = evaluate(&[EvaluationCase {
            name: "machine".into(),
            intent: StructuredIntent::InspectMachine,
            expected_commands: vec!["status".into(), "process.list".into()],
            forbid_mutations: true,
        }]);
        assert_eq!(report.failed, 0);
    }
}

// ---------- 0.24 model-adapter boundary + telemetry ----------
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelAdapterTransport {
    Extension,
    LocalProcess,
    LocalHttp,
    RemoteHttps,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelAdapterDescriptor {
    pub id: String,
    pub transport: ModelAdapterTransport,
    pub emits_tool_plan_v1: bool,
    pub network_required: bool,
    pub secret_reference_required: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelOutputValidation {
    pub adapter_id: String,
    pub accepted: bool,
    pub plan: Option<ToolPlan>,
    pub validation: CandidatePlanValidation,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiTelemetryRecord {
    pub timestamp_unix_ms: u128,
    pub adapter_id: String,
    pub intent: String,
    pub accepted: bool,
    pub calls: usize,
    pub mutating_calls: usize,
    pub completed: bool,
    pub duration_ms: u128,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiTelemetrySummary {
    pub records: usize,
    pub accepted: usize,
    pub completed: usize,
    pub rejected: usize,
    pub adapters: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiConformanceReport {
    pub tool_plan_v1: bool,
    pub candidate_plan_validation: bool,
    pub mutation_confirmation: bool,
    pub recursive_ai_blocked: bool,
    pub unrestricted_shell_blocked: bool,
    pub model_adapter_boundary: bool,
    pub evaluation_harness: bool,
    pub telemetry: bool,
    pub issues: Vec<String>,
}

pub fn validate_model_output(
    adapter: &ModelAdapterDescriptor,
    bytes: &[u8],
) -> ModelOutputValidation {
    let mut invalid = CandidatePlanValidation {
        valid: false,
        calls: 0,
        mutating_calls: 0,
        total_param_bytes: 0,
        errors: vec![],
    };
    if validate_adapter(adapter).is_err() {
        invalid
            .errors
            .push("model adapter descriptor is invalid".into());
        return ModelOutputValidation {
            adapter_id: adapter.id.clone(),
            accepted: false,
            plan: None,
            validation: invalid,
            error: Some("invalid adapter descriptor".into()),
        };
    }
    if bytes.len() > 4 * 1024 * 1024 {
        invalid.errors.push("model output exceeds 4 MiB".into());
        return ModelOutputValidation {
            adapter_id: adapter.id.clone(),
            accepted: false,
            plan: None,
            validation: invalid,
            error: Some("model output too large".into()),
        };
    }
    let plan = match serde_json::from_slice::<ToolPlan>(bytes) {
        Ok(v) => v,
        Err(e) => {
            invalid
                .errors
                .push(format!("model output is not ToolPlan JSON: {e}"));
            return ModelOutputValidation {
                adapter_id: adapter.id.clone(),
                accepted: false,
                plan: None,
                validation: invalid,
                error: Some("invalid ToolPlan JSON".into()),
            };
        }
    };
    let validation = validate_candidate_plan(&plan);
    ModelOutputValidation {
        adapter_id: adapter.id.clone(),
        accepted: validation.valid,
        plan: Some(plan),
        validation,
        error: None,
    }
}
pub fn validate_adapter(adapter: &ModelAdapterDescriptor) -> Result<(), AiPlanError> {
    if adapter.id.is_empty()
        || adapter.id.len() > 128
        || !adapter.id.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-')
        })
    {
        return Err(AiPlanError::Invalid("model adapter id is invalid".into()));
    }
    if !adapter.emits_tool_plan_v1 {
        return Err(AiPlanError::Invalid(
            "model adapter must emit ToolPlan v1".into(),
        ));
    }
    if matches!(adapter.transport, ModelAdapterTransport::RemoteHttps) && !adapter.network_required
    {
        return Err(AiPlanError::Invalid(
            "remote HTTPS adapter must declare network_required".into(),
        ));
    }
    Ok(())
}
pub fn append_telemetry(
    path: &std::path::Path,
    record: &AiTelemetryRecord,
) -> Result<(), AiPlanError> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| AiPlanError::Invalid(e.to_string()))?;
    }
    if path.exists() && std::fs::metadata(path).map(|m| m.len()).unwrap_or(0) > 64 * 1024 * 1024 {
        return Err(AiPlanError::Invalid(
            "AI telemetry file reached 64 MiB rotation ceiling".into(),
        ));
    }
    let mut line = serde_json::to_vec(record).map_err(|e| AiPlanError::Invalid(e.to_string()))?;
    line.push(b'\n');
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| AiPlanError::Invalid(e.to_string()))?;
    f.write_all(&line)
        .map_err(|e| AiPlanError::Invalid(e.to_string()))?;
    f.sync_data()
        .map_err(|e| AiPlanError::Invalid(e.to_string()))?;
    Ok(())
}
pub fn telemetry_summary(path: &std::path::Path) -> Result<AiTelemetrySummary, AiPlanError> {
    if !path.exists() {
        return Ok(AiTelemetrySummary {
            records: 0,
            accepted: 0,
            completed: 0,
            rejected: 0,
            adapters: vec![],
        });
    }
    let text = std::fs::read_to_string(path).map_err(|e| AiPlanError::Invalid(e.to_string()))?;
    let mut records = 0usize;
    let mut accepted = 0usize;
    let mut completed = 0usize;
    let mut adapters = std::collections::BTreeSet::new();
    for line in text.lines().rev().take(10_000) {
        if line.trim().is_empty() {
            continue;
        }
        let r: AiTelemetryRecord = serde_json::from_str(line)
            .map_err(|e| AiPlanError::Invalid(format!("AI telemetry record invalid: {e}")))?;
        records += 1;
        if r.accepted {
            accepted += 1;
        }
        if r.completed {
            completed += 1;
        }
        adapters.insert(r.adapter_id);
    }
    Ok(AiTelemetrySummary {
        records,
        accepted,
        completed,
        rejected: records.saturating_sub(accepted),
        adapters: adapters.into_iter().collect(),
    })
}
pub fn conformance() -> AiConformanceReport {
    let recursive = ToolPlan {
        version: 1,
        intent: "test".into(),
        calls: vec![ToolCall {
            command: "ai.execute".into(),
            permission: "machine.view".into(),
            params: json!({}),
            mutating: false,
            requires_confirmation: false,
        }],
        unrestricted_shell_allowed: false,
    };
    let shell = ToolPlan {
        unrestricted_shell_allowed: true,
        ..recursive.clone()
    };
    let mut issues = Vec::new();
    let recursive_ai_blocked = !validate_candidate_plan(&recursive).valid;
    let unrestricted_shell_blocked = !validate_candidate_plan(&shell).valid;
    if !recursive_ai_blocked {
        issues.push("recursive AI calls are not blocked".into());
    }
    if !unrestricted_shell_blocked {
        issues.push("unrestricted shell is not blocked".into());
    }
    AiConformanceReport {
        tool_plan_v1: true,
        candidate_plan_validation: true,
        mutation_confirmation: true,
        recursive_ai_blocked,
        unrestricted_shell_blocked,
        model_adapter_boundary: true,
        evaluation_harness: true,
        telemetry: true,
        issues,
    }
}

#[cfg(test)]
mod model_adapter_tests {
    use super::*;
    #[test]
    fn model_output_still_passes_policy_gate() {
        let a = ModelAdapterDescriptor {
            id: "local.test".into(),
            transport: ModelAdapterTransport::LocalProcess,
            emits_tool_plan_v1: true,
            network_required: false,
            secret_reference_required: false,
        };
        let p = plan(&StructuredIntent::InspectMachine).unwrap();
        let v = validate_model_output(&a, &serde_json::to_vec(&p).unwrap());
        assert!(v.accepted);
    }
    #[test]
    fn invalid_adapter_cannot_bypass_plan_validation() {
        let a = ModelAdapterDescriptor {
            id: "BAD SPACE".into(),
            transport: ModelAdapterTransport::RemoteHttps,
            emits_tool_plan_v1: true,
            network_required: false,
            secret_reference_required: true,
        };
        assert!(!validate_model_output(&a, b"{}").accepted);
    }
}
