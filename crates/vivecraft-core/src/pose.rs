use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub position: [f32; 3],
    /// Vivecraft wire order, not JOML constructor order.
    pub orientation: [f32; 4],
}

impl Pose {
    pub(crate) fn valid(&self) -> bool {
        let norm_squared = self
            .orientation
            .iter()
            .map(|n| f64::from(*n).powi(2))
            .sum::<f64>();
        // Valid client rotations are unit quaternions. Allow generous numerical drift,
        // but do not relay magnitudes that can overflow downstream float transforms.
        self.position
            .iter()
            .all(|n| n.is_finite() && n.abs() <= 32_000_000.)
            && self.orientation.iter().all(|n| n.is_finite())
            && (1e-12..=4.).contains(&norm_squared)
    }

    pub(crate) fn offset(&mut self, position: [f64; 3]) {
        for (value, offset) in self.position.iter_mut().zip(position) {
            *value = (f64::from(*value) + offset) as f32;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FbtMode {
    ArmsOnly,
    WithFeet,
    WithJoints,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VrPose {
    pub seated: bool,
    pub left_handed: bool,
    pub head: Pose,
    pub main_hand: Pose,
    pub off_hand: Pose,
    pub mode: FbtMode,
    /// Waist, right foot, left foot; then right/left knee and right/left elbow.
    pub trackers: Vec<Pose>,
}

impl VrPose {
    pub(crate) fn offset(&mut self, position: [f64; 3]) {
        self.head.offset(position);
        self.main_hand.offset(position);
        self.off_hand.offset(position);
        for tracker in &mut self.trackers {
            tracker.offset(position);
        }
    }
}
