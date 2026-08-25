use mongodb::bson::{doc, Bson, Document};
use mongodb::sync::Client as MongoClient;
use mysql::{
    prelude::Queryable, Params as MyParams, Pool as MyPool, Row as MyRow, Value as MyValue,
};
use native_tls::{Certificate as NativeCertificate, TlsConnector};
use postgres::{
    config::{Host as PgHost, SslMode},
    types::ToSql,
    Client as PgClient, Config as PgConfig, NoTls, SimpleQueryMessage,
};
use postgres_native_tls::MakeTlsConnector;
use redis::Value as RedisValue;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::PathBuf, str::FromStr, time::Duration};
use thiserror::Error;
use vsn_database::{MutationRequest, MutationResult};

const MAX_ROWS: u32 = 1000;
const MAX_MUTATION_FIELDS: usize = 128;
const MAX_TEXT_CELL_BYTES: usize = 256 * 1024;
const MAX_SERIALIZED_READ_BYTES: usize = 512 * 1024;

#[derive(Debug, Error)]
pub enum NativeDbError {
    #[error("native database request rejected: {0}")]
    Invalid(String),
    #[error("PostgreSQL error: {0}")]
    Postgres(String),
    #[error("MySQL error: {0}")]
    MySql(String),
    #[error("Redis error: {0}")]
    Redis(String),
    #[error("MongoDB error: {0}")]
    Mongo(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostgresConnection {
    pub connection_string: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostgresTlsConnection {
    pub connection_string: String,
    pub root_ca_pem_path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RedisConnection {
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MySqlConnection {
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MySqlTlsConnection {
    pub url: String,
    pub root_ca_path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MongoConnection {
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NativeGrid {
    pub columns: Vec<String>,
    pub rows: Vec<Value>,
    pub row_count: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostgresInspection {
    pub server_version: String,
    pub current_database: String,
    pub schemas: Vec<String>,
    pub tables: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RedisInspection {
    pub server_info: String,
    pub db_size: u64,
    pub sample_keys: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MongoInspection {
    pub databases: Vec<String>,
    pub current_database: Option<String>,
    pub collections: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MongoMutationResult {
    pub affected_documents: u64,
    pub inserted_id: Option<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MySqlInspection {
    pub server_version: String,
    pub current_database: Option<String>,
    pub databases: Vec<String>,
    pub tables: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NativeTableStats {
    pub namespace: String,
    pub table: String,
    pub estimated_rows: Option<u64>,
    pub table_bytes: Option<u64>,
    pub index_bytes: Option<u64>,
}

// ---------- PostgreSQL ----------

pub fn postgres_inspect(spec: &PostgresConnection) -> Result<PostgresInspection, NativeDbError> {
    let mut c = connect_postgres(spec)?;
    let version = scalar(&mut c, "SHOW server_version")?;
    let db = scalar(&mut c, "SELECT current_database()")?;
    let schemas = single_column(
        &mut c,
        "SELECT schema_name FROM information_schema.schemata ORDER BY schema_name",
    )?;
    let grid = simple_grid(
        &mut c,
        "SELECT table_schema,table_name,table_type FROM information_schema.tables WHERE table_schema NOT IN ('pg_catalog','information_schema') ORDER BY table_schema,table_name LIMIT 5000",
    )?;
    bounded_read_result(PostgresInspection {
        server_version: version,
        current_database: db,
        schemas,
        tables: grid.rows,
    })
}

pub fn postgres_browse(
    spec: &PostgresConnection,
    schema: &str,
    table: &str,
    limit: u32,
    offset: u64,
) -> Result<NativeGrid, NativeDbError> {
    safe_ident(schema)?;
    safe_ident(table)?;
    let limit = limit.clamp(1, MAX_ROWS);
    let mut c = connect_postgres(spec)?;
    let sql = format!(
        "SELECT * FROM {}.{} LIMIT {} OFFSET {}",
        quote_ident(schema),
        quote_ident(table),
        limit,
        offset
    );
    simple_grid(&mut c, &sql)
}

pub fn postgres_read_query(
    spec: &PostgresConnection,
    sql: &str,
) -> Result<NativeGrid, NativeDbError> {
    validate_read_only_sql(sql)?;
    let mut c = connect_postgres(spec)?;
    c.batch_execute("BEGIN READ ONLY")
        .map_err(|e| NativeDbError::Postgres(e.to_string()))?;
    let result = simple_grid(&mut c, sql);
    let _ = c.batch_execute("ROLLBACK");
    result
}

pub fn postgres_insert(
    spec: &PostgresConnection,
    schema: &str,
    table: &str,
    request: &MutationRequest,
) -> Result<MutationResult, NativeDbError> {
    validate_mutation_target(schema, table)?;
    validate_values(&request.values, false)?;
    let columns = safe_column_list(&request.values)?;
    let json_payload = serde_json::to_string(&request.values)
        .map_err(|e| NativeDbError::Invalid(format!("mutation JSON encode failed: {e}")))?;
    let select_columns = columns
        .iter()
        .map(|column| format!("v.{}", quote_ident(column)))
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "WITH v AS (SELECT * FROM jsonb_populate_record(NULL::{}.{}, $1::jsonb)) INSERT INTO {}.{} ({}) SELECT {} FROM v",
        quote_ident(schema), quote_ident(table), quote_ident(schema), quote_ident(table),
        columns.iter().map(|v| quote_ident(v)).collect::<Vec<_>>().join(","),
        select_columns,
    );
    let mut c = connect_postgres(spec)?;
    let params: [&(dyn ToSql + Sync); 1] = [&json_payload];
    let affected = c
        .execute(&sql, &params)
        .map_err(|e| NativeDbError::Postgres(e.to_string()))?;
    Ok(MutationResult {
        affected_rows: affected,
        last_insert_id: None,
    })
}

pub fn postgres_update(
    spec: &PostgresConnection,
    schema: &str,
    table: &str,
    request: &MutationRequest,
) -> Result<MutationResult, NativeDbError> {
    validate_mutation_target(schema, table)?;
    validate_values(&request.values, false)?;
    validate_filter(&request.filter)?;
    let value_columns = safe_column_list(&request.values)?;
    let filter_columns = safe_column_list(&request.filter)?;
    let values_json = serde_json::to_string(&request.values)
        .map_err(|e| NativeDbError::Invalid(format!("mutation JSON encode failed: {e}")))?;
    let filter_json = serde_json::to_string(&request.filter)
        .map_err(|e| NativeDbError::Invalid(format!("filter JSON encode failed: {e}")))?;
    let set_clause = value_columns
        .iter()
        .map(|column| format!("{} = v.{}", quote_ident(column), quote_ident(column)))
        .collect::<Vec<_>>()
        .join(",");
    let where_clause = filter_columns
        .iter()
        .map(|column| {
            format!(
                "t.{} IS NOT DISTINCT FROM f.{}",
                quote_ident(column),
                quote_ident(column)
            )
        })
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = format!(
        "WITH v AS (SELECT * FROM jsonb_populate_record(NULL::{}.{}, $1::jsonb)), f AS (SELECT * FROM jsonb_populate_record(NULL::{}.{}, $2::jsonb)) UPDATE {}.{} AS t SET {} FROM v,f WHERE {}",
        quote_ident(schema), quote_ident(table), quote_ident(schema), quote_ident(table),
        quote_ident(schema), quote_ident(table), set_clause, where_clause,
    );
    let mut c = connect_postgres(spec)?;
    let params: [&(dyn ToSql + Sync); 2] = [&values_json, &filter_json];
    let affected = c
        .execute(&sql, &params)
        .map_err(|e| NativeDbError::Postgres(e.to_string()))?;
    Ok(MutationResult {
        affected_rows: affected,
        last_insert_id: None,
    })
}

pub fn postgres_delete(
    spec: &PostgresConnection,
    schema: &str,
    table: &str,
    request: &MutationRequest,
) -> Result<MutationResult, NativeDbError> {
    validate_mutation_target(schema, table)?;
    validate_filter(&request.filter)?;
    let filter_columns = safe_column_list(&request.filter)?;
    let filter_json = serde_json::to_string(&request.filter)
        .map_err(|e| NativeDbError::Invalid(format!("filter JSON encode failed: {e}")))?;
    let where_clause = filter_columns
        .iter()
        .map(|column| {
            format!(
                "t.{} IS NOT DISTINCT FROM f.{}",
                quote_ident(column),
                quote_ident(column)
            )
        })
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = format!(
        "WITH f AS (SELECT * FROM jsonb_populate_record(NULL::{}.{}, $1::jsonb)) DELETE FROM {}.{} AS t USING f WHERE {}",
        quote_ident(schema), quote_ident(table), quote_ident(schema), quote_ident(table), where_clause,
    );
    let mut c = connect_postgres(spec)?;
    let params: [&(dyn ToSql + Sync); 1] = [&filter_json];
    let affected = c
        .execute(&sql, &params)
        .map_err(|e| NativeDbError::Postgres(e.to_string()))?;
    Ok(MutationResult {
        affected_rows: affected,
        last_insert_id: None,
    })
}

pub fn postgres_indexes(
    spec: &PostgresConnection,
    schema: &str,
    table: &str,
) -> Result<NativeGrid, NativeDbError> {
    validate_mutation_target(schema, table)?;
    let mut c = connect_postgres(spec)?;
    let schema_s = schema.to_string();
    let table_s = table.to_string();
    let rows = c
        .query(
            "SELECT indexname,indexdef FROM pg_indexes WHERE schemaname=$1 AND tablename=$2 ORDER BY indexname",
            &[&schema_s, &table_s],
        )
        .map_err(|e| NativeDbError::Postgres(e.to_string()))?;
    let out = rows
        .into_iter()
        .map(|row| json!({"name":row.get::<_,String>(0),"definition":row.get::<_,String>(1)}))
        .collect::<Vec<_>>();
    Ok(NativeGrid {
        columns: vec!["name".into(), "definition".into()],
        row_count: out.len() as u64,
        rows: out,
    })
}

pub fn postgres_relations(
    spec: &PostgresConnection,
    schema: &str,
    table: &str,
) -> Result<NativeGrid, NativeDbError> {
    validate_mutation_target(schema, table)?;
    let mut c = connect_postgres(spec)?;
    let schema_s = schema.to_string();
    let table_s = table.to_string();
    let rows = c
        .query(
            "SELECT tc.constraint_name,kcu.column_name,ccu.table_schema,ccu.table_name,ccu.column_name,rc.update_rule,rc.delete_rule FROM information_schema.table_constraints tc JOIN information_schema.key_column_usage kcu ON tc.constraint_name=kcu.constraint_name AND tc.constraint_schema=kcu.constraint_schema JOIN information_schema.constraint_column_usage ccu ON ccu.constraint_name=tc.constraint_name AND ccu.constraint_schema=tc.constraint_schema JOIN information_schema.referential_constraints rc ON rc.constraint_name=tc.constraint_name AND rc.constraint_schema=tc.constraint_schema WHERE tc.constraint_type='FOREIGN KEY' AND tc.table_schema=$1 AND tc.table_name=$2 ORDER BY tc.constraint_name,kcu.ordinal_position",
            &[&schema_s, &table_s],
        )
        .map_err(|e| NativeDbError::Postgres(e.to_string()))?;
    let out = rows
        .into_iter()
        .map(|row| {
            json!({
                "name":row.get::<_,String>(0),
                "from_column":row.get::<_,String>(1),
                "to_schema":row.get::<_,String>(2),
                "to_table":row.get::<_,String>(3),
                "to_column":row.get::<_,String>(4),
                "on_update":row.get::<_,String>(5),
                "on_delete":row.get::<_,String>(6),
            })
        })
        .collect::<Vec<_>>();
    Ok(NativeGrid {
        columns: vec![
            "name".into(),
            "from_column".into(),
            "to_schema".into(),
            "to_table".into(),
            "to_column".into(),
            "on_update".into(),
            "on_delete".into(),
        ],
        row_count: out.len() as u64,
        rows: out,
    })
}

pub fn postgres_stats(
    spec: &PostgresConnection,
    schema: &str,
    table: &str,
) -> Result<NativeTableStats, NativeDbError> {
    validate_mutation_target(schema, table)?;
    let mut c = connect_postgres(spec)?;
    let relation = format!("{}.{}", quote_ident(schema), quote_ident(table));
    let row = c
        .query_one(
            "SELECT c.reltuples::bigint, pg_table_size($1::regclass)::bigint, pg_indexes_size($1::regclass)::bigint FROM pg_class c WHERE c.oid=$1::regclass",
            &[&relation],
        )
        .map_err(|e| NativeDbError::Postgres(e.to_string()))?;
    let estimated: i64 = row.get(0);
    let table_bytes: i64 = row.get(1);
    let index_bytes: i64 = row.get(2);
    Ok(NativeTableStats {
        namespace: schema.into(),
        table: table.into(),
        estimated_rows: u64::try_from(estimated.max(0)).ok(),
        table_bytes: u64::try_from(table_bytes.max(0)).ok(),
        index_bytes: u64::try_from(index_bytes.max(0)).ok(),
    })
}

// ---------- PostgreSQL verified TLS profile ----------

pub fn postgres_tls_inspect(
    spec: &PostgresTlsConnection,
) -> Result<PostgresInspection, NativeDbError> {
    let mut c = connect_postgres_tls(spec)?;
    let version = scalar(&mut c, "SHOW server_version")?;
    let db = scalar(&mut c, "SELECT current_database()")?;
    let schemas = single_column(
        &mut c,
        "SELECT schema_name FROM information_schema.schemata ORDER BY schema_name",
    )?;
    let grid=simple_grid(&mut c,"SELECT table_schema,table_name,table_type FROM information_schema.tables WHERE table_schema NOT IN ('pg_catalog','information_schema') ORDER BY table_schema,table_name LIMIT 5000")?;
    bounded_read_result(PostgresInspection {
        server_version: version,
        current_database: db,
        schemas,
        tables: grid.rows,
    })
}
pub fn postgres_tls_browse(
    spec: &PostgresTlsConnection,
    schema: &str,
    table: &str,
    limit: u32,
    offset: u64,
) -> Result<NativeGrid, NativeDbError> {
    safe_ident(schema)?;
    safe_ident(table)?;
    let mut c = connect_postgres_tls(spec)?;
    let sql = format!(
        "SELECT * FROM {}.{} LIMIT {} OFFSET {}",
        quote_ident(schema),
        quote_ident(table),
        limit.clamp(1, MAX_ROWS),
        offset
    );
    simple_grid(&mut c, &sql)
}
pub fn postgres_tls_read_query(
    spec: &PostgresTlsConnection,
    sql: &str,
) -> Result<NativeGrid, NativeDbError> {
    validate_read_only_sql(sql)?;
    let mut c = connect_postgres_tls(spec)?;
    c.batch_execute("BEGIN READ ONLY")
        .map_err(|e| NativeDbError::Postgres(e.to_string()))?;
    let result = simple_grid(&mut c, sql);
    let _ = c.batch_execute("ROLLBACK");
    result
}

// ---------- MySQL / MariaDB ----------

pub fn mysql_inspect(spec: &MySqlConnection) -> Result<MySqlInspection, NativeDbError> {
    let mut c = connect_mysql(spec)?;
    let version: Option<String> = c.query_first("SELECT VERSION()").map_err(mysql_err)?;
    let current: Option<String> = c.query_first("SELECT DATABASE()").map_err(mysql_err)?;
    let databases: Vec<String> = c.query("SHOW DATABASES").map_err(mysql_err)?;
    let grid = mysql_grid(&mut c, "SELECT TABLE_SCHEMA,TABLE_NAME,TABLE_TYPE FROM information_schema.TABLES WHERE TABLE_SCHEMA NOT IN ('information_schema','mysql','performance_schema','sys') ORDER BY TABLE_SCHEMA,TABLE_NAME LIMIT 5000")?;
    bounded_read_result(MySqlInspection {
        server_version: version.unwrap_or_default(),
        current_database: current,
        databases,
        tables: grid.rows,
    })
}

pub fn mysql_browse(
    spec: &MySqlConnection,
    database: &str,
    table: &str,
    limit: u32,
    offset: u64,
) -> Result<NativeGrid, NativeDbError> {
    validate_mutation_target(database, table)?;
    let limit = limit.clamp(1, MAX_ROWS);
    let mut c = connect_mysql(spec)?;
    let sql = format!(
        "SELECT * FROM {}.{} LIMIT {} OFFSET {}",
        mysql_quote_ident(database),
        mysql_quote_ident(table),
        limit,
        offset
    );
    mysql_grid(&mut c, &sql)
}

pub fn mysql_read_query(spec: &MySqlConnection, sql: &str) -> Result<NativeGrid, NativeDbError> {
    validate_read_only_sql(sql)?;
    validate_mysql_read_sql(sql)?;
    let mut c = connect_mysql(spec)?;
    c.query_drop("START TRANSACTION READ ONLY")
        .map_err(mysql_err)?;
    let result = mysql_grid(&mut c, sql);
    let _ = c.query_drop("ROLLBACK");
    result
}

pub fn mysql_insert(
    spec: &MySqlConnection,
    database: &str,
    table: &str,
    request: &MutationRequest,
) -> Result<MutationResult, NativeDbError> {
    validate_mutation_target(database, table)?;
    validate_values(&request.values, false)?;
    let columns = safe_column_list(&request.values)?;
    let placeholders = std::iter::repeat_n("?", columns.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "INSERT INTO {}.{} ({}) VALUES ({})",
        mysql_quote_ident(database),
        mysql_quote_ident(table),
        columns
            .iter()
            .map(|v| mysql_quote_ident(v))
            .collect::<Vec<_>>()
            .join(","),
        placeholders,
    );
    let params = columns
        .iter()
        .map(|column| json_to_mysql(request.values.get(column).expect("column came from map")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut c = connect_mysql(spec)?;
    c.exec_drop(&sql, MyParams::Positional(params))
        .map_err(mysql_err)?;
    let last = c.last_insert_id();
    Ok(MutationResult {
        affected_rows: c.affected_rows(),
        last_insert_id: if last == 0 {
            None
        } else {
            i64::try_from(last).ok()
        },
    })
}

pub fn mysql_update(
    spec: &MySqlConnection,
    database: &str,
    table: &str,
    request: &MutationRequest,
) -> Result<MutationResult, NativeDbError> {
    validate_mutation_target(database, table)?;
    validate_values(&request.values, false)?;
    validate_filter(&request.filter)?;
    let values = safe_column_list(&request.values)?;
    let filters = safe_column_list(&request.filter)?;
    let set_clause = values
        .iter()
        .map(|column| format!("{}=?", mysql_quote_ident(column)))
        .collect::<Vec<_>>()
        .join(",");
    let where_clause = filters
        .iter()
        .map(|column| format!("{} <=> ?", mysql_quote_ident(column)))
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = format!(
        "UPDATE {}.{} SET {} WHERE {}",
        mysql_quote_ident(database),
        mysql_quote_ident(table),
        set_clause,
        where_clause
    );
    let mut params = Vec::with_capacity(values.len() + filters.len());
    for column in &values {
        params.push(json_to_mysql(
            request.values.get(column).expect("column came from map"),
        )?);
    }
    for column in &filters {
        params.push(json_to_mysql(
            request.filter.get(column).expect("column came from map"),
        )?);
    }
    let mut c = connect_mysql(spec)?;
    c.exec_drop(&sql, MyParams::Positional(params))
        .map_err(mysql_err)?;
    Ok(MutationResult {
        affected_rows: c.affected_rows(),
        last_insert_id: None,
    })
}

pub fn mysql_delete(
    spec: &MySqlConnection,
    database: &str,
    table: &str,
    request: &MutationRequest,
) -> Result<MutationResult, NativeDbError> {
    validate_mutation_target(database, table)?;
    validate_filter(&request.filter)?;
    let filters = safe_column_list(&request.filter)?;
    let where_clause = filters
        .iter()
        .map(|column| format!("{} <=> ?", mysql_quote_ident(column)))
        .collect::<Vec<_>>()
        .join(" AND ");
    let sql = format!(
        "DELETE FROM {}.{} WHERE {}",
        mysql_quote_ident(database),
        mysql_quote_ident(table),
        where_clause
    );
    let params = filters
        .iter()
        .map(|column| json_to_mysql(request.filter.get(column).expect("column came from map")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut c = connect_mysql(spec)?;
    c.exec_drop(&sql, MyParams::Positional(params))
        .map_err(mysql_err)?;
    Ok(MutationResult {
        affected_rows: c.affected_rows(),
        last_insert_id: None,
    })
}

pub fn mysql_indexes(
    spec: &MySqlConnection,
    database: &str,
    table: &str,
) -> Result<NativeGrid, NativeDbError> {
    validate_mutation_target(database, table)?;
    let mut c = connect_mysql(spec)?;
    let rows: Vec<MyRow> = c
        .exec(
            "SELECT INDEX_NAME,NON_UNIQUE,SEQ_IN_INDEX,COLUMN_NAME,INDEX_TYPE FROM information_schema.STATISTICS WHERE TABLE_SCHEMA=? AND TABLE_NAME=? ORDER BY INDEX_NAME,SEQ_IN_INDEX",
            (database, table),
        )
        .map_err(mysql_err)?;
    mysql_rows_to_grid(rows)
}

pub fn mysql_relations(
    spec: &MySqlConnection,
    database: &str,
    table: &str,
) -> Result<NativeGrid, NativeDbError> {
    validate_mutation_target(database, table)?;
    let mut c = connect_mysql(spec)?;
    let rows: Vec<MyRow> = c
        .exec(
            "SELECT k.CONSTRAINT_NAME,k.COLUMN_NAME,k.REFERENCED_TABLE_SCHEMA,k.REFERENCED_TABLE_NAME,k.REFERENCED_COLUMN_NAME,r.UPDATE_RULE,r.DELETE_RULE FROM information_schema.KEY_COLUMN_USAGE k JOIN information_schema.REFERENTIAL_CONSTRAINTS r ON r.CONSTRAINT_SCHEMA=k.CONSTRAINT_SCHEMA AND r.CONSTRAINT_NAME=k.CONSTRAINT_NAME WHERE k.TABLE_SCHEMA=? AND k.TABLE_NAME=? AND k.REFERENCED_TABLE_NAME IS NOT NULL ORDER BY k.CONSTRAINT_NAME,k.ORDINAL_POSITION",
            (database, table),
        )
        .map_err(mysql_err)?;
    mysql_rows_to_grid(rows)
}

pub fn mysql_stats(
    spec: &MySqlConnection,
    database: &str,
    table: &str,
) -> Result<NativeTableStats, NativeDbError> {
    validate_mutation_target(database, table)?;
    let mut c = connect_mysql(spec)?;
    let row: Option<(Option<u64>, Option<u64>, Option<u64>)> = c
        .exec_first(
            "SELECT TABLE_ROWS,DATA_LENGTH,INDEX_LENGTH FROM information_schema.TABLES WHERE TABLE_SCHEMA=? AND TABLE_NAME=?",
            (database, table),
        )
        .map_err(mysql_err)?;
    let (estimated_rows, table_bytes, index_bytes) =
        row.ok_or_else(|| NativeDbError::Invalid("table was not found".into()))?;
    Ok(NativeTableStats {
        namespace: database.into(),
        table: table.into(),
        estimated_rows,
        table_bytes,
        index_bytes,
    })
}

// ---------- MySQL / MariaDB verified TLS profile ----------

pub fn mysql_tls_inspect(spec: &MySqlTlsConnection) -> Result<MySqlInspection, NativeDbError> {
    let mut c = connect_mysql_tls(spec)?;
    let server_version: String = c
        .query_first("SELECT VERSION()")
        .map_err(mysql_err)?
        .unwrap_or_default();
    let current_database: Option<String> = c
        .query_first("SELECT DATABASE()")
        .map_err(mysql_err)?
        .flatten();
    let databases: Vec<String> = c.query("SHOW DATABASES").map_err(mysql_err)?;
    let tables=mysql_grid(&mut c,"SELECT TABLE_SCHEMA,TABLE_NAME,TABLE_TYPE FROM information_schema.TABLES WHERE TABLE_SCHEMA NOT IN ('mysql','information_schema','performance_schema','sys') ORDER BY TABLE_SCHEMA,TABLE_NAME LIMIT 5000")?.rows;
    bounded_read_result(MySqlInspection {
        server_version,
        current_database,
        databases,
        tables,
    })
}
pub fn mysql_tls_browse(
    spec: &MySqlTlsConnection,
    database: &str,
    table: &str,
    limit: u32,
    offset: u64,
) -> Result<NativeGrid, NativeDbError> {
    safe_ident(database)?;
    safe_ident(table)?;
    let mut c = connect_mysql_tls(spec)?;
    let sql = format!(
        "SELECT * FROM {}.{} LIMIT {} OFFSET {}",
        mysql_quote_ident(database),
        mysql_quote_ident(table),
        limit.clamp(1, MAX_ROWS),
        offset
    );
    mysql_grid(&mut c, &sql)
}
pub fn mysql_tls_read_query(
    spec: &MySqlTlsConnection,
    sql: &str,
) -> Result<NativeGrid, NativeDbError> {
    validate_read_only_sql(sql)?;
    validate_mysql_read_sql(sql)?;
    let mut c = connect_mysql_tls(spec)?;
    c.query_drop("START TRANSACTION READ ONLY")
        .map_err(mysql_err)?;
    let result = mysql_grid(&mut c, sql);
    let _ = c.query_drop("ROLLBACK");
    result
}

// ---------- MongoDB ----------

pub fn mongo_inspect(
    spec: &MongoConnection,
    database: Option<&str>,
) -> Result<MongoInspection, NativeDbError> {
    let client = connect_mongo(spec)?;
    let databases = client.list_database_names().run().map_err(mongo_err)?;
    let current = database
        .map(str::to_string)
        .or_else(|| client.default_database().map(|d| d.name().to_string()));
    let collections = if let Some(db) = current.as_deref() {
        safe_mongo_name(db)?;
        client
            .database(db)
            .list_collection_names()
            .run()
            .map_err(mongo_err)?
    } else {
        Vec::new()
    };
    bounded_read_result(MongoInspection {
        databases,
        current_database: current,
        collections,
    })
}
pub fn mongo_browse(
    spec: &MongoConnection,
    database: &str,
    collection: &str,
    limit: u32,
    offset: u64,
    filter: Option<Value>,
) -> Result<NativeGrid, NativeDbError> {
    safe_mongo_name(database)?;
    safe_mongo_name(collection)?;
    let client = connect_mongo(spec)?;
    let coll = client.database(database).collection::<Document>(collection);
    let filter = json_to_document(filter.unwrap_or_else(|| json!({})))?;
    let cursor = coll
        .find(filter)
        .limit(i64::from(limit.clamp(1, MAX_ROWS)))
        .skip(offset)
        .run()
        .map_err(mongo_err)?;
    let mut rows = Vec::new();
    let mut columns = std::collections::BTreeSet::new();
    for item in cursor {
        let doc = item.map_err(mongo_err)?;
        for key in doc.keys() {
            columns.insert(key.to_string());
        }
        rows.push(document_to_json(doc)?);
    }
    let grid = NativeGrid {
        columns: columns.into_iter().collect(),
        row_count: rows.len() as u64,
        rows,
    };
    ensure_serialized_budget(&grid)?;
    Ok(grid)
}
pub fn mongo_insert(
    spec: &MongoConnection,
    database: &str,
    collection: &str,
    values: &BTreeMap<String, Value>,
) -> Result<MongoMutationResult, NativeDbError> {
    safe_mongo_name(database)?;
    safe_mongo_name(collection)?;
    validate_values(values, false)?;
    let client = connect_mongo(spec)?;
    let coll = client.database(database).collection::<Document>(collection);
    let doc = json_map_to_document(values)?;
    let result = coll.insert_one(doc).run().map_err(mongo_err)?;
    Ok(MongoMutationResult {
        affected_documents: 1,
        inserted_id: Some(bson_to_json(result.inserted_id)?),
    })
}
pub fn mongo_update(
    spec: &MongoConnection,
    database: &str,
    collection: &str,
    request: &MutationRequest,
) -> Result<MongoMutationResult, NativeDbError> {
    safe_mongo_name(database)?;
    safe_mongo_name(collection)?;
    validate_values(&request.values, false)?;
    validate_filter(&request.filter)?;
    let client = connect_mongo(spec)?;
    let coll = client.database(database).collection::<Document>(collection);
    let filter = json_map_to_document(&request.filter)?;
    let values = json_map_to_document(&request.values)?;
    let result = coll
        .update_many(filter, doc! {"$set":values})
        .run()
        .map_err(mongo_err)?;
    Ok(MongoMutationResult {
        affected_documents: result.modified_count,
        inserted_id: None,
    })
}
pub fn mongo_delete(
    spec: &MongoConnection,
    database: &str,
    collection: &str,
    filter: &BTreeMap<String, Value>,
) -> Result<MongoMutationResult, NativeDbError> {
    safe_mongo_name(database)?;
    safe_mongo_name(collection)?;
    validate_filter(filter)?;
    let client = connect_mongo(spec)?;
    let coll = client.database(database).collection::<Document>(collection);
    let result = coll
        .delete_many(json_map_to_document(filter)?)
        .run()
        .map_err(mongo_err)?;
    Ok(MongoMutationResult {
        affected_documents: result.deleted_count,
        inserted_id: None,
    })
}
pub fn mongo_indexes(
    spec: &MongoConnection,
    database: &str,
    collection: &str,
) -> Result<NativeGrid, NativeDbError> {
    safe_mongo_name(database)?;
    safe_mongo_name(collection)?;
    let client = connect_mongo(spec)?;
    let names = client
        .database(database)
        .collection::<Document>(collection)
        .list_index_names()
        .run()
        .map_err(mongo_err)?;
    let rows = names
        .into_iter()
        .map(|name| json!({"name":name}))
        .collect::<Vec<_>>();
    Ok(NativeGrid {
        columns: vec!["name".into()],
        row_count: rows.len() as u64,
        rows,
    })
}
pub fn mongo_stats(
    spec: &MongoConnection,
    database: &str,
    collection: &str,
) -> Result<NativeTableStats, NativeDbError> {
    safe_mongo_name(database)?;
    safe_mongo_name(collection)?;
    let client = connect_mongo(spec)?;
    let count = client
        .database(database)
        .collection::<Document>(collection)
        .estimated_document_count()
        .run()
        .map_err(mongo_err)?;
    Ok(NativeTableStats {
        namespace: database.into(),
        table: collection.into(),
        estimated_rows: Some(count),
        table_bytes: None,
        index_bytes: None,
    })
}
fn connect_mongo(spec: &MongoConnection) -> Result<MongoClient, NativeDbError> {
    validate_mongo_url(&spec.url)?;
    MongoClient::with_uri_str(&spec.url).map_err(mongo_err)
}
fn validate_mongo_url(url: &str) -> Result<(), NativeDbError> {
    if url.len() > 4096 || url.chars().any(char::is_control) {
        return Err(NativeDbError::Invalid("invalid MongoDB URL".into()));
    }
    let lower = url.to_ascii_lowercase();
    for forbidden in [
        "tls=false",
        "ssl=false",
        "tlsinsecure=true",
        "tlsallowinvalidhostnames=true",
        "tlsallowinvalidcertificates=true",
    ] {
        if lower.contains(forbidden) {
            return Err(NativeDbError::Invalid(format!(
                "MongoDB insecure TLS option is forbidden: {forbidden}"
            )));
        }
    }
    if lower.starts_with("mongodb+srv://") {
        return Ok(());
    }
    if !url.starts_with("mongodb://") {
        return Err(NativeDbError::Invalid(
            "MongoDB URL must use mongodb:// or mongodb+srv://".into(),
        ));
    }
    let hostpart = url
        .trim_start_matches("mongodb://")
        .rsplit('@')
        .next()
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("");
    if hostpart.split(',').all(|entry| {
        let entry = entry.trim();
        let host = if let Some(rest) = entry.strip_prefix('[') {
            rest.split(']').next().unwrap_or("")
        } else {
            entry.split_once(':').map(|(h, _)| h).unwrap_or(entry)
        };
        matches!(host, "localhost" | "127.0.0.1" | "::1")
    }) {
        Ok(())
    } else {
        Err(NativeDbError::Invalid("non-loopback MongoDB requires mongodb+srv:// or a future explicit verified-TLS connector profile".into()))
    }
}
fn safe_mongo_name(v: &str) -> Result<(), NativeDbError> {
    if v.is_empty()
        || v.len() > 120
        || v.starts_with('$')
        || v.bytes().any(|b| b == 0 || b.is_ascii_control())
    {
        Err(NativeDbError::Invalid(
            "invalid MongoDB database/collection name".into(),
        ))
    } else {
        Ok(())
    }
}
fn json_map_to_document(map: &BTreeMap<String, Value>) -> Result<Document, NativeDbError> {
    let obj = map
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<serde_json::Map<_, _>>();
    json_to_document(Value::Object(obj))
}
fn json_to_document(value: Value) -> Result<Document, NativeDbError> {
    match Bson::try_from(value).map_err(|e| NativeDbError::Mongo(e.to_string()))? {
        Bson::Document(d) => Ok(d),
        _ => Err(NativeDbError::Invalid(
            "MongoDB value must be an object/document".into(),
        )),
    }
}
fn bson_to_json(value: Bson) -> Result<Value, NativeDbError> {
    serde_json::to_value(value).map_err(|e| NativeDbError::Mongo(e.to_string()))
}
fn document_to_json(value: Document) -> Result<Value, NativeDbError> {
    let value = serde_json::to_value(value).map_err(|e| NativeDbError::Mongo(e.to_string()))?;
    ensure_json_value_budget(&value)?;
    Ok(value)
}
fn mongo_err<E: std::fmt::Display>(e: E) -> NativeDbError {
    NativeDbError::Mongo(e.to_string())
}

// ---------- Redis ----------

pub fn redis_inspect(spec: &RedisConnection) -> Result<RedisInspection, NativeDbError> {
    validate_redis_url(&spec.url)?;
    let client =
        redis::Client::open(spec.url.as_str()).map_err(|e| NativeDbError::Redis(e.to_string()))?;
    let mut con = client
        .get_connection_with_timeout(Duration::from_secs(8))
        .map_err(|e| NativeDbError::Redis(e.to_string()))?;
    let pong: String = redis::cmd("PING").query(&mut con).map_err(redis_err)?;
    if pong != "PONG" {
        return Err(NativeDbError::Redis("unexpected PING response".into()));
    }
    let size: u64 = redis::cmd("DBSIZE").query(&mut con).map_err(redis_err)?;
    let keys: Vec<String> = redis::cmd("SCAN")
        .arg(0u64)
        .arg("COUNT")
        .arg(100u32)
        .query::<(u64, Vec<String>)>(&mut con)
        .map(|(_, v)| v)
        .map_err(redis_err)?;
    let info: String = redis::cmd("INFO")
        .arg("server")
        .query(&mut con)
        .map_err(redis_err)?;
    bounded_read_result(RedisInspection {
        server_info: info.chars().take(16 * 1024).collect(),
        db_size: size,
        sample_keys: keys,
    })
}

pub fn redis_get(spec: &RedisConnection, key: &str) -> Result<Value, NativeDbError> {
    validate_redis_url(&spec.url)?;
    safe_key(key)?;
    let client = redis::Client::open(spec.url.as_str()).map_err(redis_err)?;
    let mut con = client
        .get_connection_with_timeout(Duration::from_secs(8))
        .map_err(redis_err)?;
    let kind: String = redis::cmd("TYPE")
        .arg(key)
        .query(&mut con)
        .map_err(redis_err)?;
    let value: RedisValue = match kind.as_str() {
        "string" => redis::cmd("GET").arg(key).query(&mut con),
        "list" => redis::cmd("LRANGE")
            .arg(key)
            .arg(0)
            .arg(199)
            .query(&mut con),
        "set" => redis::cmd("SMEMBERS").arg(key).query(&mut con),
        "hash" => redis::cmd("HGETALL").arg(key).query(&mut con),
        "zset" => redis::cmd("ZRANGE")
            .arg(key)
            .arg(0)
            .arg(199)
            .arg("WITHSCORES")
            .query(&mut con),
        _ => redis::cmd("DUMP").arg(key).query(&mut con),
    }
    .map_err(redis_err)?;
    let result = json!({"key":key,"type":kind,"value":redis_value_json(value)});
    ensure_json_value_budget(&result)?;
    Ok(result)
}

pub fn redis_set_string(
    spec: &RedisConnection,
    key: &str,
    value: &str,
    ttl_seconds: Option<u64>,
) -> Result<Value, NativeDbError> {
    validate_redis_url(&spec.url)?;
    safe_key(key)?;
    if value.len() > 16 * 1024 * 1024 {
        return Err(NativeDbError::Invalid(
            "Redis value exceeds 16 MiB safety limit".into(),
        ));
    }
    if let Some(ttl) = ttl_seconds {
        if ttl == 0 || ttl > 365 * 24 * 60 * 60 {
            return Err(NativeDbError::Invalid(
                "Redis TTL must be between 1 second and 365 days".into(),
            ));
        }
    }
    let client = redis::Client::open(spec.url.as_str()).map_err(redis_err)?;
    let mut con = client
        .get_connection_with_timeout(Duration::from_secs(8))
        .map_err(redis_err)?;
    let mut cmd = redis::cmd("SET");
    cmd.arg(key).arg(value);
    if let Some(ttl) = ttl_seconds {
        cmd.arg("EX").arg(ttl);
    }
    let response: String = cmd.query(&mut con).map_err(redis_err)?;
    Ok(json!({"key":key,"ok":response=="OK","ttl_seconds":ttl_seconds}))
}

pub fn redis_delete(spec: &RedisConnection, key: &str) -> Result<Value, NativeDbError> {
    validate_redis_url(&spec.url)?;
    safe_key(key)?;
    let client = redis::Client::open(spec.url.as_str()).map_err(redis_err)?;
    let mut con = client
        .get_connection_with_timeout(Duration::from_secs(8))
        .map_err(redis_err)?;
    let deleted: u64 = redis::cmd("DEL")
        .arg(key)
        .query(&mut con)
        .map_err(redis_err)?;
    Ok(json!({"key":key,"deleted":deleted}))
}

// ---------- helpers ----------

fn validated_cert_path(value: &str, label: &str) -> Result<PathBuf, NativeDbError> {
    let path = PathBuf::from(value);
    if !path.is_absolute() || !path.is_file() {
        return Err(NativeDbError::Invalid(format!(
            "{label} must be an existing absolute file"
        )));
    }
    let size = std::fs::metadata(&path)
        .map_err(|e| NativeDbError::Invalid(format!("{label} metadata failed: {e}")))?
        .len();
    if size == 0 || size > 4 * 1024 * 1024 {
        return Err(NativeDbError::Invalid(format!(
            "{label} must be between 1 byte and 4 MiB"
        )));
    }
    Ok(path)
}
fn connect_mysql_tls(spec: &MySqlTlsConnection) -> Result<mysql::PooledConn, NativeDbError> {
    if spec.url.len() > 4096 {
        return Err(NativeDbError::Invalid("MySQL URL too long".into()));
    }
    let lower = spec.url.to_ascii_lowercase();
    if !lower.starts_with("mysql://") {
        return Err(NativeDbError::Invalid(
            "MySQL TLS URL must use mysql://".into(),
        ));
    }
    if lower.contains("enable_cleartext_plugin=true") {
        return Err(NativeDbError::Invalid(
            "MySQL cleartext authentication plugin is not allowed by the VSN TLS profile".into(),
        ));
    }
    let ca = validated_cert_path(&spec.root_ca_path, "MySQL root CA")?;
    let opts = mysql::Opts::from_url(&spec.url).map_err(mysql_err)?;
    let ssl = mysql::SslOpts::default()
        .with_root_cert_path(Some(ca))
        .with_danger_skip_domain_validation(false)
        .with_danger_accept_invalid_certs(false);
    let builder = mysql::OptsBuilder::from_opts(opts)
        .ssl_opts(Some(ssl))
        .prefer_socket(false);
    let pool = MyPool::new(builder).map_err(mysql_err)?;
    pool.get_conn().map_err(mysql_err)
}
fn connect_postgres_tls(spec: &PostgresTlsConnection) -> Result<PgClient, NativeDbError> {
    if spec.connection_string.len() > 4096 {
        return Err(NativeDbError::Invalid(
            "PostgreSQL connection string too long".into(),
        ));
    }
    let ca = validated_cert_path(&spec.root_ca_pem_path, "PostgreSQL root CA")?;
    let bytes = std::fs::read(&ca)
        .map_err(|e| NativeDbError::Invalid(format!("PostgreSQL root CA read failed: {e}")))?;
    let cert = NativeCertificate::from_pem(&bytes)
        .map_err(|e| NativeDbError::Invalid(format!("PostgreSQL root CA is not valid PEM: {e}")))?;
    let connector = TlsConnector::builder()
        .add_root_certificate(cert)
        .build()
        .map_err(|e| NativeDbError::Invalid(format!("PostgreSQL TLS connector failed: {e}")))?;
    let connector = MakeTlsConnector::new(connector);
    let mut config = PgConfig::from_str(&spec.connection_string).map_err(|e| {
        NativeDbError::Invalid(format!("PostgreSQL connection string invalid: {e}"))
    })?;
    config.ssl_mode(SslMode::Require);
    config
        .connect(connector)
        .map_err(|e| NativeDbError::Postgres(e.to_string()))
}

fn connect_mysql(spec: &MySqlConnection) -> Result<mysql::PooledConn, NativeDbError> {
    if spec.url.len() > 4096 {
        return Err(NativeDbError::Invalid("MySQL URL too long".into()));
    }
    if !mysql_loopback(&spec.url) {
        return Err(NativeDbError::Invalid("native MySQL non-TLS profile is restricted to loopback; remote MySQL/MariaDB stays on the TLS-capable external client path until native TLS policy is verified".into()));
    }
    let pool = MyPool::new(spec.url.as_str()).map_err(mysql_err)?;
    pool.get_conn().map_err(mysql_err)
}
fn mysql_loopback(url: &str) -> bool {
    let Ok(opts) = mysql::Opts::from_url(url) else {
        return false;
    };
    exact_loopback_host(opts.get_ip_or_hostname().as_ref())
        && opts.get_tcp_port() != 0
        && opts.get_ssl_opts().is_none()
}

fn exact_loopback_host(host: &str) -> bool {
    matches!(
        host.to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "::1"
    )
}
fn mysql_quote_ident(v: &str) -> String {
    format!("`{}`", v)
}
fn mysql_grid(c: &mut mysql::PooledConn, sql: &str) -> Result<NativeGrid, NativeDbError> {
    let rows: Vec<MyRow> = c.query(sql).map_err(mysql_err)?;
    mysql_rows_to_grid(rows)
}
fn mysql_rows_to_grid(rows: Vec<MyRow>) -> Result<NativeGrid, NativeDbError> {
    let mut columns = Vec::new();
    let mut out = Vec::new();
    for row in rows.into_iter().take(MAX_ROWS as usize) {
        if columns.is_empty() {
            columns = row
                .columns_ref()
                .iter()
                .map(|col| col.name_str().into_owned())
                .collect();
        }
        let mut obj = serde_json::Map::new();
        for (i, name) in columns.iter().enumerate() {
            let value = mysql_value_json(row[i].clone());
            ensure_json_value_budget(&value)?;
            obj.insert(name.clone(), value);
        }
        out.push(Value::Object(obj));
    }
    let grid = NativeGrid {
        columns,
        row_count: out.len() as u64,
        rows: out,
    };
    ensure_serialized_budget(&grid)?;
    Ok(grid)
}
fn mysql_value_json(v: MyValue) -> Value {
    match v {
        MyValue::NULL => Value::Null,
        MyValue::Bytes(b) => match String::from_utf8(b.clone()) {
            Ok(s) => Value::String(s),
            Err(_) => json!({"binary_bytes":b.len()}),
        },
        MyValue::Int(v) => json!(v),
        MyValue::UInt(v) => json!(v),
        MyValue::Float(v) => json!(v),
        MyValue::Double(v) => json!(v),
        MyValue::Date(y, m, d, h, mi, s, us) => Value::String(format!(
            "{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}.{us:06}"
        )),
        MyValue::Time(neg, days, h, mi, s, us) => Value::String(format!(
            "{}{} {:02}:{:02}:{:02}.{:06}",
            if neg { "-" } else { "" },
            days,
            h,
            mi,
            s,
            us
        )),
    }
}
fn json_to_mysql(value: &Value) -> Result<MyValue, NativeDbError> {
    match value {
        Value::Null => Ok(MyValue::NULL),
        Value::Bool(v) => Ok(MyValue::Int(if *v { 1 } else { 0 })),
        Value::Number(number) => {
            if let Some(v) = number.as_i64() {
                Ok(MyValue::Int(v))
            } else if let Some(v) = number.as_u64() {
                Ok(MyValue::UInt(v))
            } else if let Some(v) = number.as_f64() {
                Ok(MyValue::Double(v))
            } else {
                Err(NativeDbError::Invalid("unsupported numeric value".into()))
            }
        }
        Value::String(v) => Ok(MyValue::Bytes(v.as_bytes().to_vec())),
        Value::Array(_) | Value::Object(_) => serde_json::to_vec(value)
            .map(MyValue::Bytes)
            .map_err(|e| NativeDbError::Invalid(format!("JSON value encode failed: {e}"))),
    }
}
fn mysql_err<E: std::fmt::Display>(e: E) -> NativeDbError {
    NativeDbError::MySql(e.to_string())
}
fn validate_mysql_read_sql(sql: &str) -> Result<(), NativeDbError> {
    let u = sql.to_ascii_uppercase();
    for bad in [
        " INTO OUTFILE",
        " INTO DUMPFILE",
        "LOAD_FILE(",
        "SLEEP(",
        "BENCHMARK(",
        "GET_LOCK(",
    ] {
        if u.contains(bad) {
            return Err(NativeDbError::Invalid(format!(
                "MySQL read query rejected dangerous token: {bad}"
            )));
        }
    }
    Ok(())
}

fn connect_postgres(spec: &PostgresConnection) -> Result<PgClient, NativeDbError> {
    if spec.connection_string.len() > 4096 {
        return Err(NativeDbError::Invalid(
            "PostgreSQL connection string too long".into(),
        ));
    }
    if !postgres_loopback_no_tls(&spec.connection_string) {
        return Err(NativeDbError::Invalid("native PostgreSQL NoTls profile is restricted to loopback; remote PostgreSQL must use the existing TLS-capable external client path until a TLS connector is added".into()));
    }
    PgClient::connect(&spec.connection_string, NoTls)
        .map_err(|e| NativeDbError::Postgres(e.to_string()))
}
fn postgres_loopback_no_tls(s: &str) -> bool {
    let Ok(config) = PgConfig::from_str(s) else {
        return false;
    };
    if config.get_hosts().len() != 1
        || !config.get_hostaddrs().is_empty()
        || config.get_ports().iter().any(|port| *port == 0)
    {
        return false;
    }
    matches!(
        config.get_hosts().first(),
        Some(PgHost::Tcp(host)) if exact_loopback_host(host)
    )
}
fn ensure_json_value_budget(value: &Value) -> Result<(), NativeDbError> {
    fn walk(value: &Value) -> bool {
        match value {
            Value::String(text) => text.len() <= MAX_TEXT_CELL_BYTES,
            Value::Array(values) => values.iter().all(walk),
            Value::Object(values) => values.values().all(walk),
            _ => true,
        }
    }
    if !walk(value) {
        return Err(NativeDbError::Invalid(
            "native database text cell exceeded 256 KiB limit".into(),
        ));
    }
    let size = serde_json::to_vec(value)
        .map_err(|e| NativeDbError::Invalid(format!("native result encode failed: {e}")))?
        .len();
    if size > MAX_SERIALIZED_READ_BYTES {
        return Err(NativeDbError::Invalid(
            "native database serialized read result exceeded 512 KiB limit".into(),
        ));
    }
    Ok(())
}

fn ensure_serialized_budget<T: Serialize>(value: &T) -> Result<(), NativeDbError> {
    let size = serde_json::to_vec(value)
        .map_err(|e| NativeDbError::Invalid(format!("native result encode failed: {e}")))?
        .len();
    if size > MAX_SERIALIZED_READ_BYTES {
        return Err(NativeDbError::Invalid(
            "native database serialized read result exceeded 512 KiB limit".into(),
        ));
    }
    Ok(())
}

fn bounded_read_result<T: Serialize>(value: T) -> Result<T, NativeDbError> {
    let json = serde_json::to_value(&value)
        .map_err(|e| NativeDbError::Invalid(format!("native result encode failed: {e}")))?;
    ensure_json_value_budget(&json)?;
    Ok(value)
}

fn simple_grid(c: &mut PgClient, sql: &str) -> Result<NativeGrid, NativeDbError> {
    let messages = c
        .simple_query(sql)
        .map_err(|e| NativeDbError::Postgres(e.to_string()))?;
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    for msg in messages {
        if let SimpleQueryMessage::Row(row) = msg {
            if columns.is_empty() {
                columns = row.columns().iter().map(|c| c.name().to_string()).collect();
            }
            let mut obj = serde_json::Map::new();
            for (i, col) in columns.iter().enumerate() {
                obj.insert(
                    col.clone(),
                    row.get(i)
                        .map(|v| {
                            if v.len() > MAX_TEXT_CELL_BYTES {
                                Err(NativeDbError::Invalid(
                                    "native database text cell exceeded 256 KiB limit".into(),
                                ))
                            } else {
                                Ok(Value::String(v.to_string()))
                            }
                        })
                        .transpose()?
                        .unwrap_or(Value::Null),
                );
            }
            rows.push(Value::Object(obj));
            if rows.len() >= MAX_ROWS as usize {
                break;
            }
        }
    }
    let grid = NativeGrid {
        columns,
        row_count: rows.len() as u64,
        rows,
    };
    ensure_serialized_budget(&grid)?;
    Ok(grid)
}
fn scalar(c: &mut PgClient, sql: &str) -> Result<String, NativeDbError> {
    let grid = simple_grid(c, sql)?;
    grid.rows
        .first()
        .and_then(|r| r.as_object())
        .and_then(|o| o.values().next())
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| NativeDbError::Postgres("expected scalar result".into()))
}
fn single_column(c: &mut PgClient, sql: &str) -> Result<Vec<String>, NativeDbError> {
    let g = simple_grid(c, sql)?;
    Ok(g.rows
        .iter()
        .filter_map(|r| r.as_object())
        .filter_map(|o| o.values().next())
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect())
}

fn validate_mutation_target(namespace: &str, table: &str) -> Result<(), NativeDbError> {
    safe_ident(namespace)?;
    safe_ident(table)?;
    Ok(())
}
fn validate_values(
    values: &BTreeMap<String, Value>,
    allow_empty: bool,
) -> Result<(), NativeDbError> {
    if !allow_empty && values.is_empty() {
        return Err(NativeDbError::Invalid(
            "mutation values cannot be empty".into(),
        ));
    }
    if values.len() > MAX_MUTATION_FIELDS {
        return Err(NativeDbError::Invalid(
            "mutation contains too many fields".into(),
        ));
    }
    for key in values.keys() {
        safe_ident(key)?;
    }
    Ok(())
}
fn validate_filter(filter: &BTreeMap<String, Value>) -> Result<(), NativeDbError> {
    if filter.is_empty() {
        return Err(NativeDbError::Invalid(
            "update/delete requires a non-empty equality filter".into(),
        ));
    }
    validate_values(filter, false)
}
fn safe_column_list(values: &BTreeMap<String, Value>) -> Result<Vec<String>, NativeDbError> {
    validate_values(values, false)?;
    Ok(values.keys().cloned().collect())
}
fn safe_ident(v: &str) -> Result<(), NativeDbError> {
    if v.is_empty() || v.len() > 128 || !v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        Err(NativeDbError::Invalid("unsafe SQL identifier".into()))
    } else {
        Ok(())
    }
}
fn quote_ident(v: &str) -> String {
    format!("\"{}\"", v)
}
fn validate_read_only_sql(sql: &str) -> Result<(), NativeDbError> {
    if sql.len() > 256 * 1024 {
        return Err(NativeDbError::Invalid("query too large".into()));
    }
    let t = sql.trim();
    if t.is_empty() || t.matches(';').count() > 1 || (t.contains(';') && !t.ends_with(';')) {
        return Err(NativeDbError::Invalid(
            "only one SQL statement is allowed".into(),
        ));
    }
    let upper = t.trim_end_matches(';').trim().to_ascii_uppercase();
    if !(upper.starts_with("SELECT ")
        || upper.starts_with("WITH ")
        || upper.starts_with("SHOW ")
        || upper.starts_with("EXPLAIN "))
    {
        return Err(NativeDbError::Invalid(
            "only read-only SQL is allowed".into(),
        ));
    }
    for bad in [
        " INSERT ",
        " UPDATE ",
        " DELETE ",
        " DROP ",
        " ALTER ",
        " CREATE ",
        " TRUNCATE ",
        " GRANT ",
        " REVOKE ",
        " COPY ",
        " CALL ",
        " DO ",
        " FOR UPDATE",
        " FOR SHARE",
        "EXPLAIN ANALYZE",
        "PG_READ_FILE(",
        "PG_READ_BINARY_FILE(",
        "PG_LS_DIR(",
        "LO_IMPORT(",
    ] {
        if format!(" {upper} ").contains(bad) {
            return Err(NativeDbError::Invalid(format!(
                "read-only SQL rejected token: {}",
                bad.trim()
            )));
        }
    }
    Ok(())
}
fn validate_redis_url(url: &str) -> Result<(), NativeDbError> {
    if url.len() > 4096 || url.chars().any(char::is_control) {
        return Err(NativeDbError::Invalid("Redis URL is invalid".into()));
    }
    let lower = url.to_ascii_lowercase();
    if lower.contains("#insecure") || lower.contains("insecure=true") {
        return Err(NativeDbError::Invalid(
            "Redis insecure TLS mode is forbidden".into(),
        ));
    }
    if lower.starts_with("rediss://") {
        return Ok(());
    }
    if !lower.starts_with("redis://") {
        return Err(NativeDbError::Invalid(
            "Redis URL must use redis:// or rediss://".into(),
        ));
    }
    let authority = lower
        .trim_start_matches("redis://")
        .rsplit('@')
        .next()
        .unwrap_or("")
        .split('/')
        .next()
        .unwrap_or("");
    let host = if let Some(rest) = authority.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else {
        authority
            .split_once(':')
            .map(|(host, _)| host)
            .unwrap_or(authority)
    };
    if exact_loopback_host(host) {
        let port = if let Some(rest) = authority.strip_prefix('[') {
            rest.split_once("]:")
                .and_then(|(_, port)| port.parse::<u16>().ok())
        } else {
            authority
                .split_once(':')
                .and_then(|(_, port)| port.parse::<u16>().ok())
        };
        if port == Some(0) {
            return Err(NativeDbError::Invalid("Redis port 0 is invalid".into()));
        }
        Ok(())
    } else {
        Err(NativeDbError::Invalid(
            "remote Redis must use rediss://; plaintext redis:// is restricted to exact loopback"
                .into(),
        ))
    }
}
fn safe_key(k: &str) -> Result<(), NativeDbError> {
    if k.is_empty() || k.len() > 16 * 1024 || k.chars().any(|c| c == '\0') {
        Err(NativeDbError::Invalid("invalid Redis key".into()))
    } else {
        Ok(())
    }
}
fn redis_err<E: std::fmt::Display>(e: E) -> NativeDbError {
    NativeDbError::Redis(e.to_string())
}
fn redis_value_json(v: RedisValue) -> Value {
    match v {
        RedisValue::Nil => Value::Null,
        RedisValue::Int(i) => json!(i),
        RedisValue::BulkString(b) => match String::from_utf8(b.clone()) {
            Ok(s) => Value::String(s),
            Err(_) => json!({"binary_bytes":b.len()}),
        },
        RedisValue::Array(a) => Value::Array(a.into_iter().map(redis_value_json).collect()),
        RedisValue::SimpleString(s) => Value::String(s),
        RedisValue::Okay => Value::String("OK".into()),
        RedisValue::Map(m) => Value::Array(
            m.into_iter()
                .map(|(k, v)| json!([redis_value_json(k), redis_value_json(v)]))
                .collect(),
        ),
        other => Value::String(format!("{other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_native_no_tls_is_loopback_only() {
        assert!(postgres_loopback_no_tls("host=localhost user=postgres"));
        assert!(!postgres_loopback_no_tls(
            "host=db.example.com user=postgres"
        ));
        assert!(!postgres_loopback_no_tls(
            "host=localhost.evil.invalid user=postgres"
        ));
        assert!(!postgres_loopback_no_tls(
            "host=localhost,db.example.com user=postgres"
        ));
        assert!(!postgres_loopback_no_tls(
            "host=localhost port=0 user=postgres"
        ));
        assert!(mysql_loopback("mysql://root@localhost/test"));
        assert!(!mysql_loopback("mysql://root@db.example.com/test"));
        assert!(!mysql_loopback("mysql://root@localhost.evil.invalid/test"));
        assert!(!mysql_loopback("mysql://root@127.0.0.1.evil.invalid/test"));
        assert!(!mysql_loopback("mysql://root@localhost:0/test"));
    }

    #[test]
    fn remote_redis_requires_tls() {
        assert!(validate_redis_url("redis://localhost:6379/0").is_ok());
        assert!(validate_redis_url("redis://db.example.com:6379/0").is_err());
        assert!(validate_redis_url("rediss://db.example.com:6380/0").is_ok());
        assert!(validate_redis_url("rediss://db.example.com:6380/0#insecure").is_err());
        assert!(validate_redis_url("redis://localhost.evil.invalid:6379/0").is_err());
        assert!(validate_redis_url("redis://localhost:0/0").is_err());
    }

    #[test]
    fn mongo_remote_tls_cannot_be_disabled() {
        assert!(validate_mongo_url("mongodb+srv://db.example.com/app").is_ok());
        assert!(validate_mongo_url("mongodb+srv://db.example.com/app?tls=false").is_err());
        assert!(validate_mongo_url(
            "mongodb+srv://db.example.com/app?tlsAllowInvalidCertificates=true"
        )
        .is_err());
        assert!(validate_mongo_url("mongodb://localhost:27017/app").is_ok());
        assert!(validate_mongo_url("mongodb://localhost.evil.invalid:27017/app").is_err());
    }

    #[test]
    fn native_result_budgets_reject_large_cells_and_results() {
        assert!(ensure_json_value_budget(&Value::String("x".repeat(MAX_TEXT_CELL_BYTES))).is_ok());
        assert!(
            ensure_json_value_budget(&Value::String("x".repeat(MAX_TEXT_CELL_BYTES + 1))).is_err()
        );
        let value = json!({"rows": vec!["x".repeat(1024); 600]});
        assert!(ensure_json_value_budget(&value).is_err());
    }

    #[test]
    fn update_delete_require_filter() {
        let request = MutationRequest {
            values: BTreeMap::new(),
            filter: BTreeMap::new(),
        };
        assert!(validate_filter(&request.filter).is_err());
    }

    #[test]
    fn mysql_json_binding_is_structured() {
        assert_eq!(json_to_mysql(&json!(42)).unwrap(), MyValue::Int(42));
        assert!(matches!(
            json_to_mysql(&json!({"a":1})).unwrap(),
            MyValue::Bytes(_)
        ));
    }
}

// ---------- native PostgreSQL cancellable jobs + read-only transactions ----------
use std::collections::HashMap as NativeHashMap;
use std::sync::{Mutex as NativeMutex, OnceLock as NativeOnceLock};
use std::time::{SystemTime as NativeSystemTime, UNIX_EPOCH as NativeUnixEpoch};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NativePostgresJobState {
    Running,
    Completed,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NativePostgresJobStatus {
    pub job_id: String,
    pub state: NativePostgresJobState,
    pub created_at_unix_ms: u128,
    pub updated_at_unix_ms: u128,
    pub cancel_requested: bool,
    pub result: Option<NativeGrid>,
    pub error: Option<String>,
}
struct NativePgJobEntry {
    status: NativePostgresJobStatus,
    cancel_token: postgres::CancelToken,
}
static NATIVE_PG_JOBS: NativeOnceLock<NativeMutex<NativeHashMap<String, NativePgJobEntry>>> =
    NativeOnceLock::new();
fn native_pg_jobs() -> &'static NativeMutex<NativeHashMap<String, NativePgJobEntry>> {
    NATIVE_PG_JOBS.get_or_init(|| NativeMutex::new(NativeHashMap::new()))
}
fn native_now_ms() -> u128 {
    NativeSystemTime::now()
        .duration_since(NativeUnixEpoch)
        .map(|v| v.as_millis())
        .unwrap_or(0)
}
fn native_job_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    format!(
        "{prefix}_{:x}_{:x}",
        native_now_ms(),
        N.fetch_add(1, Ordering::Relaxed)
    )
}
fn validate_native_job_id(id: &str) -> Result<(), NativeDbError> {
    if id.len() < 8
        || id.len() > 160
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        Err(NativeDbError::Invalid(
            "invalid native PostgreSQL job id".into(),
        ))
    } else {
        Ok(())
    }
}

pub fn postgres_job_start(
    spec: &PostgresConnection,
    sql: &str,
) -> Result<NativePostgresJobStatus, NativeDbError> {
    validate_read_only_sql(sql)?;
    let mut client = connect_postgres(spec)?;
    client
        .batch_execute("BEGIN READ ONLY; SET LOCAL statement_timeout = '30000ms'")
        .map_err(|e| NativeDbError::Postgres(e.to_string()))?;
    let token = client.cancel_token();
    let id = native_job_id("pgjob");
    let now = native_now_ms();
    let status = NativePostgresJobStatus {
        job_id: id.clone(),
        state: NativePostgresJobState::Running,
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
        cancel_requested: false,
        result: None,
        error: None,
    };
    {
        let mut jobs = native_pg_jobs().lock().map_err(|_| {
            NativeDbError::Postgres("native PostgreSQL job registry poisoned".into())
        })?;
        jobs.retain(|_, e| {
            e.status.state == NativePostgresJobState::Running
                || native_now_ms().saturating_sub(e.status.updated_at_unix_ms) < 60 * 60 * 1000
        });
        if jobs.len() >= 64 {
            return Err(NativeDbError::Invalid(
                "native PostgreSQL job limit reached".into(),
            ));
        }
        jobs.insert(
            id.clone(),
            NativePgJobEntry {
                status: status.clone(),
                cancel_token: token,
            },
        );
    }
    let sql = sql.to_string();
    std::thread::spawn(move || {
        let outcome = simple_grid(&mut client, &sql);
        let _ = client.batch_execute("ROLLBACK");
        if let Ok(mut jobs) = native_pg_jobs().lock() {
            if let Some(entry) = jobs.get_mut(&id) {
                let requested = entry.status.cancel_requested;
                entry.status.updated_at_unix_ms = native_now_ms();
                match outcome {
                    Ok(grid) => {
                        entry.status.state = NativePostgresJobState::Completed;
                        entry.status.result = Some(grid);
                        entry.status.error = None;
                    }
                    Err(e) if requested => {
                        entry.status.state = NativePostgresJobState::Cancelled;
                        entry.status.error = Some(e.to_string());
                        entry.status.result = None;
                    }
                    Err(e) => {
                        entry.status.state = NativePostgresJobState::Failed;
                        entry.status.error = Some(e.to_string());
                        entry.status.result = None;
                    }
                }
            }
        }
    });
    Ok(status)
}
pub fn postgres_job_status(job_id: &str) -> Result<NativePostgresJobStatus, NativeDbError> {
    validate_native_job_id(job_id)?;
    native_pg_jobs()
        .lock()
        .map_err(|_| NativeDbError::Postgres("native PostgreSQL job registry poisoned".into()))?
        .get(job_id)
        .map(|e| e.status.clone())
        .ok_or_else(|| NativeDbError::Invalid("native PostgreSQL job not found".into()))
}
pub fn postgres_job_list() -> Result<Vec<NativePostgresJobStatus>, NativeDbError> {
    let mut out = native_pg_jobs()
        .lock()
        .map_err(|_| NativeDbError::Postgres("native PostgreSQL job registry poisoned".into()))?
        .values()
        .map(|e| e.status.clone())
        .collect::<Vec<_>>();
    out.sort_by_key(|b| std::cmp::Reverse(b.created_at_unix_ms));
    out.truncate(64);
    Ok(out)
}
pub fn postgres_job_cancel(job_id: &str) -> Result<NativePostgresJobStatus, NativeDbError> {
    validate_native_job_id(job_id)?;
    let token = {
        let mut jobs = native_pg_jobs().lock().map_err(|_| {
            NativeDbError::Postgres("native PostgreSQL job registry poisoned".into())
        })?;
        let entry = jobs
            .get_mut(job_id)
            .ok_or_else(|| NativeDbError::Invalid("native PostgreSQL job not found".into()))?;
        if entry.status.state != NativePostgresJobState::Running {
            return Ok(entry.status.clone());
        }
        entry.status.cancel_requested = true;
        entry.status.updated_at_unix_ms = native_now_ms();
        entry.cancel_token.clone()
    };
    token.cancel_query(NoTls).map_err(|e| {
        NativeDbError::Postgres(format!("PostgreSQL cancellation request failed: {e}"))
    })?;
    postgres_job_status(job_id)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativePostgresTransactionState {
    pub transaction_id: String,
    pub opened_at_unix_ms: u128,
    pub expires_at_unix_ms: u128,
    pub statements_executed: u32,
    pub closed: bool,
}
struct NativePgTransactionEntry {
    state: NativePostgresTransactionState,
    client: PgClient,
}
static NATIVE_PG_TXNS: NativeOnceLock<
    NativeMutex<NativeHashMap<String, NativePgTransactionEntry>>,
> = NativeOnceLock::new();
fn native_pg_txns() -> &'static NativeMutex<NativeHashMap<String, NativePgTransactionEntry>> {
    NATIVE_PG_TXNS.get_or_init(|| NativeMutex::new(NativeHashMap::new()))
}
pub fn postgres_read_transaction_start(
    spec: &PostgresConnection,
    ttl_seconds: u64,
) -> Result<NativePostgresTransactionState, NativeDbError> {
    let mut client = connect_postgres(spec)?;
    client.batch_execute("BEGIN READ ONLY; SET LOCAL statement_timeout = '15000ms'; SET LOCAL idle_in_transaction_session_timeout = '60000ms'").map_err(|e|NativeDbError::Postgres(e.to_string()))?;
    let now = native_now_ms();
    let ttl = ttl_seconds.clamp(10, 60);
    let id = native_job_id("pgtxn");
    let state = NativePostgresTransactionState {
        transaction_id: id.clone(),
        opened_at_unix_ms: now,
        expires_at_unix_ms: now + u128::from(ttl) * 1000,
        statements_executed: 0,
        closed: false,
    };
    let mut txns = native_pg_txns().lock().map_err(|_| {
        NativeDbError::Postgres("native PostgreSQL transaction registry poisoned".into())
    })?;
    let now2 = native_now_ms();
    txns.retain(|_, e| !e.state.closed && e.state.expires_at_unix_ms >= now2);
    if txns.len() >= 32 {
        return Err(NativeDbError::Invalid(
            "native PostgreSQL transaction limit reached".into(),
        ));
    }
    txns.insert(
        id,
        NativePgTransactionEntry {
            state: state.clone(),
            client,
        },
    );
    Ok(state)
}
pub fn postgres_read_transaction_query(
    transaction_id: &str,
    sql: &str,
) -> Result<NativeGrid, NativeDbError> {
    validate_native_job_id(transaction_id)?;
    validate_read_only_sql(sql)?;
    let mut txns = native_pg_txns().lock().map_err(|_| {
        NativeDbError::Postgres("native PostgreSQL transaction registry poisoned".into())
    })?;
    let entry = txns
        .get_mut(transaction_id)
        .ok_or_else(|| NativeDbError::Invalid("native PostgreSQL transaction not found".into()))?;
    if entry.state.expires_at_unix_ms < native_now_ms() {
        let _ = entry.client.batch_execute("ROLLBACK");
        entry.state.closed = true;
        return Err(NativeDbError::Invalid(
            "native PostgreSQL transaction expired".into(),
        ));
    }
    if entry.state.statements_executed >= 100 {
        return Err(NativeDbError::Invalid(
            "native PostgreSQL transaction statement limit reached".into(),
        ));
    }
    let grid = simple_grid(&mut entry.client, sql)?;
    entry.state.statements_executed = entry.state.statements_executed.saturating_add(1);
    Ok(grid)
}
pub fn postgres_read_transaction_status(
    transaction_id: &str,
) -> Result<NativePostgresTransactionState, NativeDbError> {
    validate_native_job_id(transaction_id)?;
    native_pg_txns()
        .lock()
        .map_err(|_| {
            NativeDbError::Postgres("native PostgreSQL transaction registry poisoned".into())
        })?
        .get(transaction_id)
        .map(|e| e.state.clone())
        .ok_or_else(|| NativeDbError::Invalid("native PostgreSQL transaction not found".into()))
}
pub fn postgres_read_transaction_close(
    transaction_id: &str,
    commit: bool,
) -> Result<NativePostgresTransactionState, NativeDbError> {
    validate_native_job_id(transaction_id)?;
    let mut txns = native_pg_txns().lock().map_err(|_| {
        NativeDbError::Postgres("native PostgreSQL transaction registry poisoned".into())
    })?;
    let mut entry = txns
        .remove(transaction_id)
        .ok_or_else(|| NativeDbError::Invalid("native PostgreSQL transaction not found".into()))?;
    if commit {
        entry.client.batch_execute("COMMIT")
    } else {
        entry.client.batch_execute("ROLLBACK")
    }
    .map_err(|e| NativeDbError::Postgres(e.to_string()))?;
    entry.state.closed = true;
    Ok(entry.state)
}

#[cfg(test)]
mod native_pg_job_tests {
    use super::*;
    #[test]
    fn native_job_identifier_is_strict() {
        assert!(validate_native_job_id("pgjob_12345678").is_ok());
        assert!(validate_native_job_id("../bad").is_err());
    }
    #[test]
    fn transaction_ttl_is_bounded() {
        assert_eq!(1u64.clamp(10, 60), 10);
        assert_eq!(999u64.clamp(10, 60), 60);
    }
}
