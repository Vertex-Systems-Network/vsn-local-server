use rusqlite::{params_from_iter, types::{Value as SqlValue, ValueRef}, Connection, OpenFlags};
use serde_json::{json, Map, Value};
use std::{collections::{BTreeMap, HashMap}, path::{Path, PathBuf}, sync::{Mutex, MutexGuard}};
use vsn_database::{BrowsePage, BrowseRequest, CapabilitySet, DataModel, DatabaseError, DatabaseProvider, EntityMeta, EntityStatistics, FieldMeta, FieldType, IndexMeta, MutationRequest, MutationResult, RelationMeta};

const MAX_READ_RESULT_BYTES:usize=16*1024*1024;
const MAX_TEXT_CELL_BYTES:usize=8*1024*1024;

pub struct SqliteProvider {
    path: Option<PathBuf>,
    connection: Option<Mutex<Connection>>,
    read_only: bool,
}
impl Default for SqliteProvider { fn default() -> Self { Self::new(true) } }

impl SqliteProvider {
    pub fn new(read_only: bool) -> Self { Self { path: None, connection: None, read_only } }
    pub fn open(path: &Path, read_only: bool) -> Result<Self, DatabaseError> { let mut provider=Self::new(read_only);provider.connect(&json!({"path":path}))?;Ok(provider) }
    pub fn path(&self)->Option<&Path>{self.path.as_deref()}
    fn conn(&self)->Result<MutexGuard<'_,Connection>,DatabaseError>{self.connection.as_ref().ok_or_else(||DatabaseError::Provider("SQLite provider is not connected".into()))?.lock().map_err(|_|DatabaseError::Provider("SQLite connection lock poisoned".into()))}
    fn ensure_write(&self)->Result<(),DatabaseError>{if self.read_only{Err(DatabaseError::Invalid("SQLite provider is read-only".into()))}else{Ok(())}}
}

