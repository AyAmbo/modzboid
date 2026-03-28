use serde::{Deserialize, Serialize};
use crate::app_core::error::AppError;
use super::client;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RconResponse {
    pub success: bool,
    pub response: String,
}

/// Connect to RCON, execute a command, and return the response.
#[tauri::command]
pub async fn rcon_command_cmd(
    host: String,
    port: u16,
    password: String,
    command: String,
) -> Result<RconResponse, AppError> {
    let mut stream = client::connect_and_auth(&host, port, &password).await?;
    let response = client::send_command(&mut stream, &command).await?;
    Ok(RconResponse {
        success: true,
        response,
    })
}

/// Test RCON connection (auth only).
#[tauri::command]
pub async fn rcon_test_cmd(
    host: String,
    port: u16,
    password: String,
) -> Result<RconResponse, AppError> {
    let _stream = client::connect_and_auth(&host, port, &password).await?;
    Ok(RconResponse {
        success: true,
        response: "Connected successfully".into(),
    })
}
