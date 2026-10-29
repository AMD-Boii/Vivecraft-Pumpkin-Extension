use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{
    config::Settings,
    pose::VrPose,
    protocol::{self, Packet},
};
use serde::{Deserialize, Serialize};

pub type PlayerId = [u8; 16];

/// Authoritative host context, never a retained Pumpkin resource handle.
#[derive(Clone, Debug)]
pub struct Context {
    pub id: PlayerId,
    pub generation: u64,
    pub world: String,
    pub position: [f64; 3],
    pub tick: u64,
    pub view_distance: u8,
    /// Subjects this observer can see, resolved by the adapter before entering sessions.
    pub visible: BTreeSet<PlayerId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub id: PlayerId,
    pub generation: u64,
    pub client_version: String,
    pub network_version: i32,
    pub vr: bool,
    pub pose: Option<VrPose>,
    pub world_scale: f32,
    pub height_scale: f32,
    pub active_body_part: u8,
    pub use_body_part_for_aim: bool,
    pub draw: f32,
    pub aim_position: Option<[f32; 3]>,
    pub aim_direction: Option<[f32; 3]>,
}
#[derive(Clone, Debug)]
pub struct Outgoing {
    pub recipient: PlayerId,
    pub generation: u64,
    pub bytes: Vec<u8>,
    /// Pose effects expire when the source session ends or leaves VR.
    pub subject: Option<(PlayerId, u64)>,
}
#[derive(Default, Debug)]
pub struct Effects {
    pub outgoing: Vec<Outgoing>,
    pub changes: Vec<PlayerId>,
    pub diagnostics: Vec<String>,
}
struct Session {
    data: SessionSnapshot,
    history: VecDeque<(u64, VrPose)>,
    recipients: BTreeMap<PlayerId, u64>,
    last_tick: Option<u64>,
    aim_deadline: Option<u64>,
    legacy: crate::legacy::LegacyFrame,
}

