use serde_json::Value;
#[tauri::command]
fn agent_call(command:String,params:Value)->Result<Value,String>{let response=vsn_ipc::call(&command,params).map_err(|e|e.to_string())?;if response.ok{Ok(response.payload)}else{Err(response.payload.to_string())}}
fn main(){tauri::Builder::default().invoke_handler(tauri::generate_handler![agent_call]).run(tauri::generate_context!()).expect("error while running VSN desktop")}
