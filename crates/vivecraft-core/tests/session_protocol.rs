use std::collections::BTreeSet;
use vivecraft_core::{Context, FbtMode, Sessions, config::Settings};

fn context(n: u8) -> Context {
    Context {
        id: [n; 16],
        generation: u64::from(n),
        world: "world".into(),
        position: [0.; 3],
        tick: 1,
        view_distance: 10,
        visible: BTreeSet::from([[1; 16], [2; 16]]),
    }
}
fn vector(name: &str) -> Vec<u8> {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/vectors.json")).unwrap();
    let text = vectors[name].as_str().unwrap();
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
fn connect(sessions: &mut Sessions, context: &Context) {
    sessions
        .receive(context, "vivecraft:data", &vector("version_10"))
        .unwrap();
}
#[test]
fn negotiation_has_explicit_disabled_gameplay() {
    let mut sessions = Sessions::new(Settings::default());
    let effects = sessions
        .receive(&context(1), "vivecraft:data", &vector("version_10"))
        .unwrap();
    for id in [9, 10, 14, 19, 23] {
        assert!(effects.outgoing.iter().any(|p| p.bytes == vec![id, 0]));
    }
    assert_eq!(effects.outgoing.last().unwrap().bytes, vec![15, 10]);
    assert!(!effects.outgoing.iter().any(|p| p.bytes[0] == 28));
    assert_eq!(sessions.snapshot(&[1; 16]).unwrap().network_version, 10);
}
#[test]
fn version_boundaries_and_legacy_rejection_are_honest() {
    for version in [7, 8, 9, 10] {
        let mut sessions = Sessions::new(Settings::default());
        let bytes = format!("\0client VR\n{version}\n0").into_bytes();
        sessions
            .receive(&context(1), "vivecraft:data", &bytes)
            .unwrap();
        assert_eq!(
            sessions.snapshot(&[1; 16]).unwrap().network_version,
            version
        );
    }
    for bytes in [
        vector("version_legacy"),
        b"\0client VR\n6\n0".to_vec(),
        b"\0client VR\n11\n11".to_vec(),
        b"\0client VR\n2\n3".to_vec(),
    ] {
        let mut sessions = Sessions::new(Settings::default());
        assert!(
            sessions
                .receive(&context(1), "vivecraft:data", &bytes)
                .is_err()
        );
        assert!(sessions.snapshot(&[1; 16]).is_none());
    }
}
#[test]
fn independently_generated_pose_and_relay_vectors() {
    for (name, mode, trackers) in [
        ("pose_arms", FbtMode::ArmsOnly, 0),
        ("pose_feet", FbtMode::WithFeet, 3),
        ("pose_joints", FbtMode::WithJoints, 7),
    ] {
        let mut sessions = Sessions::new(Settings::default());
        connect(&mut sessions, &context(1));
        connect(&mut sessions, &context(2));
        sessions
            .receive(&context(1), "vivecraft:data", &vector(name))
            .unwrap();
        let pose = sessions.snapshot(&[1; 16]).unwrap().pose.unwrap();
        assert_eq!(pose.mode, mode);
        assert_eq!(pose.trackers.len(), trackers);
        assert_eq!(pose.head.position, [1., 2., 3.]);
        assert_eq!(pose.head.orientation, [1., 0., 0., 0.]);
        assert!(pose.left_handed);
        let effects = sessions.tick(&[context(1), context(2)]);
        let relay = effects
            .outgoing
            .iter()
            .find(|p| p.recipient == [2; 16])
            .unwrap();
        if name == "pose_arms" {
            assert_eq!(relay.bytes, vector("relay_arms"));
        }
    }
}
#[test]
fn lifecycle_visibility_range_and_history() {
    let mut sessions = Sessions::new(Settings::default());
    let mut a = context(1);
    let mut b = context(2);
    connect(&mut sessions, &a);
    connect(&mut sessions, &b);
    sessions
        .receive(&a, "vivecraft:data", &vector("pose_arms"))
        .unwrap();
    assert_eq!(sessions.tick(&[a.clone(), b.clone()]).outgoing.len(), 1);
    assert!(sessions.tick(&[a.clone(), b.clone()]).outgoing.is_empty());
    a.tick += 1;
    b.tick += 1;
    b.visible.clear();
    let inactive = sessions.tick(&[a.clone(), b.clone()]);
    assert_eq!(&inactive.outgoing[0].bytes[..2], &[17, 0]);
    a.tick += 1;
    b.tick += 1;
    b.visible.insert(a.id);
    b.position = [176., 0., 0.];
    assert!(sessions.tick(&[a.clone(), b.clone()]).outgoing.is_empty());
    b.position = [175.9, 0., 0.];
    a.tick += 1;
    b.tick += 1;
    assert_eq!(sessions.tick(&[a.clone(), b.clone()]).outgoing.len(), 1);
    b.world = "nether".into();
    a.tick += 1;
    b.tick += 1;
    assert_eq!(
        &sessions.tick(&[a.clone(), b.clone()]).outgoing[0].bytes[..2],
        &[17, 0]
    );
    for tick in 10..240 {
        a.tick = tick;
        b.tick = tick;
        sessions.tick(&[a.clone(), b.clone()]);
    }
    assert!(sessions.history(&a.id, 200).is_some());
    assert!(sessions.history(&a.id, 201).is_none());
    sessions.receive(&a, "vivecraft:data", &[17, 0]).unwrap();
    assert!(sessions.history(&a.id, 0).is_none());
    assert!(sessions.snapshot(&a.id).unwrap().pose.is_none());
    sessions.reset(&a.id);
    assert!(sessions.snapshot(&a.id).is_none());
    connect(&mut sessions, &a);
    let mut stale = a.clone();
    stale.generation += 1;
    assert!(
        sessions
            .receive(&stale, "vivecraft:data", &vector("pose_arms"))
            .is_err()
    );
    sessions.clear();
    assert!(sessions.snapshots().is_empty());
}
#[test]
fn malformed_input_does_not_mutate_session() {
    let mut sessions = Sessions::new(Settings::default());
    let a = context(1);
    assert!(
        sessions
            .receive(&a, "vivecraft:data", &vector("pose_arms"))
            .is_err()
    );
    connect(&mut sessions, &a);
    let original = serde_json::to_value(sessions.snapshot(&a.id)).unwrap();
    for bytes in [
        vec![],
        vec![255],
        vec![5, 0x7f, 0x80, 0, 0],
        vec![13, 255],
        vec![17, 2],
        vec![25, 255],
        vec![18, 0],
    ] {
        assert!(sessions.receive(&a, "vivecraft:data", &bytes).is_err());
        assert_eq!(
            original,
            serde_json::to_value(sessions.snapshot(&a.id)).unwrap()
        );
    }
    let valid = vector("pose_joints");
    // Arms-only is itself a valid historical frame; other truncations must not panic.
    for len in 0..valid.len() {
        let _ = sessions.receive(&a, "vivecraft:data", &valid[..len]);
    }
    let mut seed = 1u64;
    for _ in 0..2048 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut bytes = vec![0; (seed as usize) % 400];
        for byte in &mut bytes {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            *byte = (seed >> 32) as u8;
        }
        let _ = sessions.receive(&a, "vivecraft:data", &bytes);
    }
}
#[test]
fn deferred_pose_effect_expires_when_source_leaves() {
    let mut sessions = Sessions::new(Settings::default());
    let a = context(1);
    let b = context(2);
    connect(&mut sessions, &a);
    connect(&mut sessions, &b);
    sessions
        .receive(&a, "vivecraft:data", &vector("pose_arms"))
        .unwrap();
    let effects = sessions.tick(&[a.clone(), b]);
    assert!(sessions.can_deliver(&effects.outgoing[0]));
    let cleanup = sessions.remove(&a.id);
    assert!(!sessions.can_deliver(&effects.outgoing[0]));
    assert!(
        cleanup
            .outgoing
            .iter()
            .all(|outgoing| sessions.can_deliver(outgoing))
    );
}

#[test]
fn owner_echo_and_duplicate_handshake_cleanup() {
    let settings = Settings::default()
        .edit("general.sendDataToOwner", "set", Some("true"))
        .unwrap();
    let mut sessions = Sessions::new(settings);
    let a = context(1);
    connect(&mut sessions, &a);
    sessions
        .receive(&a, "vivecraft:data", &vector("pose_arms"))
        .unwrap();
    assert_eq!(sessions.tick(std::slice::from_ref(&a)).outgoing.len(), 1);
    let effects = sessions
        .receive(&a, "vivecraft:data", &vector("version_nonvr"))
        .unwrap();
    assert!(
        effects
            .outgoing
            .iter()
            .any(|p| p.bytes.starts_with(&[17, 0]))
    );
    assert!(!sessions.snapshot(&a.id).unwrap().vr);
    assert!(sessions.snapshot(&a.id).unwrap().pose.is_none());
}
