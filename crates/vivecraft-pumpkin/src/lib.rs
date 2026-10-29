//! Pumpkin WIT v0.1 adapter. All host calls happen outside session locks.
mod commands;
mod events;

use pumpkin_plugin_api::wit::pumpkin::plugin::{player::Player, uuid::Uuid};
use pumpkin_plugin_api::{Context, Plugin, PluginMetadata, Server, register_plugin};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use vivecraft_core::{
    Effects, PlayerId, Sessions,
    config::{self, Settings},
};

pub(crate) struct State {
    sessions: Sessions,
    generations: BTreeMap<PlayerId, u64>,
    next_generation: u64,
    tick: u64,
    last_warning: BTreeMap<PlayerId, u64>,
    folder: PathBuf,
    active: bool,
    config_busy: bool,
}
pub(crate) type Shared = Arc<Mutex<State>>;
struct Vivecraft {
    shared: Shared,
}
impl Plugin for Vivecraft {
    fn new() -> Self {
        Self {
            shared: Arc::new(Mutex::new(State {
                sessions: Sessions::new(Settings::default()),
                generations: BTreeMap::new(),
                next_generation: 0,
                tick: 0,
                last_warning: BTreeMap::new(),
                folder: PathBuf::new(),
                active: false,
                config_busy: false,
            })),
        }
    }
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "Vivecraft Pumpkin".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Vivecraft contributors".into()],
            description: "Vivecraft sessions preview; VR gameplay parity is incomplete".into(),
            dependencies: vec![],
            permissions: vec![
                pumpkin_plugin_api::permissions::FS_READ_DATA.into(),
                pumpkin_plugin_api::permissions::FS_WRITE_DATA.into(),
            ],
        }
    }
    fn on_load(&self, context: Context) -> Result<(), String> {
        let folder = PathBuf::from(context.get_data_folder());
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let path = folder.join("config.toml");
        let settings = if path.exists() {
            Settings::parse(&config::read_bounded(&path)?)?
        } else {
            let settings = Settings::default();
            config::save_atomic(&folder, &settings)?;
            settings
        };
        {
            let mut state = self.shared.lock().map_err(|e| e.to_string())?;
            state.sessions = Sessions::new(settings);
            state.folder = folder;
            state.active = true;
        }
        commands::register(&context, self.shared.clone())?;
        events::register(&context, self.shared.clone())?;
        Ok(())
    }
    fn on_unload(&self, context: Context) -> Result<(), String> {
        let effects = {
            let mut state = self.shared.lock().map_err(|e| e.to_string())?;
            state.active = false;
            let effects = state.sessions.clear();
            state.generations.clear();
            state.last_warning.clear();
            effects
        };
        // The host owns event/command registrations and disposes them with the instance.
        // Clear notifications are best-effort during host teardown.
        deliver_unchecked(&context.get_server(), effects);
        Ok(())
    }
    fn handle_ipc_message(&self, _sender: String, message: Vec<u8>) -> Result<Vec<u8>, String> {
        if message.len() > 4096 {
            return Err("IPC request too large".into());
        }
        let request: serde_json::Value =
            serde_json::from_slice(&message).map_err(|e| e.to_string())?;
        if request.get("version").and_then(|v| v.as_u64()) != Some(1)
            || request.get("query").and_then(|v| v.as_str()) != Some("sessions")
        {
            return Err("supported request: {\"version\":1,\"query\":\"sessions\"}".into());
        }
        // Read-only preview contract, not a completed replacement for VRAPI/placeholder providers.
        let snapshots = self
            .shared
            .lock()
            .map_err(|e| e.to_string())?
            .sessions
            .snapshots();
        serde_json::to_vec(&serde_json::json!({"version":1,"preview":true,"sessions":snapshots}))
            .map_err(|e| e.to_string())
    }
}
register_plugin!(Vivecraft);

pub(crate) fn id(player: &Player) -> PlayerId {
    let uuid = player.get_id();
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&uuid.high.to_be_bytes());
    bytes[8..].copy_from_slice(&uuid.low.to_be_bytes());
    bytes
}
fn uuid(id: PlayerId) -> Uuid {
    let mut high = [0; 8];
    let mut low = [0; 8];
    high.copy_from_slice(&id[..8]);
    low.copy_from_slice(&id[8..]);
    Uuid {
        high: u64::from_be_bytes(high),
        low: u64::from_be_bytes(low),
    }
}
pub(crate) fn deliver(server: &Server, shared: &Shared, effects: Effects) {
    for outgoing in effects.outgoing {
        let valid = shared.lock().is_ok_and(|state| {
            state.active
                && state.generations.get(&outgoing.recipient) == Some(&outgoing.generation)
                && state.sessions.can_deliver(&outgoing)
        });
        if !valid {
            continue;
        }
        if let Some(player) = server.get_player_by_uuid(uuid(outgoing.recipient))
            && let Some(java) = player.as_java()
        {
            java.send_custom_payload("vivecraft:data", &outgoing.bytes);
        }
    }
}
fn deliver_unchecked(server: &Server, effects: Effects) {
    for outgoing in effects.outgoing {
        if let Some(player) = server.get_player_by_uuid(uuid(outgoing.recipient))
            && let Some(java) = player.as_java()
        {
            java.send_custom_payload("vivecraft:data", &outgoing.bytes);
        }
    }
}
