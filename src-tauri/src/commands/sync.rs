//! Commands for the sync screens: the PC's "Phone" dialog and the phone's sync page.

use tauri::State;

use crate::{
    sync::{Invite, Node, Status},
    SyncState,
};

fn node(state: &State<'_, SyncState>) -> Result<Node, String> {
    match state.0.lock().unwrap().as_ref() {
        Some(Ok(node)) => Ok(node.clone()),
        Some(Err(error)) => Err(error.clone()),
        None => Err("Phone sync is still starting.".into()),
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncInfo {
    phone: bool,
    status: Option<Status>,
    error: Option<String>,
}

#[tauri::command]
pub fn sync_status(state: State<'_, SyncState>) -> SyncInfo {
    let phone = crate::is_phone();
    match node(&state) {
        Ok(node) => SyncInfo { phone, status: Some(node.status()), error: None },
        Err(error) => SyncInfo { phone, status: None, error: Some(error) },
    }
}

#[tauri::command]
pub fn sync_invite(state: State<'_, SyncState>) -> Result<Invite, String> {
    node(&state)?.create_invite().map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub async fn sync_use_code(code: String, state: State<'_, SyncState>) -> Result<String, String> {
    let node = node(&state)?;
    node.use_code(&code).await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
pub fn sync_remove_device(id: String, state: State<'_, SyncState>) -> Result<(), String> {
    node(&state)?.remove_device(&id).map_err(|e| format!("{e:#}"))
}

/// Phone: check the PC now (also after the app returns to the foreground, when the network
/// may have changed).
#[tauri::command]
pub async fn sync_now(state: State<'_, SyncState>) -> Result<(), String> {
    node(&state)?.network_changed().await;
    Ok(())
}