/// The public test/consumer interface for protocol, state, relay, and policy.
pub struct Sessions {
    settings: Settings,
    sessions: BTreeMap<PlayerId, Session>,
}
impl Sessions {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            sessions: BTreeMap::new(),
        }
    }
    pub fn snapshot(&self, id: &PlayerId) -> Option<SessionSnapshot> {
        self.sessions.get(id).map(|s| s.data.clone())
    }
    pub fn snapshots(&self) -> Vec<SessionSnapshot> {
        self.sessions.values().map(|s| s.data.clone()).collect()
    }
    pub fn history(&self, id: &PlayerId, ticks_back: usize) -> Option<VrPose> {
        self.sessions
            .get(id)
            .filter(|s| s.data.vr)?
            .history
            .get(ticks_back)
            .map(|(_, p)| p.clone())
    }
    pub fn can_deliver(&self, outgoing: &Outgoing) -> bool {
        outgoing.subject.is_none_or(|(id, generation)| {
            self.sessions.get(&id).is_some_and(|s| {
                s.data.generation == generation && s.data.vr && s.data.pose.is_some()
            })
        })
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub fn configure(&mut self, settings: Settings) -> Effects {
        self.settings = settings;
        let mut effects = Effects::default();
        for session in self.sessions.values() {
            self.handshake(&session.data, &mut effects);
        }
        effects
    }
    fn send(data: &SessionSnapshot, bytes: Vec<u8>, effects: &mut Effects) {
        effects.outgoing.push(Outgoing {
            recipient: data.id,
            generation: data.generation,
            bytes,
            subject: None,
        });
    }
    fn handshake(&self, data: &SessionSnapshot, effects: &mut Effects) {
        Self::send(data, protocol::server_version(), effects);
        if self.settings.boolean("general.requestData") {
            Self::send(data, vec![1], effects);
        }
        // OPTION_TOGGLE is required by the runtime so these false values can be represented honestly.
        for id in [9, 10, 14, 19, 23] {
            Self::send(data, vec![id, 0], effects);
        }
        Self::send(
            data,
            vec![16, u8::from(self.settings.boolean("vrSwitching.enabled"))],
            effects,
        );
        Self::send(data, vec![15, data.network_version as u8], effects);
    }
    pub fn receive(
        &mut self,
        context: &Context,
        channel: &str,
        bytes: &[u8],
    ) -> Result<Effects, String> {
        if channel != "vivecraft:data" {
            return Ok(Effects::default());
        }
        if !context.position.iter().all(|n| n.is_finite()) {
            return Err("invalid host position".into());
        }
        let packet = protocol::decode(bytes)?;
        if let Packet::Version { text, vr, min, max } = packet {
            let negotiated = max.min(protocol::MAX_VERSION);
            // Historical codecs are retained, but older runtimes cannot explicitly disable teleport.
            if negotiated < min || negotiated < protocol::OPTION_TOGGLE {
                return Err("sessions preview requires Vivecraft wire protocol 7..10; historical codecs are fixture-only".into());
            }
            if !self.sessions.contains_key(&context.id) && self.sessions.len() >= 1024 {
                return Err("session limit reached".into());
            }
            let mut effects = self.remove(&context.id);
            let data = SessionSnapshot {
                id: context.id,
                generation: context.generation,
                client_version: text,
                network_version: negotiated,
                vr,
                pose: None,
                world_scale: 1.,
                height_scale: 1.,
                active_body_part: 0,
                use_body_part_for_aim: false,
                draw: 0.,
                aim_position: None,
                aim_direction: None,
            };
            self.handshake(&data, &mut effects);
            self.sessions.insert(
                context.id,
                Session {
                    data,
                    history: VecDeque::new(),
                    recipients: BTreeMap::new(),
                    last_tick: None,
                    aim_deadline: None,
                    legacy: crate::legacy::LegacyFrame::default(),
                },
            );
            effects.changes.push(context.id);
            return Ok(effects);
        }
        let switching = self.settings.boolean("vrSwitching.enabled");
        let session = self
            .sessions
            .get_mut(&context.id)
            .ok_or("VERSION must be received first")?;
        if session.data.generation != context.generation {
            return Err("stale connection generation".into());
        }
        let mut effects = Effects::default();
        match packet {
            Packet::Active(vr) => {
                if session.data.vr != vr {
                    if !switching {
                        return Err("VR switching is disabled".into());
                    }
                    if !vr {
                        Self::clear_pose(session, &mut effects);
                    }
                    session.data.vr = vr;
                    effects.changes.push(context.id);
                }
            }
            Packet::Pose(pose) => {
                if !session.data.vr {
                    return Err("pose from inactive player".into());
                }
                session.data.pose = Some(pose);
                effects.changes.push(context.id);
            }
            Packet::Scale(world, value) => {
                if world {
                    session.data.world_scale = value;
                } else {
                    session.data.height_scale = value;
                }
            }
            Packet::Draw(draw) => session.data.draw = draw,
            Packet::BodyPart(body, aim) => {
                session.data.active_body_part =
                    if session.data.pose.as_ref().is_some_and(|p| p.seated) && body != 9 {
                        0
                    } else {
                        body
                    };
                session.data.use_body_part_for_aim = aim;
            }
            Packet::AimPosition(pos) => session.data.aim_position = Some(pos),
            Packet::AimDirection(dir) => session.data.aim_direction = Some(dir),
            Packet::AimReset(ticks) => {
                if ticks > 127 {
                    return Err("negative aim reset duration".into());
                }
                session.aim_deadline = if ticks == 0 {
                    None
                } else {
                    Some(context.tick.saturating_add(u64::from(ticks)))
                };
                if ticks == 0 {
                    session.data.aim_position = None;
                    session.data.aim_direction = None;
                }
            }
            Packet::Gameplay(id) => {
                return Err(format!(
                    "gameplay payload {id} is not enabled in the sessions milestone"
                ));
            }
            Packet::Legacy(id, flag, pose) => {
                if session.data.network_version >= 0 {
                    return Err(
                        "legacy pose fragments are not valid for this negotiated runtime".into(),
                    );
                }
                // Retained historical assembly; current admission requires explicit option toggles.
                // Historical Minecraft-version classification remains a host admission concern.
                if let Some(pose) =
                    session
                        .legacy
                        .push(id, flag, pose, context.tick, false, context.position)
                {
                    session.data.pose = Some(pose);
                }
            }
            Packet::DamageDirection => {}
            Packet::Version { .. } => unreachable!(),
        }
        Ok(effects)
    }
    fn clear_pose(session: &mut Session, effects: &mut Effects) {
        for (recipient, generation) in std::mem::take(&mut session.recipients) {
            effects.outgoing.push(Outgoing {
                recipient,
                generation,
                bytes: protocol::inactive(session.data.id),
                subject: None,
            });
        }
        session.data.pose = None;
        session.history.clear();
        session.last_tick = None;
        session.data.draw = 0.;
        session.data.aim_direction = None;
        session.data.aim_position = None;
        session.aim_deadline = None;
    }
    pub fn remove(&mut self, id: &PlayerId) -> Effects {
        let mut effects = Effects::default();
        if let Some(mut session) = self.sessions.remove(id) {
            Self::clear_pose(&mut session, &mut effects);
            effects.changes.push(*id);
        }
        for session in self.sessions.values_mut() {
            session.recipients.remove(id);
        }
        effects
    }
    pub fn reset(&mut self, id: &PlayerId) -> Effects {
        self.remove(id)
    }
    pub fn clear(&mut self) -> Effects {
        let mut effects = Effects::default();
        for session in self.sessions.values_mut() {
            Self::clear_pose(session, &mut effects);
        }
        self.sessions.clear();
        effects
    }
    pub fn tick(&mut self, contexts: &[Context]) -> Effects {
        let contexts: BTreeMap<_, _> = contexts.iter().map(|c| (c.id, c)).collect();
        let negotiated: BTreeSet<_> = self
            .sessions
            .iter()
            .filter_map(|(id, s)| {
                contexts
                    .get(id)
                    .filter(|c| c.generation == s.data.generation)
                    .map(|_| *id)
            })
            .collect();
        let mut effects = Effects::default();
        let stale: Vec<_> = self
            .sessions
            .iter()
            .filter(|(id, s)| {
                contexts
                    .get(*id)
                    .is_none_or(|c| c.generation != s.data.generation)
            })
            .map(|(id, _)| *id)
            .collect();
        for id in stale {
            let removed = self.remove(&id);
            effects.outgoing.extend(removed.outgoing);
            effects.changes.extend(removed.changes);
        }
        for session in self.sessions.values_mut() {
            let Some(subject) = contexts.get(&session.data.id) else {
                continue;
            };
            if session.last_tick == Some(subject.tick) {
                continue;
            }
            session.last_tick = Some(subject.tick);
            if session
                .aim_deadline
                .is_some_and(|deadline| subject.tick >= deadline)
            {
                session.data.aim_direction = None;
                session.data.aim_position = None;
                session.aim_deadline = None;
            }
            let mut recipients = BTreeMap::new();
            if session.data.vr
                && let Some(pose) = &session.data.pose
            {
                session.history.push_front((subject.tick, pose.clone()));
                session.history.truncate(201);
                if self.settings.boolean("general.sendData") {
                    let radius = (f64::from(subject.view_distance) + 1.) * 16.;
                    for observer in contexts.values() {
                        let owner = observer.id == subject.id;
                        let distance: f64 = subject
                            .position
                            .iter()
                            .zip(observer.position)
                            .map(|(a, b)| (a - b).powi(2))
                            .sum();
                        if !negotiated.contains(&observer.id)
                            || (!owner
                                && (observer.world != subject.world
                                    || distance >= radius * radius
                                    || !observer.visible.contains(&subject.id)))
                            || (owner && !self.settings.boolean("general.sendDataToOwner"))
                        {
                            continue;
                        }
                        recipients.insert(observer.id, observer.generation);
                    }
                }
            }
            for (id, generation) in &session.recipients {
                if recipients.get(id) != Some(generation) {
                    effects.outgoing.push(Outgoing {
                        recipient: *id,
                        generation: *generation,
                        bytes: protocol::inactive(subject.id),
                        subject: None,
                    });
                }
            }
            if let Some(pose) = &session.data.pose {
                let bytes = protocol::relay(
                    subject.id,
                    pose,
                    session.data.world_scale,
                    session.data.height_scale,
                    protocol::MAX_VERSION,
                    None,
                );
                for (id, generation) in &recipients {
                    effects.outgoing.push(Outgoing {
                        recipient: *id,
                        generation: *generation,
                        bytes: bytes.clone(),
                        subject: Some((subject.id, session.data.generation)),
                    });
                }
            }
            session.recipients = recipients;
        }
        effects
    }
}
