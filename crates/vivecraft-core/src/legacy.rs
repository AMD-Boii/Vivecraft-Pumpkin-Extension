//! Historical fragment assembly retained for fixture coverage, not old Minecraft admission.
use crate::{FbtMode, Pose, VrPose};

#[derive(Default)]
pub(crate) struct LegacyFrame {
    tick: Option<u64>,
    parts: [Option<(bool, Pose)>; 3],
}
impl LegacyFrame {
    pub(crate) fn push(
        &mut self,
        id: u8,
        flag: bool,
        pose: Pose,
        tick: u64,
        world_space: bool,
        player_position: [f64; 3],
    ) -> Option<VrPose> {
        if self
            .tick
            .is_none_or(|previous| tick < previous || tick.saturating_sub(previous) > 1)
        {
            self.parts = [None; 3];
            self.tick = Some(tick);
        }
        let index = usize::from(id.checked_sub(2)?);
        if index >= self.parts.len() {
            return None;
        }
        self.parts[index] = Some((flag, pose));
        let (seated, head) = self.parts[0]?;
        let (left_handed, main_hand) = self.parts[1]?;
        let (_, off_hand) = self.parts[2]?;
        self.parts = [None; 3];
        self.tick = None;
        let mut pose = VrPose {
            seated,
            left_handed,
            head,
            main_hand,
            off_hand,
            mode: FbtMode::ArmsOnly,
            trackers: vec![],
        };
        if world_space {
            pose.offset(player_position.map(|n| -n));
        }
        Some(pose)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assembles_legacy_world_and_relative_coordinates_without_stale_fragments() {
        let pose = Pose {
            position: [101., 66., -9.],
            orientation: [1., 0., 0., 0.],
        };
        let mut frame = LegacyFrame::default();
        assert!(
            frame
                .push(2, true, pose, 10, true, [100., 64., -10.])
                .is_none()
        );
        assert!(
            frame
                .push(3, false, pose, 10, true, [100., 64., -10.])
                .is_none()
        );
        let vr = frame
            .push(4, false, pose, 10, true, [100., 64., -10.])
            .unwrap();
        assert_eq!(vr.head.position, [1., 2., 1.]);
        assert!(vr.seated);
        assert!(frame.push(2, false, pose, 11, false, [0.; 3]).is_none());
        assert!(frame.push(3, false, pose, 15, false, [0.; 3]).is_none());
        assert!(frame.push(4, false, pose, 15, false, [0.; 3]).is_none());
        assert_eq!(
            frame
                .push(2, false, pose, 15, false, [0.; 3])
                .unwrap()
                .head
                .position,
            pose.position
        );
    }
}
