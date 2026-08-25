use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataModel {
    Relational,
    Document,
    KeyValue,
    Graph,
    Search,
    TimeSeries,
    Column,
    Vector,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Integer,
    Decimal,
    Boolean,
    Text,
    LongText,
    Date,
    DateTime,
    Json,
    Binary,
    Uuid,
    Enum,
    Relation,
    Geo,
    Vector,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldMeta {
    pub name: String,
    pub field_type: FieldType,
    pub nullable: bool,
    pub primary: bool,
    pub generated: bool,
    pub enum_values: Vec<String>,
    pub relation_target: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityMeta {
    pub name: String,
    pub display_name: String,
    pub fields: Vec<FieldMeta>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CapabilitySet {
    pub connect: bool,
    pub introspect: bool,
    pub query: bool,
    pub browse: bool,
    pub insert: bool,
    pub update: bool,
    pub delete: bool,
    pub schemas: bool,
    pub indexes: bool,
    pub relations: bool,
    pub functions: bool,
    pub users: bool,
    pub permissions: bool,
    pub import: bool,
    pub export: bool,
    pub backup: bool,
    pub restore: bool,
    pub statistics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrowseRequest {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u64,
    pub order_by: Option<String>,
    #[serde(default)]
    pub descending: bool,
}
fn default_limit() -> u32 {
    100
}
impl Default for BrowseRequest {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            offset: 0,
            order_by: None,
            descending: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrowsePage {
    pub entity: String,
    pub columns: Vec<String>,
    pub rows: Vec<Value>,
    pub total_rows: u64,
    pub limit: u32,
    pub offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutationRequest {
    #[serde(default)]
    pub values: BTreeMap<String, Value>,
    #[serde(default)]
    pub filter: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MutationResult {
    pub affected_rows: u64,
    pub last_insert_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexMeta {
    pub name: String,
    pub unique: bool,
    pub primary: bool,
    pub columns: Vec<String>,
    pub metadata: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationMeta {
    pub name: String,
    pub from_entity: String,
    pub from_fields: Vec<String>,
    pub to_entity: String,
    pub to_fields: Vec<String>,
    pub on_update: Option<String>,
    pub on_delete: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityStatistics {
    pub entity: String,
    pub row_count: Option<u64>,
    pub storage_bytes: Option<u64>,
    pub index_bytes: Option<u64>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionMeta {
    pub name: String,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatabaseUserMeta {
    pub name: String,
    #[serde(default)]
    pub metadata: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatabasePermissionMeta {
    pub principal: String,
    pub permission: String,
    #[serde(default)]
    pub resource: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataTransferRequest {
    pub format: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub entity: Option<String>,
    pub path: String,
    #[serde(default)]
    pub options: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataTransferResult {
    pub format: String,
    pub path: String,
    pub records: Option<u64>,
    pub bytes: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BackupRequest {
    pub path: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub options: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupResult {
    pub path: String,
    pub bytes: Option<u64>,
    #[serde(default)]
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub sdk_version: u32,
    pub id: String,
    pub model: DataModel,
    pub capabilities: CapabilitySet,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConformanceIssue {
    pub severity: String,
    pub capability: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConformanceReport {
    pub descriptor: ProviderDescriptor,
    pub passed: bool,
    pub issues: Vec<ProviderConformanceIssue>,
}
pub const DATABASE_SDK_VERSION: u32 = 1;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("database provider error: {0}")]
    Provider(String),
    #[error("unsupported database capability: {0}")]
    Unsupported(&'static str),
    #[error("invalid database request: {0}")]
    Invalid(String),
}

pub trait DatabaseProvider: Send + Sync {
    fn id(&self) -> &str;
    fn model(&self) -> DataModel;
    fn capabilities(&self) -> CapabilitySet;
    fn connect(&mut self, connection: &Value) -> Result<(), DatabaseError>;
    fn disconnect(&mut self) -> Result<(), DatabaseError>;
    fn list_namespaces(&self) -> Result<Vec<String>, DatabaseError>;
    fn list_entities(&self, namespace: Option<&str>) -> Result<Vec<String>, DatabaseError>;
    fn describe_entity(
        &self,
        namespace: Option<&str>,
        entity: &str,
    ) -> Result<EntityMeta, DatabaseError>;
    fn query(&self, statement: &str, parameters: &Value) -> Result<Value, DatabaseError>;
    fn browse(
        &self,
        _namespace: Option<&str>,
        _entity: &str,
        _request: &BrowseRequest,
    ) -> Result<BrowsePage, DatabaseError> {
        Err(DatabaseError::Unsupported("browse"))
    }
    fn insert(
        &self,
        _namespace: Option<&str>,
        _entity: &str,
        _request: &MutationRequest,
    ) -> Result<MutationResult, DatabaseError> {
        Err(DatabaseError::Unsupported("insert"))
    }
    fn update(
        &self,
        _namespace: Option<&str>,
        _entity: &str,
        _request: &MutationRequest,
    ) -> Result<MutationResult, DatabaseError> {
        Err(DatabaseError::Unsupported("update"))
    }
    fn delete(
        &self,
        _namespace: Option<&str>,
        _entity: &str,
        _request: &MutationRequest,
    ) -> Result<MutationResult, DatabaseError> {
        Err(DatabaseError::Unsupported("delete"))
    }
    fn list_indexes(
        &self,
        _namespace: Option<&str>,
        _entity: &str,
    ) -> Result<Vec<IndexMeta>, DatabaseError> {
        Err(DatabaseError::Unsupported("indexes"))
    }
    fn list_relations(
        &self,
        _namespace: Option<&str>,
        _entity: &str,
    ) -> Result<Vec<RelationMeta>, DatabaseError> {
        Err(DatabaseError::Unsupported("relations"))
    }
    fn statistics(
        &self,
        _namespace: Option<&str>,
        _entity: &str,
    ) -> Result<EntityStatistics, DatabaseError> {
        Err(DatabaseError::Unsupported("statistics"))
    }
    fn list_functions(&self, _namespace: Option<&str>) -> Result<Vec<FunctionMeta>, DatabaseError> {
        Err(DatabaseError::Unsupported("functions"))
    }
    fn list_users(&self) -> Result<Vec<DatabaseUserMeta>, DatabaseError> {
        Err(DatabaseError::Unsupported("users"))
    }
    fn list_permissions(
        &self,
        _principal: Option<&str>,
    ) -> Result<Vec<DatabasePermissionMeta>, DatabaseError> {
        Err(DatabaseError::Unsupported("permissions"))
    }
    fn import_data(
        &self,
        _request: &DataTransferRequest,
    ) -> Result<DataTransferResult, DatabaseError> {
        Err(DatabaseError::Unsupported("import"))
    }
    fn export_data(
        &self,
        _request: &DataTransferRequest,
    ) -> Result<DataTransferResult, DatabaseError> {
        Err(DatabaseError::Unsupported("export"))
    }
    fn backup(&self, _request: &BackupRequest) -> Result<BackupResult, DatabaseError> {
        Err(DatabaseError::Unsupported("backup"))
    }
    fn restore(&self, _request: &BackupRequest) -> Result<BackupResult, DatabaseError> {
        Err(DatabaseError::Unsupported("restore"))
    }
}

pub fn provider_descriptor(provider: &dyn DatabaseProvider) -> ProviderDescriptor {
    ProviderDescriptor {
        sdk_version: DATABASE_SDK_VERSION,
        id: provider.id().to_string(),
        model: provider.model(),
        capabilities: provider.capabilities(),
    }
}
pub fn validate_provider_descriptor(descriptor: &ProviderDescriptor) -> ProviderConformanceReport {
    let mut issues = Vec::new();
    if descriptor.sdk_version != DATABASE_SDK_VERSION {
        issues.push(ProviderConformanceIssue {
            severity: "error".into(),
            capability: "sdk_version".into(),
            message: format!(
                "provider SDK version {} does not match {}",
                descriptor.sdk_version, DATABASE_SDK_VERSION
            ),
        });
    }
    if descriptor.id.is_empty()
        || descriptor.id.len() > 96
        || !descriptor.id.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.')
        })
    {
        issues.push(ProviderConformanceIssue {
            severity: "error".into(),
            capability: "id".into(),
            message: "provider id must be a lowercase stable identifier".into(),
        });
    }
    let c = &descriptor.capabilities;
    if !c.connect {
        for (name, enabled) in [
            ("introspect", c.introspect),
            ("query", c.query),
            ("browse", c.browse),
            ("insert", c.insert),
            ("update", c.update),
            ("delete", c.delete),
            ("backup", c.backup),
            ("restore", c.restore),
        ] {
            if enabled {
                issues.push(ProviderConformanceIssue {
                    severity: "error".into(),
                    capability: name.into(),
                    message: format!("{name} requires connect capability"),
                });
            }
        }
    }
    if c.browse && !c.introspect {
        issues.push(ProviderConformanceIssue {
            severity: "error".into(),
            capability: "browse".into(),
            message: "browse requires introspection metadata".into(),
        });
    }
    if (c.indexes || c.relations || c.statistics || c.schemas) && !c.introspect {
        issues.push(ProviderConformanceIssue {
            severity: "error".into(),
            capability: "introspect".into(),
            message: "schema/index/relation/statistics capabilities require introspect".into(),
        });
    }
    if (c.insert || c.update || c.delete) && !c.browse {
        issues.push(ProviderConformanceIssue {
            severity: "warning".into(),
            capability: "mutation".into(),
            message:
                "mutation-capable provider without browse support limits schema-driven studio UX"
                    .into(),
        });
    }
    ProviderConformanceReport {
        descriptor: descriptor.clone(),
        passed: !issues.iter().any(|i| i.severity == "error"),
        issues,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiControl {
    Number,
    Decimal,
    Toggle,
    Text,
    TextArea,
    Date,
    DateTime,
    JsonEditor,
    FileViewer,
    Uuid,
    Select,
    RelationSelector,
    Map,
    VectorViewer,
    Raw,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiField {
    pub name: String,
    pub label: String,
    pub control: UiControl,
    pub required: bool,
    pub read_only: bool,
    pub options: Vec<String>,
    pub relation_target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityUiSchema {
    pub entity: String,
    pub fields: Vec<UiField>,
    pub tabs: Vec<String>,
    pub actions: Vec<String>,
}

pub fn generate_ui_schema(entity: &EntityMeta, capabilities: &CapabilitySet) -> EntityUiSchema {
    let fields = entity
        .fields
        .iter()
        .map(|field| UiField {
            name: field.name.clone(),
            label: humanize(&field.name),
            control: control_for(&field.field_type),
            required: !field.nullable && !field.generated,
            read_only: field.generated,
            options: field.enum_values.clone(),
            relation_target: field.relation_target.clone(),
        })
        .collect();
    let mut tabs = vec!["browse".into(), "structure".into()];
    if capabilities.indexes {
        tabs.push("indexes".into());
    }
    if capabilities.relations {
        tabs.push("relations".into());
    }
    if capabilities.query {
        tabs.push("query".into());
    }
    if capabilities.statistics {
        tabs.push("statistics".into());
    }
    let mut actions = Vec::new();
    if capabilities.insert {
        actions.push("insert".into());
    }
    if capabilities.update {
        actions.push("update".into());
    }
    if capabilities.delete {
        actions.push("delete".into());
    }
    if capabilities.import {
        actions.push("import".into());
    }
    if capabilities.export {
        actions.push("export".into());
    }
    if capabilities.backup {
        actions.push("backup".into());
    }
    EntityUiSchema {
        entity: entity.name.clone(),
        fields,
        tabs,
        actions,
    }
}

pub fn workspace_for_model(model: DataModel) -> Vec<&'static str> {
    match model {
        DataModel::Relational => vec!["schemas", "tables", "views", "query"],
        DataModel::Document => vec!["collections", "documents", "json_editor", "query"],
        DataModel::KeyValue => vec!["keys", "value_editor", "ttl", "memory"],
        DataModel::Graph => vec!["nodes", "relationships", "graph_view", "query"],
        DataModel::Search => vec!["indexes", "documents", "mappings", "search"],
        DataModel::TimeSeries => vec!["measurements", "series", "time_chart", "query"],
        DataModel::Column => vec!["keyspaces", "tables", "partitions", "query"],
        DataModel::Vector => vec!["collections", "vectors", "metadata", "similarity_search"],
        DataModel::Custom => vec!["entities", "raw", "query"],
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatabaseStudioModelCoverage {
    pub model: DataModel,
    pub workspace_tabs: Vec<String>,
    pub analyzer: bool,
    pub ui_schema: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DatabaseStudioConformanceReport {
    pub ok: bool,
    pub models: Vec<DatabaseStudioModelCoverage>,
    pub missing: Vec<String>,
}
pub fn database_studio_conformance() -> DatabaseStudioConformanceReport {
    let models = [
        DataModel::Relational,
        DataModel::Document,
        DataModel::KeyValue,
        DataModel::Graph,
        DataModel::Search,
        DataModel::TimeSeries,
        DataModel::Column,
        DataModel::Vector,
        DataModel::Custom,
    ];
    let mut coverage = Vec::new();
    let mut missing = Vec::new();
    for model in models {
        let tabs = workspace_for_model(model.clone())
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if tabs.is_empty() {
            missing.push(format!("{:?}:workspace", model));
        }
        let analyzer = matches!(
            model,
            DataModel::Document
                | DataModel::KeyValue
                | DataModel::Graph
                | DataModel::Search
                | DataModel::TimeSeries
                | DataModel::Column
                | DataModel::Vector
        );
        let ui_schema = !tabs.is_empty();
        coverage.push(DatabaseStudioModelCoverage {
            model,
            workspace_tabs: tabs,
            analyzer,
            ui_schema,
        });
    }
    DatabaseStudioConformanceReport {
        ok: missing.is_empty(),
        models: coverage,
        missing,
    }
}

fn control_for(field_type: &FieldType) -> UiControl {
    match field_type {
        FieldType::Integer => UiControl::Number,
        FieldType::Decimal => UiControl::Decimal,
        FieldType::Boolean => UiControl::Toggle,
        FieldType::Text => UiControl::Text,
        FieldType::LongText => UiControl::TextArea,
        FieldType::Date => UiControl::Date,
        FieldType::DateTime => UiControl::DateTime,
        FieldType::Json => UiControl::JsonEditor,
        FieldType::Binary => UiControl::FileViewer,
        FieldType::Uuid => UiControl::Uuid,
        FieldType::Enum => UiControl::Select,
        FieldType::Relation => UiControl::RelationSelector,
        FieldType::Geo => UiControl::Map,
        FieldType::Vector => UiControl::VectorViewer,
        FieldType::Unknown => UiControl::Raw,
    }
}
fn humanize(value: &str) -> String {
    value
        .split('_')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "model", rename_all = "snake_case")]
pub enum AdvancedModelRequest {
    Document {
        documents: Vec<Value>,
    },
    KeyValue {
        entries: Vec<KeyValueSample>,
    },
    Vector {
        vectors: Vec<Vec<f64>>,
    },
    Graph {
        nodes: Vec<GraphNodeSample>,
        edges: Vec<GraphEdgeSample>,
    },
    Search {
        documents: Vec<SearchDocumentSample>,
    },
    TimeSeries {
        points: Vec<TimeSeriesPointSample>,
    },
    Column {
        rows: Vec<ColumnRowSample>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyValueSample {
    pub key: String,
    pub value: Value,
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphNodeSample {
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphEdgeSample {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchDocumentSample {
    pub id: String,
    pub fields: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeSeriesPointSample {
    pub series: String,
    pub timestamp_unix_ms: i64,
    pub value: f64,
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnRowSample {
    pub partition_key: String,
    #[serde(default)]
    pub clustering_key: Option<String>,
    pub columns: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdvancedModelAnalysis {
    pub model: DataModel,
    pub records: usize,
    pub fields: BTreeMap<String, BTreeMap<String, u64>>,
    pub metrics: BTreeMap<String, Value>,
    pub warnings: Vec<String>,
}

pub fn analyze_advanced_model(
    request: &AdvancedModelRequest,
) -> Result<AdvancedModelAnalysis, DatabaseError> {
    match request {
        AdvancedModelRequest::Document { documents } => analyze_documents(documents),
        AdvancedModelRequest::KeyValue { entries } => analyze_key_values(entries),
        AdvancedModelRequest::Vector { vectors } => analyze_vectors(vectors),
        AdvancedModelRequest::Graph { nodes, edges } => analyze_graph(nodes, edges),
        AdvancedModelRequest::Search { documents } => analyze_search(documents),
        AdvancedModelRequest::TimeSeries { points } => analyze_time_series(points),
        AdvancedModelRequest::Column { rows } => analyze_column(rows),
    }
}
fn ensure_analysis_count(count: usize) -> Result<(), DatabaseError> {
    if count > 10_000 {
        Err(DatabaseError::Invalid(
            "advanced model analysis is limited to 10,000 records".into(),
        ))
    } else {
        Ok(())
    }
}
fn value_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "decimal"
            }
        }
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
fn analyze_documents(documents: &[Value]) -> Result<AdvancedModelAnalysis, DatabaseError> {
    ensure_analysis_count(documents.len())?;
    let mut fields: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut warnings = Vec::new();
    for (doc_index, doc) in documents.iter().enumerate() {
        let obj = doc.as_object().ok_or_else(|| {
            DatabaseError::Invalid(format!("document {doc_index} is not an object"))
        })?;
        if obj.len() > 1024 {
            return Err(DatabaseError::Invalid(
                "document exceeds 1,024 top-level fields".into(),
            ));
        }
        for (k, v) in obj {
            if k.len() > 512 || k.chars().any(char::is_control) {
                return Err(DatabaseError::Invalid(
                    "document field name is invalid".into(),
                ));
            }
            *fields
                .entry(k.clone())
                .or_default()
                .entry(value_kind(v).into())
                .or_default() += 1;
        }
    }
    for (name, kinds) in &fields {
        if kinds.len() > 2 {
            warnings.push(format!(
                "field {name} has {} observed value types",
                kinds.len()
            ));
        }
    }
    let mut metrics = BTreeMap::new();
    metrics.insert("distinct_fields".into(), Value::from(fields.len() as u64));
    metrics.insert(
        "heterogeneous_fields".into(),
        Value::from(warnings.len() as u64),
    );
    Ok(AdvancedModelAnalysis {
        model: DataModel::Document,
        records: documents.len(),
        fields,
        metrics,
        warnings,
    })
}
fn analyze_key_values(entries: &[KeyValueSample]) -> Result<AdvancedModelAnalysis, DatabaseError> {
    ensure_analysis_count(entries.len())?;
    let mut fields = BTreeMap::new();
    let mut types = BTreeMap::new();
    let mut ttl = 0u64;
    let mut seen = std::collections::HashSet::new();
    let mut warnings = Vec::new();
    for item in entries {
        if item.key.is_empty() || item.key.len() > 4096 || item.key.chars().any(char::is_control) {
            return Err(DatabaseError::Invalid("key-value key is invalid".into()));
        }
        if !seen.insert(item.key.clone()) {
            warnings.push(format!("duplicate key sample: {}", item.key));
        }
        *types.entry(value_kind(&item.value).into()).or_insert(0) += 1;
        if item.ttl_seconds.is_some() {
            ttl += 1;
        }
    }
    fields.insert("value".into(), types);
    let mut metrics = BTreeMap::new();
    metrics.insert("ttl_entries".into(), Value::from(ttl));
    metrics.insert("unique_keys".into(), Value::from(seen.len() as u64));
    Ok(AdvancedModelAnalysis {
        model: DataModel::KeyValue,
        records: entries.len(),
        fields,
        metrics,
        warnings,
    })
}
fn analyze_vectors(vectors: &[Vec<f64>]) -> Result<AdvancedModelAnalysis, DatabaseError> {
    ensure_analysis_count(vectors.len())?;
    let dimension = vectors.first().map(Vec::len).unwrap_or(0);
    if dimension > 4096 {
        return Err(DatabaseError::Invalid(
            "vector dimension exceeds 4,096".into(),
        ));
    }
    let mut min_norm = f64::INFINITY;
    let mut max_norm = 0f64;
    let mut sum_norm = 0f64;
    for (vindex, vector) in vectors.iter().enumerate() {
        if vector.len() != dimension {
            return Err(DatabaseError::Invalid(format!(
                "vector {vindex} dimension mismatch"
            )));
        }
        if vector.iter().any(|x| !x.is_finite()) {
            return Err(DatabaseError::Invalid(format!(
                "vector {vindex} contains non-finite values"
            )));
        }
        let norm = vector.iter().map(|x| x * x).sum::<f64>().sqrt();
        min_norm = min_norm.min(norm);
        max_norm = max_norm.max(norm);
        sum_norm += norm;
    }
    if vectors.is_empty() {
        min_norm = 0.0;
    }
    let mut metrics = BTreeMap::new();
    metrics.insert("dimension".into(), Value::from(dimension as u64));
    metrics.insert("min_l2_norm".into(), serde_json::json!(min_norm));
    metrics.insert("max_l2_norm".into(), serde_json::json!(max_norm));
    metrics.insert(
        "average_l2_norm".into(),
        serde_json::json!(if vectors.is_empty() {
            0.0
        } else {
            sum_norm / vectors.len() as f64
        }),
    );
    Ok(AdvancedModelAnalysis {
        model: DataModel::Vector,
        records: vectors.len(),
        fields: BTreeMap::new(),
        metrics,
        warnings: Vec::new(),
    })
}
fn analyze_graph(
    nodes: &[GraphNodeSample],
    edges: &[GraphEdgeSample],
) -> Result<AdvancedModelAnalysis, DatabaseError> {
    ensure_analysis_count(nodes.len().saturating_add(edges.len()))?;
    let mut ids = std::collections::HashSet::new();
    let mut labels: BTreeMap<String, u64> = BTreeMap::new();
    for n in nodes {
        if n.id.is_empty() || n.id.len() > 512 || !ids.insert(n.id.clone()) {
            return Err(DatabaseError::Invalid(
                "graph node IDs must be unique, non-empty and <= 512 characters".into(),
            ));
        }
        if let Some(label) = &n.label {
            *labels.entry(label.clone()).or_default() += 1;
        }
    }
    let mut dangling = 0u64;
    let mut edge_labels: BTreeMap<String, u64> = BTreeMap::new();
    for e in edges {
        if !ids.contains(&e.from) || !ids.contains(&e.to) {
            dangling += 1;
        }
        if let Some(label) = &e.label {
            *edge_labels.entry(label.clone()).or_default() += 1;
        }
    }
    let mut metrics = BTreeMap::new();
    metrics.insert("nodes".into(), Value::from(nodes.len() as u64));
    metrics.insert("edges".into(), Value::from(edges.len() as u64));
    metrics.insert("dangling_edges".into(), Value::from(dangling));
    metrics.insert(
        "node_labels".into(),
        serde_json::to_value(labels).unwrap_or(Value::Null),
    );
    metrics.insert(
        "edge_labels".into(),
        serde_json::to_value(edge_labels).unwrap_or(Value::Null),
    );
    let warnings = if dangling > 0 {
        vec![format!("{dangling} edges reference missing nodes")]
    } else {
        Vec::new()
    };
    Ok(AdvancedModelAnalysis {
        model: DataModel::Graph,
        records: nodes.len() + edges.len(),
        fields: BTreeMap::new(),
        metrics,
        warnings,
    })
}

fn analyze_search(
    documents: &[SearchDocumentSample],
) -> Result<AdvancedModelAnalysis, DatabaseError> {
    ensure_analysis_count(documents.len())?;
    let mut ids = std::collections::HashSet::new();
    let mut fields: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut warnings = Vec::new();
    let mut text_fields = std::collections::BTreeSet::new();
    for d in documents {
        if d.id.is_empty() || d.id.len() > 512 {
            return Err(DatabaseError::Invalid(
                "search document id is invalid".into(),
            ));
        }
        if !ids.insert(d.id.clone()) {
            warnings.push(format!("duplicate search document id: {}", d.id));
        }
        if d.fields.len() > 1024 {
            return Err(DatabaseError::Invalid(
                "search document exceeds 1,024 fields".into(),
            ));
        }
        for (k, v) in &d.fields {
            if k.is_empty() || k.len() > 512 || k.chars().any(char::is_control) {
                return Err(DatabaseError::Invalid(
                    "search field name is invalid".into(),
                ));
            }
            let kind = value_kind(v);
            *fields
                .entry(k.clone())
                .or_default()
                .entry(kind.into())
                .or_default() += 1;
            if matches!(v, Value::String(_)) {
                text_fields.insert(k.clone());
            }
        }
    }
    let mut metrics = BTreeMap::new();
    metrics.insert("unique_documents".into(), Value::from(ids.len() as u64));
    metrics.insert(
        "text_fields".into(),
        serde_json::to_value(text_fields).unwrap_or(Value::Null),
    );
    metrics.insert("field_count".into(), Value::from(fields.len() as u64));
    Ok(AdvancedModelAnalysis {
        model: DataModel::Search,
        records: documents.len(),
        fields,
        metrics,
        warnings,
    })
}
fn analyze_time_series(
    points: &[TimeSeriesPointSample],
) -> Result<AdvancedModelAnalysis, DatabaseError> {
    ensure_analysis_count(points.len())?;
    let mut series: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    let mut tag_keys = std::collections::BTreeSet::new();
    let mut min_ts = i64::MAX;
    let mut max_ts = i64::MIN;
    let mut min_value = f64::INFINITY;
    let mut max_value = f64::NEG_INFINITY;
    for p in points {
        if p.series.is_empty() || p.series.len() > 512 || p.series.chars().any(char::is_control) {
            return Err(DatabaseError::Invalid("time-series name is invalid".into()));
        }
        if !p.value.is_finite() {
            return Err(DatabaseError::Invalid(
                "time-series value must be finite".into(),
            ));
        }
        if p.tags.len() > 128
            || p.tags.iter().any(|(k, v)| {
                k.is_empty()
                    || k.len() > 256
                    || v.len() > 1024
                    || k.chars().any(char::is_control)
                    || v.chars().any(char::is_control)
            })
        {
            return Err(DatabaseError::Invalid(
                "time-series tags are invalid".into(),
            ));
        }
        series
            .entry(p.series.clone())
            .or_default()
            .push(p.timestamp_unix_ms);
        tag_keys.extend(p.tags.keys().cloned());
        min_ts = min_ts.min(p.timestamp_unix_ms);
        max_ts = max_ts.max(p.timestamp_unix_ms);
        min_value = min_value.min(p.value);
        max_value = max_value.max(p.value);
    }
    let mut warnings = Vec::new();
    for (name, timestamps) in &series {
        if timestamps.windows(2).any(|w| w[1] < w[0]) {
            warnings.push(format!("series {name} contains out-of-order timestamps"));
        }
    }
    if points.is_empty() {
        min_ts = 0;
        max_ts = 0;
        min_value = 0.0;
        max_value = 0.0;
    }
    let mut metrics = BTreeMap::new();
    metrics.insert("series_count".into(), Value::from(series.len() as u64));
    metrics.insert("min_timestamp_unix_ms".into(), Value::from(min_ts));
    metrics.insert("max_timestamp_unix_ms".into(), Value::from(max_ts));
    metrics.insert("min_value".into(), serde_json::json!(min_value));
    metrics.insert("max_value".into(), serde_json::json!(max_value));
    metrics.insert(
        "tag_keys".into(),
        serde_json::to_value(tag_keys).unwrap_or(Value::Null),
    );
    Ok(AdvancedModelAnalysis {
        model: DataModel::TimeSeries,
        records: points.len(),
        fields: BTreeMap::new(),
        metrics,
        warnings,
    })
}
fn analyze_column(rows: &[ColumnRowSample]) -> Result<AdvancedModelAnalysis, DatabaseError> {
    ensure_analysis_count(rows.len())?;
    let mut partitions: BTreeMap<String, u64> = BTreeMap::new();
    let mut fields: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut seen = std::collections::HashSet::new();
    let mut warnings = Vec::new();
    for row in rows {
        if row.partition_key.is_empty()
            || row.partition_key.len() > 2048
            || row.partition_key.chars().any(char::is_control)
        {
            return Err(DatabaseError::Invalid(
                "column partition key is invalid".into(),
            ));
        }
        *partitions.entry(row.partition_key.clone()).or_default() += 1;
        let identity = (
            row.partition_key.clone(),
            row.clustering_key.clone().unwrap_or_default(),
        );
        if !seen.insert(identity) {
            warnings.push(format!(
                "duplicate partition/clustering sample for {}",
                row.partition_key
            ));
        }
        if row.columns.len() > 1024 {
            return Err(DatabaseError::Invalid(
                "column row exceeds 1,024 columns".into(),
            ));
        }
        for (k, v) in &row.columns {
            if k.is_empty() || k.len() > 512 || k.chars().any(char::is_control) {
                return Err(DatabaseError::Invalid("column name is invalid".into()));
            }
            *fields
                .entry(k.clone())
                .or_default()
                .entry(value_kind(v).into())
                .or_default() += 1;
        }
    }
    let max_partition = partitions.values().copied().max().unwrap_or(0);
    if rows.len() >= 100 && max_partition as usize > rows.len() / 2 {
        warnings.push("sample is dominated by a single partition key".into());
    }
    let mut metrics = BTreeMap::new();
    metrics.insert(
        "partition_count".into(),
        Value::from(partitions.len() as u64),
    );
    metrics.insert(
        "largest_partition_sample".into(),
        Value::from(max_partition),
    );
    metrics.insert("column_count".into(), Value::from(fields.len() as u64));
    Ok(AdvancedModelAnalysis {
        model: DataModel::Column,
        records: rows.len(),
        fields,
        metrics,
        warnings,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteDatabaseCapability {
    pub engine: String,
    pub inspect: bool,
    pub browse: bool,
    pub query: bool,
    pub write: bool,
    pub indexes: bool,
    pub relations: bool,
    pub statistics: bool,
    pub durable_jobs: bool,
    pub cancellable_jobs: bool,
    pub transactions: bool,
    pub live_stream_read: bool,
    pub plaintext_loopback: bool,
    pub verified_tls_remote: bool,
    pub notes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteDatabaseConformanceReport {
    pub valid: bool,
    pub engines: Vec<RemoteDatabaseCapability>,
    pub issues: Vec<String>,
}
pub fn remote_database_capabilities() -> Vec<RemoteDatabaseCapability> {
    vec![
        RemoteDatabaseCapability {
            engine: "postgresql".into(),
            inspect: true,
            browse: true,
            query: true,
            write: true,
            indexes: true,
            relations: true,
            statistics: true,
            durable_jobs: true,
            cancellable_jobs: true,
            transactions: true,
            live_stream_read: true,
            plaintext_loopback: true,
            verified_tls_remote: true,
            notes: vec![
                "native plaintext is exact-loopback only; verified TLS is available for remote reads".into(),
                "structured writes remain DatabaseWrite-gated and native loopback-only".into(),
            ],
        },
        RemoteDatabaseCapability {
            engine: "mysql".into(),
            inspect: true,
            browse: true,
            query: true,
            write: true,
            indexes: true,
            relations: true,
            statistics: true,
            durable_jobs: true,
            cancellable_jobs: true,
            transactions: false,
            live_stream_read: true,
            plaintext_loopback: true,
            verified_tls_remote: true,
            notes: vec![
                "native plaintext is exact-loopback only; verified TLS is available for remote reads".into(),
                "structured writes remain DatabaseWrite-gated and native loopback-only".into(),
            ],
        },
        RemoteDatabaseCapability {
            engine: "mariadb".into(),
            inspect: true,
            browse: false,
            query: true,
            write: false,
            indexes: false,
            relations: false,
            statistics: false,
            durable_jobs: true,
            cancellable_jobs: true,
            transactions: false,
            live_stream_read: true,
            plaintext_loopback: true,
            verified_tls_remote: true,
            notes: vec![
                "external client read/query beta only; remote use forces CA and server-certificate verification".into(),
            ],
        },
        RemoteDatabaseCapability {
            engine: "mongodb".into(),
            inspect: true,
            browse: true,
            query: false,
            write: true,
            indexes: true,
            relations: false,
            statistics: true,
            durable_jobs: false,
            cancellable_jobs: false,
            transactions: false,
            live_stream_read: false,
            plaintext_loopback: true,
            verified_tls_remote: true,
            notes: vec![
                "structured document browse/filter and CRUD; arbitrary JavaScript/query execution is unavailable".into(),
                "remote native SRV and external client paths reject insecure TLS overrides".into(),
            ],
        },
        RemoteDatabaseCapability {
            engine: "redis".into(),
            inspect: true,
            browse: false,
            query: false,
            write: true,
            indexes: false,
            relations: false,
            statistics: false,
            durable_jobs: false,
            cancellable_jobs: false,
            transactions: false,
            live_stream_read: false,
            plaintext_loopback: true,
            verified_tls_remote: true,
            notes: vec![
                "typed key inspection/get/set/delete baseline; arbitrary Redis command execution is unavailable".into(),
                "remote TLS uses trusted certificate verification; insecure mode is rejected".into(),
            ],
        },
    ]
}
pub fn validate_remote_database_capabilities() -> RemoteDatabaseConformanceReport {
    let engines = remote_database_capabilities();
    let mut issues = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for c in &engines {
        if !seen.insert(c.engine.clone()) {
            issues.push(format!("duplicate remote database engine: {}", c.engine));
        }
        if !c.plaintext_loopback {
            issues.push(format!(
                "{} does not declare exact-loopback plaintext policy",
                c.engine
            ));
        }
        if !c.verified_tls_remote {
            issues.push(format!(
                "{} does not declare verified remote TLS policy",
                c.engine
            ));
        }
        if c.write && !c.inspect {
            issues.push(format!("{} exposes writes without inspection", c.engine));
        }
        if c.transactions && !c.query {
            issues.push(format!("{} exposes transactions without query", c.engine));
        }
        if c.cancellable_jobs && !c.durable_jobs && c.engine != "postgresql" {
            issues.push(format!(
                "{} exposes cancellable jobs without job lifecycle",
                c.engine
            ));
        }
        if c.live_stream_read && !c.query && !c.browse {
            issues.push(format!(
                "{} exposes live read stream without query/browse",
                c.engine
            ));
        }
    }
    RemoteDatabaseConformanceReport {
        valid: issues.is_empty(),
        engines,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boolean_becomes_toggle() {
        let e = EntityMeta {
            name: "users".into(),
            display_name: "Users".into(),
            fields: vec![FieldMeta {
                name: "active".into(),
                field_type: FieldType::Boolean,
                nullable: false,
                primary: false,
                generated: false,
                enum_values: vec![],
                relation_target: None,
                metadata: Value::Null,
            }],
            metadata: Value::Null,
        };
        let ui = generate_ui_schema(&e, &CapabilitySet::default());
        assert_eq!(ui.fields[0].control, UiControl::Toggle);
    }
    #[test]
    fn document_analysis_detects_types() {
        let r = analyze_advanced_model(&AdvancedModelRequest::Document {
            documents: vec![
                serde_json::json!({"id":1,"name":"a"}),
                serde_json::json!({"id":2,"name":null}),
            ],
        })
        .unwrap();
        assert_eq!(r.records, 2);
        assert_eq!(r.fields["id"]["integer"], 2);
    }
    #[test]
    fn vector_model_has_similarity_search() {
        assert!(workspace_for_model(DataModel::Vector).contains(&"similarity_search"));
    }
    #[test]
    fn ui_actions_follow_capabilities() {
        let caps = CapabilitySet {
            insert: true,
            export: true,
            ..Default::default()
        };
        let e = EntityMeta {
            name: "x".into(),
            display_name: "X".into(),
            fields: vec![],
            metadata: Value::Null,
        };
        let ui = generate_ui_schema(&e, &caps);
        assert_eq!(ui.actions, vec!["insert", "export"]);
    }
    #[test]
    fn conformance_rejects_query_without_connect() {
        let d = ProviderDescriptor {
            sdk_version: DATABASE_SDK_VERSION,
            id: "demo".into(),
            model: DataModel::Relational,
            capabilities: CapabilitySet {
                query: true,
                ..CapabilitySet::default()
            },
        };
        let r = validate_provider_descriptor(&d);
        assert!(!r.passed);
        assert!(r.issues.iter().any(|i| i.capability == "query"));
    }
    #[test]
    fn complete_descriptor_passes() {
        let d = ProviderDescriptor {
            sdk_version: DATABASE_SDK_VERSION,
            id: "demo.sql".into(),
            model: DataModel::Relational,
            capabilities: CapabilitySet {
                connect: true,
                introspect: true,
                query: true,
                browse: true,
                ..CapabilitySet::default()
            },
        };
        assert!(validate_provider_descriptor(&d).passed);
    }
    #[test]
    fn advanced_models_cover_all_non_relational_families() {
        let search = analyze_advanced_model(&AdvancedModelRequest::Search {
            documents: vec![SearchDocumentSample {
                id: "1".into(),
                fields: BTreeMap::from([("title".into(), Value::String("x".into()))]),
            }],
        })
        .unwrap();
        assert_eq!(search.model, DataModel::Search);
        let ts = analyze_advanced_model(&AdvancedModelRequest::TimeSeries {
            points: vec![TimeSeriesPointSample {
                series: "cpu".into(),
                timestamp_unix_ms: 1,
                value: 1.0,
                tags: BTreeMap::new(),
            }],
        })
        .unwrap();
        assert_eq!(ts.model, DataModel::TimeSeries);
        let col = analyze_advanced_model(&AdvancedModelRequest::Column {
            rows: vec![ColumnRowSample {
                partition_key: "a".into(),
                clustering_key: None,
                columns: BTreeMap::new(),
            }],
        })
        .unwrap();
        assert_eq!(col.model, DataModel::Column);
    }
}