impl DatabaseProvider for SqliteProvider {
    fn id(&self) -> &str { "sqlite" }
    fn model(&self) -> DataModel { DataModel::Relational }
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet { connect:true,introspect:true,query:true,browse:true,insert:!self.read_only,update:!self.read_only,delete:!self.read_only,schemas:false,indexes:true,relations:true,functions:false,users:false,permissions:false,import:false,export:true,backup:true,restore:!self.read_only,statistics:true }
    }
    fn connect(&mut self, connection: &Value) -> Result<(), DatabaseError> {
        let raw=connection.get("path").and_then(Value::as_str).ok_or_else(||DatabaseError::Invalid("SQLite connection requires path".into()))?;
        let path=PathBuf::from(raw);if !path.is_file(){return Err(DatabaseError::Invalid(format!("SQLite file not found: {}",path.display())));}
        let flags=if self.read_only { OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX } else { OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX };
        let conn=Connection::open_with_flags(&path,flags).map_err(db_err)?;conn.pragma_update(None,"foreign_keys","ON").map_err(db_err)?;conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(db_err)?;
        self.path=Some(path);self.connection=Some(Mutex::new(conn));Ok(())
    }
    fn disconnect(&mut self) -> Result<(), DatabaseError> { self.connection.take(); Ok(()) }
    fn list_namespaces(&self) -> Result<Vec<String>, DatabaseError> { Ok(vec!["main".into()]) }
    fn list_entities(&self, _namespace: Option<&str>) -> Result<Vec<String>, DatabaseError> {
        let conn=self.conn()?;let mut stmt=conn.prepare("SELECT name FROM sqlite_schema WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name").map_err(db_err)?;
        let rows=stmt.query_map([],|row|row.get::<_,String>(0)).map_err(db_err)?;rows.collect::<Result<Vec<_>,_>>().map_err(db_err)
    }
    fn describe_entity(&self, _namespace: Option<&str>, entity: &str) -> Result<EntityMeta, DatabaseError> {
        validate_identifier(entity)?;let conn=self.conn()?;let sql=format!("PRAGMA table_info({})",quote_identifier(entity));let mut stmt=conn.prepare(&sql).map_err(db_err)?;
        let rows=stmt.query_map([],|row|{let name:String=row.get(1)?;let declared:String=row.get::<_,Option<String>>(2)?.unwrap_or_default();let notnull:i64=row.get(3)?;let default_value:Option<String>=row.get(4)?;let primary:i64=row.get(5)?;Ok(FieldMeta{name,field_type:sqlite_type(&declared),nullable:notnull==0&&primary==0,primary:primary>0,generated:false,enum_values:vec![],relation_target:None,metadata:json!({"declared_type":declared,"default":default_value,"pk_order":primary})})}).map_err(db_err)?;
        let mut fields=rows.collect::<Result<Vec<_>,_>>().map_err(db_err)?;attach_foreign_keys(&conn,entity,&mut fields)?;
        let kind:String=conn.query_row("SELECT type FROM sqlite_schema WHERE name=?1 LIMIT 1",[entity],|row|row.get(0)).map_err(db_err)?;
        Ok(EntityMeta{name:entity.into(),display_name:entity.into(),fields,metadata:json!({"engine":"sqlite","kind":kind})})
    }
    fn query(&self, statement: &str, parameters: &Value) -> Result<Value, DatabaseError> {
        validate_read_query(statement)?;let bind=json_params(parameters)?;let conn=self.conn()?;let mut stmt=conn.prepare(statement).map_err(db_err)?;
        let names:Vec<String>=stmt.column_names().iter().map(|v|(*v).to_string()).collect();if names.is_empty(){return Err(DatabaseError::Invalid("query returned no columns".into()));}
        let mut rows=stmt.query(params_from_iter(bind.iter())).map_err(db_err)?;let mut out=Vec::new();let mut count=0usize;let mut result_bytes=0usize;
        while let Some(row)=rows.next().map_err(db_err)? {if count>=10_000{return Err(DatabaseError::Invalid("query row limit exceeded (10000)".into()));}let mut object=Map::new();for(i,name)in names.iter().enumerate(){object.insert(name.clone(),value_ref(row.get_ref(i).map_err(db_err)?));}let value=Value::Object(object);result_bytes=result_bytes.saturating_add(serde_json::to_vec(&value).map_err(|e|DatabaseError::Provider(e.to_string()))?.len());if result_bytes>MAX_READ_RESULT_BYTES{return Err(DatabaseError::Invalid("query result exceeds 16 MiB safety limit".into()));}out.push(value);count+=1;}
        Ok(json!({"columns":names,"rows":out,"row_count":count,"truncated":false}))
    }
    fn browse(&self,_namespace:Option<&str>,entity:&str,request:&BrowseRequest)->Result<BrowsePage,DatabaseError>{
        validate_identifier(entity)?;let limit=request.limit.clamp(1,1000);let offset=request.offset.min(10_000_000_000);let meta=self.describe_entity(None,entity)?;
        let order=if let Some(field)=request.order_by.as_ref(){validate_identifier(field)?;if !meta.fields.iter().any(|f|f.name==*field){return Err(DatabaseError::Invalid("order_by is not a field on the entity".into()));}format!(" ORDER BY {} {}",quote_identifier(field),if request.descending{"DESC"}else{"ASC"})}else{String::new()};
        let conn=self.conn()?;let total_i64:i64=conn.query_row(&format!("SELECT COUNT(*) FROM {}",quote_identifier(entity)),[],|row|row.get(0)).map_err(db_err)?;
        let sql=format!("SELECT * FROM {}{} LIMIT ?1 OFFSET ?2",quote_identifier(entity),order);let mut stmt=conn.prepare(&sql).map_err(db_err)?;let columns=stmt.column_names().iter().map(|v|(*v).to_string()).collect::<Vec<_>>();let mut rows=stmt.query([i64::from(limit),i64::try_from(offset).unwrap_or(i64::MAX)]).map_err(db_err)?;let mut out=Vec::new();let mut result_bytes=0usize;
        while let Some(row)=rows.next().map_err(db_err)?{let mut object=Map::new();for(i,name)in columns.iter().enumerate(){object.insert(name.clone(),value_ref(row.get_ref(i).map_err(db_err)?));}let value=Value::Object(object);result_bytes=result_bytes.saturating_add(serde_json::to_vec(&value).map_err(|e|DatabaseError::Provider(e.to_string()))?.len());if result_bytes>MAX_READ_RESULT_BYTES{return Err(DatabaseError::Invalid("browse result exceeds 16 MiB safety limit".into()));}out.push(value);}
        Ok(BrowsePage{entity:entity.into(),columns,rows:out,total_rows:u64::try_from(total_i64.max(0)).unwrap_or(0),limit,offset})
    }
    fn insert(&self,_namespace:Option<&str>,entity:&str,request:&MutationRequest)->Result<MutationResult,DatabaseError>{
        self.ensure_write()?;validate_identifier(entity)?;if request.values.is_empty(){return Err(DatabaseError::Invalid("insert requires at least one value".into()));}validate_map(&request.values,64)?;
        let fields=request.values.keys().cloned().collect::<Vec<_>>();let placeholders=(1..=fields.len()).map(|i|format!("?{i}")).collect::<Vec<_>>().join(",");let sql=format!("INSERT INTO {} ({}) VALUES ({})",quote_identifier(entity),fields.iter().map(|f|quote_identifier(f)).collect::<Vec<_>>().join(","),placeholders);let values=fields.iter().map(|f|json_to_sql(request.values.get(f).expect("known key"))).collect::<Result<Vec<_>,_>>()?;
        let conn=self.conn()?;let affected=conn.execute(&sql,params_from_iter(values.iter())).map_err(db_err)?;Ok(MutationResult{affected_rows:affected as u64,last_insert_id:Some(conn.last_insert_rowid())})
    }
    fn update(&self,_namespace:Option<&str>,entity:&str,request:&MutationRequest)->Result<MutationResult,DatabaseError>{
        self.ensure_write()?;validate_identifier(entity)?;if request.values.is_empty(){return Err(DatabaseError::Invalid("update requires values".into()));}if request.filter.is_empty(){return Err(DatabaseError::Invalid("update requires a non-empty equality filter".into()));}validate_map(&request.values,64)?;validate_map(&request.filter,32)?;
        let value_fields=request.values.keys().cloned().collect::<Vec<_>>();let filter_fields=request.filter.keys().cloned().collect::<Vec<_>>();let sets=value_fields.iter().enumerate().map(|(i,f)|format!("{}=?{}",quote_identifier(f),i+1)).collect::<Vec<_>>().join(",");let where_sql=filter_fields.iter().enumerate().map(|(i,f)|format!("{}=?{}",quote_identifier(f),value_fields.len()+i+1)).collect::<Vec<_>>().join(" AND ");let sql=format!("UPDATE {} SET {} WHERE {}",quote_identifier(entity),sets,where_sql);
        let mut values=value_fields.iter().map(|f|json_to_sql(request.values.get(f).expect("known key"))).collect::<Result<Vec<_>,_>>()?;values.extend(filter_fields.iter().map(|f|json_to_sql(request.filter.get(f).expect("known key"))).collect::<Result<Vec<_>,_>>()?);let conn=self.conn()?;let affected=conn.execute(&sql,params_from_iter(values.iter())).map_err(db_err)?;Ok(MutationResult{affected_rows:affected as u64,last_insert_id:None})
    }
    fn delete(&self,_namespace:Option<&str>,entity:&str,request:&MutationRequest)->Result<MutationResult,DatabaseError>{
        self.ensure_write()?;validate_identifier(entity)?;if request.filter.is_empty(){return Err(DatabaseError::Invalid("delete requires a non-empty equality filter".into()));}validate_map(&request.filter,32)?;let fields=request.filter.keys().cloned().collect::<Vec<_>>();let where_sql=fields.iter().enumerate().map(|(i,f)|format!("{}=?{}",quote_identifier(f),i+1)).collect::<Vec<_>>().join(" AND ");let sql=format!("DELETE FROM {} WHERE {}",quote_identifier(entity),where_sql);let values=fields.iter().map(|f|json_to_sql(request.filter.get(f).expect("known key"))).collect::<Result<Vec<_>,_>>()?;let conn=self.conn()?;let affected=conn.execute(&sql,params_from_iter(values.iter())).map_err(db_err)?;Ok(MutationResult{affected_rows:affected as u64,last_insert_id:None})
    }
    fn list_indexes(&self,_namespace:Option<&str>,entity:&str)->Result<Vec<IndexMeta>,DatabaseError>{
        validate_identifier(entity)?;let conn=self.conn()?;let mut stmt=conn.prepare(&format!("PRAGMA index_list({})",quote_identifier(entity))).map_err(db_err)?;let base=stmt.query_map([],|row|Ok((row.get::<_,String>(1)?,row.get::<_,i64>(2)?!=0,row.get::<_,String>(3).unwrap_or_default()))).map_err(db_err)?.collect::<Result<Vec<_>,_>>().map_err(db_err)?;drop(stmt);let mut out=Vec::new();for(name,unique,origin)in base{validate_identifier(&name)?;let mut detail=conn.prepare(&format!("PRAGMA index_info({})",quote_identifier(&name))).map_err(db_err)?;let columns=detail.query_map([],|row|row.get::<_,String>(2)).map_err(db_err)?.collect::<Result<Vec<_>,_>>().map_err(db_err)?;out.push(IndexMeta{name,unique,primary:origin=="pk",columns,metadata:json!({"origin":origin})});}Ok(out)
    }
    fn list_relations(&self,_namespace:Option<&str>,entity:&str)->Result<Vec<RelationMeta>,DatabaseError>{
        validate_identifier(entity)?;let conn=self.conn()?;relations_for(&conn,entity)
    }
    fn statistics(&self,_namespace:Option<&str>,entity:&str)->Result<EntityStatistics,DatabaseError>{
        validate_identifier(entity)?;let conn=self.conn()?;let count:i64=conn.query_row(&format!("SELECT COUNT(*) FROM {}",quote_identifier(entity)),[],|row|row.get(0)).map_err(db_err)?;let file_bytes=self.path.as_ref().and_then(|p|std::fs::metadata(p).ok()).map(|m|m.len());Ok(EntityStatistics{entity:entity.into(),row_count:Some(u64::try_from(count.max(0)).unwrap_or(0)),storage_bytes:file_bytes,index_bytes:None,metadata:json!({"database_file_bytes":file_bytes})})
    }
}

