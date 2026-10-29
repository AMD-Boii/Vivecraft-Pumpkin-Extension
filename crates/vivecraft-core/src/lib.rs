//! Host-independent Vivecraft wire compatibility and session policy.
//! Wire semantics ported from the LGPLv3 Vivecraft server extension in `core/`.

pub mod config;
mod legacy;
mod pose;
mod protocol;
mod sessions;

pub use pose::{FbtMode, Pose, VrPose};
pub use sessions::{Context, Effects, Outgoing, PlayerId, SessionSnapshot, Sessions};
