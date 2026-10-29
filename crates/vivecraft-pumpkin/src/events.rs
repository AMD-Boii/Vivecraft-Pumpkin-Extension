use crate::{Shared, deliver, id, uuid};
use pumpkin_plugin_api::events::{
    EventData, EventHandler, EventPriority, PlayerChangedWorldEvent, PlayerCustomPayloadEvent,
    PlayerJoinEvent, PlayerLeaveEvent, PlayerPermissionCheckEvent, PlayerRespawnEvent,
    ServerTickEndEvent,
};
use pumpkin_plugin_api::wit::pumpkin::plugin::{
    logging::{self as log, Level},
    player::Player,
};
use pumpkin_plugin_api::{Context as HostContext, Server};
use std::collections::BTreeSet;
use vivecraft_core::{Context, Effects};

struct Handler(Shared);
pub(crate) fn register(context: &HostContext, shared: Shared) -> Result<(), String> {
    context.register_event_handler::<PlayerPermissionCheckEvent, _>(
        Handler(shared.clone()),
        EventPriority::Normal,
        true,
    )?;
    context.register_event_handler::<PlayerCustomPayloadEvent, _>(
        Handler(shared.clone()),
        EventPriority::Normal,
        true,
    )?;
    context.register_event_handler::<PlayerJoinEvent, _>(
        Handler(shared.clone()),
        EventPriority::Lowest,
        true,
    )?;
    context.register_event_handler::<PlayerLeaveEvent, _>(
        Handler(shared.clone()),
        EventPriority::Lowest,
        true,
    )?;
    context.register_event_handler::<PlayerRespawnEvent, _>(
        Handler(shared.clone()),
        EventPriority::Lowest,
        true,
    )?;
    context.register_event_handler::<PlayerChangedWorldEvent, _>(
        Handler(shared.clone()),
        EventPriority::Lowest,
        true,
    )?;
    context.register_event_handler::<ServerTickEndEvent, _>(
        Handler(shared),
        EventPriority::Normal,
        true,
    )?;
    Ok(())
}
fn host_snapshot(server: &Server, player: &Player, generation: u64, tick: u64) -> Context {
    let position = player.get_position();
    Context {
        id: id(player),
        generation,
        world: player.get_world().get_name(),
        position: [position.0, position.1, position.2],
        tick,
        view_distance: server.get_view_distance(),
        visible: BTreeSet::new(),
    }
}
impl EventHandler<PlayerPermissionCheckEvent> for Handler {
    fn handle(
        &self,
        _server: Server,
        mut event: EventData<PlayerPermissionCheckEvent>,
    ) -> EventData<PlayerPermissionCheckEvent> {
        if vivecraft_core::config::PERMISSIONS.contains(&event.permission.as_str()) {
            // Respect an explicit legacy grant OR denial. Only fall back to the
            // namespaced default when the legacy node has no explicit attachment.
            if let Some(value) = event.player.has_permission_set(&event.permission) {
                event.permission_result = value;
            } else if !event.permission_result {
                // The recursive permission event uses a different node, so this
                // handler leaves it alone. No session lock crosses this host call.
                event.permission_result = event
                    .player
                    .has_permission(&crate::commands::native_permission(&event.permission));
            }
        }
        event
    }
}
impl EventHandler<PlayerCustomPayloadEvent> for Handler {
    fn handle(
        &self,
        server: Server,
        event: EventData<PlayerCustomPayloadEvent>,
    ) -> EventData<PlayerCustomPayloadEvent> {
        if event.channel != "vivecraft:data" || event.player.as_java().is_none() {
            return event;
        }
        let player_id = id(&event.player);
        let (generation, tick) = {
            let Ok(mut state) = self.0.lock() else {
                return event;
            };
            if !state.active {
                return event;
            }
            if !state.generations.contains_key(&player_id) {
                state.next_generation = state.next_generation.saturating_add(1);
                let next = state.next_generation;
                state.generations.insert(player_id, next);
            }
            (state.generations[&player_id], state.tick)
        };
        let context = host_snapshot(&server, &event.player, generation, tick);
        let (effects, warning) = {
            let Ok(mut state) = self.0.lock() else {
                return event;
            };
            match state
                .sessions
                .receive(&context, &event.channel, &event.data)
            {
                Ok(effects) => (effects, None),
                Err(error) => {
                    let previous = state.last_warning.get(&player_id).copied();
                    let warning = if previous.is_none_or(|last| tick.saturating_sub(last) >= 100) {
                        state.last_warning.insert(player_id, tick);
                        Some(error)
                    } else {
                        None
                    };
                    (Effects::default(), warning)
                }
            }
        };
        if let Some(warning) = warning {
            log::log(
                Level::Warn,
                &format!("Vivecraft payload rejected: {warning}"),
            );
        }
        deliver(&server, &self.0, effects);
        event
    }
}
impl EventHandler<PlayerJoinEvent> for Handler {
    fn handle(
        &self,
        server: Server,
        event: EventData<PlayerJoinEvent>,
    ) -> EventData<PlayerJoinEvent> {
        let player_id = id(&event.player);
        let effects = if let Ok(mut state) = self.0.lock() {
            let effects = state.sessions.remove(&player_id);
            state.next_generation = state.next_generation.saturating_add(1);
            let generation = state.next_generation;
            state.generations.insert(player_id, generation);
            effects
        } else {
            Effects::default()
        };
        deliver(&server, &self.0, effects);
        event
    }
}
impl EventHandler<PlayerLeaveEvent> for Handler {
    fn handle(
        &self,
        server: Server,
        event: EventData<PlayerLeaveEvent>,
    ) -> EventData<PlayerLeaveEvent> {
        let player_id = id(&event.player);
        let effects = if let Ok(mut state) = self.0.lock() {
            let effects = state.sessions.remove(&player_id);
            state.generations.remove(&player_id);
            state.last_warning.remove(&player_id);
            effects
        } else {
            Effects::default()
        };
        deliver(&server, &self.0, effects);
        event
    }
}
macro_rules! reset_handler {
    ($event:ty) => {
        impl EventHandler<$event> for Handler {
            fn handle(&self, server: Server, event: EventData<$event>) -> EventData<$event> {
                let player_id = id(&event.player);
                let effects = self
                    .0
                    .lock()
                    .map(|mut state| state.sessions.reset(&player_id))
                    .unwrap_or_default();
                deliver(&server, &self.0, effects);
                event
            }
        }
    };
}
reset_handler!(PlayerRespawnEvent);
reset_handler!(PlayerChangedWorldEvent);
impl EventHandler<ServerTickEndEvent> for Handler {
    fn handle(
        &self,
        server: Server,
        event: EventData<ServerTickEndEvent>,
    ) -> EventData<ServerTickEndEvent> {
        let (tick, generations, subjects) = {
            let Ok(mut state) = self.0.lock() else {
                return event;
            };
            if !state.active {
                return event;
            }
            state.tick = state.tick.saturating_add(1);
            (
                state.tick,
                state.generations.clone(),
                state.sessions.snapshots(),
            )
        };
        let mut contexts = Vec::new();
        for player in server.get_all_players().into_iter().take(1024) {
            if player.as_java().is_none() {
                continue;
            }
            let Some(generation) = generations.get(&id(&player)) else {
                continue;
            };
            let mut snapshot = host_snapshot(&server, &player, *generation, tick);
            for subject in &subjects {
                if let Some(other) = server.get_player_by_uuid(uuid(subject.id))
                    && player.can_see(other)
                {
                    snapshot.visible.insert(subject.id);
                }
            }
            contexts.push(snapshot);
        }
        let effects = self
            .0
            .lock()
            .map(|mut state| state.sessions.tick(&contexts))
            .unwrap_or_default();
        deliver(&server, &self.0, effects);
        event
    }
}