pub fn inspect(path:&Path)->Result<Value,DatabaseError>{let provider=SqliteProvider::open(path,true)?;let entities=provider.list_entities(None)?;let mut described=Vec::new();for entity in entities{let meta=provider.describe_entity(None,&entity)?;let indexes=provider.list_indexes(None,&entity).unwrap_or_default();let relations=provider.list_relations(None,&entity).unwrap_or_default();let statistics=provider.statistics(None,&entity).ok();described.push(json!({"entity":meta,"indexes":indexes,"relations":relations,"statistics":statistics}));}Ok(json!({"provider":"sqlite","path":path,"capabilities":provider.capabilities(),"entities":described}))}

fn attach_foreign_keys(conn:&Connection,entity:&str,fields:&mut[FieldMeta])->Result<(),DatabaseError>{for rel in relations_for(conn,entity)?{for field in rel.from_fields{if let Some(item)=fields.iter_mut().find(|f|f.name==field){item.field_type=FieldType::Relation;item.relation_target=Some(rel.to_entity.clone());}}}Ok(())}
fn relations_for(conn:&Connection,entity:&str)->Result<Vec<RelationMeta>,DatabaseError>{let sql=format!("PRAGMA foreign_key_list({})",quote_identifier(entity));let mut stmt=conn.prepare(&sql).map_err(db_err)?;let rows=stmt.query_map([],|row|Ok((row.get::<_,i64>(0)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5).ok(),row.get::<_,String>(6).ok()))).map_err(db_err)?;let mut grouped:HashMap<i64,RelationMeta>=HashMap::new();for row in rows{let(id,target,from,to,on_update,on_delete)=row.map_err(db_err)?;let rel=grouped.entry(id).or_insert_with(||RelationMeta{name:format!("fk_{entity}_{id}"),from_entity:entity.into(),from_fields:vec![],to_entity:target.clone(),to_fields:vec![],on_update:on_update.clone(),on_delete:on_delete.clone()});rel.from_fields.push(from);rel.to_fields.push(to);}let mut out=grouped.into_values().collect::<Vec<_>>();out.sort_by(|a,b|a.name.cmp(&b.name));Ok(out)}
fn sqlite_type(value:&str)->FieldType{let v=value.to_ascii_uppercase();if v.contains("INT"){FieldType::Integer}else if v.contains("CHAR")||v.contains("CLOB")||v.contains("TEXT"){FieldType::Text}else if v.contains("BLOB"){FieldType::Binary}else if v.contains("REAL")||v.contains("FLOA")||v.contains("DOUB")||v.contains("NUM")||v.contains("DEC"){FieldType::Decimal}else if v.contains("BOOL"){FieldType::Boolean}else if v.contains("DATE")&&v.contains("TIME"){FieldType::DateTime}else if v.contains("DATE"){FieldType::Date}else if v.contains("JSON"){FieldType::Json}else{FieldType::Unknown}}
fn value_ref(v:ValueRef<'_>)->Value{match v{ValueRef::Null=>Value::Null,ValueRef::Integer(v)=>json!(v),ValueRef::Real(v)=>json!(v),ValueRef::Text(v)=>{if v.len()>MAX_TEXT_CELL_BYTES{json!({"type":"text","bytes":v.len(),"truncated":true})}else{Value::String(String::from_utf8_lossy(v).into_owned())}},ValueRef::Blob(v)=>json!({"type":"blob","bytes":v.len()})}}
fn json_to_sql(v:&Value)->Result<SqlValue,DatabaseError>{Ok(match v{Value::Null=>SqlValue::Null,Value::Bool(v)=>SqlValue::Integer(if *v{1}else{0}),Value::Number(n)=>{if let Some(i)=n.as_i64(){SqlValue::Integer(i)}else if let Some(f)=n.as_f64(){SqlValue::Real(f)}else{return Err(DatabaseError::Invalid("numeric value is outside SQLite range".into()));}},Value::String(v)=>{if v.len()>8*1024*1024{return Err(DatabaseError::Invalid("string value exceeds 8 MiB".into()));}SqlValue::Text(v.clone())},Value::Array(_) | Value::Object(_)=>{let text=serde_json::to_string(v).map_err(|e|DatabaseError::Invalid(e.to_string()))?;if text.len()>8*1024*1024{return Err(DatabaseError::Invalid("JSON value exceeds 8 MiB".into()));}SqlValue::Text(text)}})}
fn json_params(v:&Value)->Result<Vec<SqlValue>,DatabaseError>{match v{Value::Null=>Ok(vec![]),Value::Array(items)=>{if items.len()>1024{return Err(DatabaseError::Invalid("too many SQLite bind parameters".into()));}items.iter().map(json_to_sql).collect()},_=>Err(DatabaseError::Invalid("SQLite parameters must be a JSON array or null".into()))}}
fn validate_map(map:&BTreeMap<String,Value>,max:usize)->Result<(),DatabaseError>{if map.len()>max{return Err(DatabaseError::Invalid("too many mutation fields".into()));}for key in map.keys(){validate_identifier(key)?;}Ok(())}
fn quote_identifier(v:&str)->String{format!("\"{}\"",v.replace('"',"\"\""))}
fn validate_identifier(v:&str)->Result<(),DatabaseError>{if v.is_empty()||v.len()>255||v.contains('\0'){Err(DatabaseError::Invalid("unsafe SQLite identifier".into()))}else{Ok(())}}
fn validate_read_query(v:&str)->Result<(),DatabaseError>{let trimmed=v.trim();if trimmed.is_empty()||trimmed.len()>1024*1024{return Err(DatabaseError::Invalid("query is empty or too large".into()));}let no_trailing=trimmed.trim_end_matches(';').trim_end();if no_trailing.contains(';'){return Err(DatabaseError::Invalid("multiple SQL statements are not allowed".into()));}let upper=no_trailing.trim_start().to_ascii_uppercase();if upper.starts_with("SELECT ")||upper=="SELECT"{return Ok(());}if upper.starts_with("EXPLAIN SELECT ")||upper.starts_with("EXPLAIN QUERY PLAN SELECT "){return Ok(());}if upper.starts_with("PRAGMA "){let risky=["=","JOURNAL_MODE","WAL_CHECKPOINT","OPTIMIZE","VACUUM","FOREIGN_KEYS","SYNCHRONOUS","LOCKING_MODE"];if risky.iter().any(|x|upper.contains(x)){return Err(DatabaseError::Invalid("mutating/operational PRAGMA is not allowed through read query".into()));}return Ok(());}Err(DatabaseError::Invalid("read query accepts SELECT, EXPLAIN SELECT, or non-mutating PRAGMA only".into()))}
fn db_err<E:std::fmt::Display>(e:E)->DatabaseError{DatabaseError::Provider(e.to_string())}

#[cfg(test)] mod tests{
    use super::*;
    use std::time::{SystemTime,UNIX_EPOCH};
    #[test]fn read_only_gate_blocks_mutation_and_with(){assert!(validate_read_query("DELETE FROM users").is_err());assert!(validate_read_query("WITH x AS (SELECT 1) SELECT * FROM x").is_err());assert!(validate_read_query("SELECT * FROM users").is_ok());}
    #[test]fn sqlite_affinity_mapping(){assert_eq!(sqlite_type("INTEGER"),FieldType::Integer);assert_eq!(sqlite_type("TEXT"),FieldType::Text);}
    #[test]fn update_delete_require_filter(){let req=MutationRequest{values:BTreeMap::new(),filter:BTreeMap::new()};assert!(req.filter.is_empty());}
    #[test]fn crud_browse_indexes_relations_and_stats_roundtrip(){
        let stamp=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path=std::env::temp_dir().join(format!("vsn-sqlite-{stamp}.db"));
        {
            let conn=Connection::open(&path).unwrap();
            conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE teams(id INTEGER PRIMARY KEY,name TEXT NOT NULL); CREATE TABLE users(id INTEGER PRIMARY KEY,name TEXT NOT NULL,team_id INTEGER REFERENCES teams(id)); CREATE UNIQUE INDEX idx_users_name ON users(name); INSERT INTO teams(name) VALUES('core');").unwrap();
        }
        let provider=SqliteProvider::open(&path,false).unwrap();
        let mut insert_values=BTreeMap::new();insert_values.insert("name".into(),json!("Ada"));insert_values.insert("team_id".into(),json!(1));
        let inserted=provider.insert(None,"users",&MutationRequest{values:insert_values,filter:BTreeMap::new()}).unwrap();assert_eq!(inserted.affected_rows,1);
        let page=provider.browse(None,"users",&BrowseRequest{limit:50,offset:0,order_by:Some("id".into()),descending:false}).unwrap();assert_eq!(page.rows.len(),1);assert_eq!(page.total_rows,Some(1));
        let indexes=provider.list_indexes(None,"users").unwrap();assert!(indexes.iter().any(|i|i.name=="idx_users_name"&&i.unique));
        let relations=provider.list_relations(None,"users").unwrap();assert!(relations.iter().any(|r|r.to_entity=="teams"&&r.from_fields==vec!["team_id"]));
        let stats=provider.statistics(None,"users").unwrap();assert_eq!(stats.row_count,Some(1));
        let mut values=BTreeMap::new();values.insert("name".into(),json!("Ada Lovelace"));let mut filter=BTreeMap::new();filter.insert("id".into(),json!(1));provider.update(None,"users",&MutationRequest{values,filter:filter.clone()}).unwrap();
        let page=provider.browse(None,"users",&BrowseRequest{limit:10,offset:0,order_by:None,descending:false}).unwrap();assert_eq!(page.rows[0].get("name"),Some(&json!("Ada Lovelace")));
        provider.delete(None,"users",&MutationRequest{values:BTreeMap::new(),filter}).unwrap();assert_eq!(provider.statistics(None,"users").unwrap().row_count,Some(0));
        drop(provider);let _=std::fs::remove_file(path);
    }
}
