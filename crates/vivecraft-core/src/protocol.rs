// Explicit wire values from Vivecraft 26.3-1.3.17 and the Java reference.
use crate::pose::{FbtMode, Pose, VrPose};

pub(crate) const MAX_PAYLOAD: usize = 65_536;
pub(crate) const MAX_VERSION: i32 = 10;
pub(crate) const OPTION_TOGGLE: i32 = 7;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Packet {
    Version {
        text: String,
        vr: bool,
        min: i32,
        max: i32,
    },
    Legacy(u8, bool, Pose),
    Scale(bool, f32),
    Draw(f32),
    Pose(VrPose),
    Active(bool),
    BodyPart(u8, bool),
    AimPosition([f32; 3]),
    AimDirection([f32; 3]),
    AimReset(u8),
    DamageDirection,
    Gameplay(u8),
}

struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], String> {
        let (head, tail) = self.0.split_at_checked(N).ok_or("truncated payload")?;
        self.0 = tail;
        head.try_into().map_err(|_| "invalid payload length".into())
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take::<1>()?[0])
    }
    fn boolean(&mut self) -> Result<bool, String> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("invalid boolean".into()),
        }
    }
    fn float(&mut self) -> Result<f32, String> {
        let n = f32::from_be_bytes(self.take()?);
        if n.is_finite() {
            Ok(n)
        } else {
            Err("non-finite float".into())
        }
    }
    fn vector(&mut self) -> Result<[f32; 3], String> {
        Ok([self.float()?, self.float()?, self.float()?])
    }
    fn pose(&mut self) -> Result<Pose, String> {
        let pose = Pose {
            position: self.vector()?,
            orientation: [self.float()?, self.float()?, self.float()?, self.float()?],
        };
        if pose.valid() {
            Ok(pose)
        } else {
            Err("degenerate pose".into())
        }
    }
    fn vr_pose(&mut self, reserved: usize) -> Result<VrPose, String> {
        let seated = self.boolean()?;
        let head = self.pose()?;
        let left_handed = self.boolean()?;
        let main_hand = self.pose()?;
        let _legacy_reverse = self.boolean()?;
        let off_hand = self.pose()?;
        let (mode, count) = if self.0.len() > reserved {
            match self.byte()? {
                0 => (FbtMode::ArmsOnly, 0),
                1 => (FbtMode::WithFeet, 3),
                2 => (FbtMode::WithJoints, 7),
                _ => return Err("invalid FBT mode".into()),
            }
        } else {
            (FbtMode::ArmsOnly, 0)
        };
        let mut trackers = Vec::with_capacity(count);
        for _ in 0..count {
            trackers.push(self.pose()?);
        }
        if self.0.len() != reserved {
            return Err("unexpected pose tail".into());
        }
        Ok(VrPose {
            seated,
            left_handed,
            head,
            main_hand,
            off_hand,
            mode,
            trackers,
        })
    }
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Packet, String> {
    if bytes.is_empty() || bytes.len() > MAX_PAYLOAD {
        return Err("invalid payload size".into());
    }
    let mut r = Reader(bytes);
    let id = r.byte()?;
    let packet = match id {
        0 => {
            if r.0.len() > 512 {
                return Err("version string too long".into());
            }
            let text = std::str::from_utf8(r.0).map_err(|_| "invalid version UTF-8")?;
            let parts: Vec<_> = text.split('\n').collect();
            if parts[0].is_empty() || !(parts.len() == 1 || parts.len() == 3) {
                return Err("invalid VERSION framing".into());
            }
            let (min, max) = if parts.len() == 3 {
                (
                    parts[2]
                        .parse::<i32>()
                        .map_err(|_| "invalid minimum version")?,
                    parts[1]
                        .parse::<i32>()
                        .map_err(|_| "invalid maximum version")?,
                )
            } else {
                (-1, -1)
            };
            if min < -1 || max < min {
                return Err("invalid version range".into());
            }
            let packet = Packet::Version {
                text: parts[0].into(),
                vr: !parts[0].contains("NONVR"),
                min,
                max,
            };
            r.0 = &[];
            packet
        }
        2..=4 => Packet::Legacy(id, r.boolean()?, r.pose()?),
        5 | 12 => {
            let scale = r.float()?;
            // Client world-scale slider supports 0.1..100; height is a positive calibrated multiplier.
            if !(0.01..=100.0).contains(&scale) {
                return Err("scale outside supported bounds".into());
            }
            Packet::Scale(id == 5, scale)
        }
        6 => {
            let draw = r.float()?;
            if !(0.0..=1.0).contains(&draw) {
                return Err("invalid bow draw".into());
            }
            Packet::Draw(draw)
        }
        9 => {
            r.vector()?;
            Packet::Gameplay(id)
        }
        10 | 27 => Packet::Gameplay(id),
        13 => {
            let body = r.byte()?;
            if body > 9 {
                return Err("invalid body part".into());
            }
            let aim = if r.0.is_empty() { false } else { r.boolean()? };
            Packet::BodyPart(body, aim)
        }
        14 => {
            r.boolean()?;
            Packet::Gameplay(id)
        }
        17 => Packet::Active(r.boolean()?),
        18 => Packet::Pose(r.vr_pose(0)?),
        22 => Packet::DamageDirection,
        24 => {
            let v = r.vector()?;
            if v.iter().map(|n| f64::from(*n).powi(2)).sum::<f64>() < 1e-12 {
                return Err("zero aim direction".into());
            }
            Packet::AimDirection(v)
        }
        25 => Packet::AimReset(r.byte()?),
        26 => Packet::AimPosition(r.vector()?),
        28 => {
            r.boolean()?;
            r.byte()?;
            Packet::Gameplay(id)
        }
        _ => return Err(format!("unknown or wrong-direction payload {id}")),
    };
    if !r.0.is_empty() {
        return Err("unexpected trailing bytes".into());
    }
    Ok(packet)
}

fn put_float(out: &mut Vec<u8>, value: f32) {
    out.extend(value.to_be_bytes());
}
fn put_pose(out: &mut Vec<u8>, pose: &Pose) {
    for n in pose.position.iter().chain(&pose.orientation) {
        put_float(out, *n);
    }
}
pub(crate) fn put_vr_pose(out: &mut Vec<u8>, pose: &VrPose, version: i32) {
    out.push(u8::from(pose.seated));
    put_pose(out, &pose.head);
    out.push(u8::from(pose.left_handed));
    put_pose(out, &pose.main_hand);
    out.push(u8::from(pose.left_handed));
    put_pose(out, &pose.off_hand);
    if version >= 1 && pose.mode != FbtMode::ArmsOnly {
        out.push(if pose.mode == FbtMode::WithFeet { 1 } else { 2 });
        for tracker in &pose.trackers {
            put_pose(out, tracker);
        }
    }
}
pub(crate) fn relay(
    id: [u8; 16],
    pose: &VrPose,
    world: f32,
    height: f32,
    version: i32,
    legacy_offset: Option<[f64; 3]>,
) -> Vec<u8> {
    let mut out = vec![8];
    out.extend(id);
    if let Some(offset) = legacy_offset {
        let mut pose = pose.clone();
        pose.offset(offset);
        put_vr_pose(&mut out, &pose, version);
    } else {
        put_vr_pose(&mut out, pose, version);
    }
    put_float(&mut out, world);
    put_float(&mut out, height);
    out
}
pub(crate) fn inactive(id: [u8; 16]) -> Vec<u8> {
    let mut out = vec![17, 0];
    out.extend(id);
    out
}
pub(crate) fn server_version() -> Vec<u8> {
    let text = b"Vivecraft Pumpkin 0.1.0 (sessions preview)";
    let mut out = vec![0, text.len() as u8];
    out.extend(text);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_all_truncations_and_unknowns() {
        for id in 0..=255 {
            let _ = decode(&[id]);
        }
        assert!(decode(&[]).is_err());
        assert!(decode(&[255]).is_err());
        assert!(decode(&[5, 0x7f, 0x80, 0, 0]).is_err());
    }
    #[test]
    fn legacy_version_and_fragments() {
        assert!(matches!(
            decode(b"\0Vivecraft 1.12 VR"),
            Ok(Packet::Version {
                min: -1,
                max: -1,
                ..
            })
        ));
        let mut data = vec![2, 1];
        for value in [1f32, 2., 3., 1., 0., 0., 0.] {
            data.extend(value.to_be_bytes());
        }
        assert!(matches!(decode(&data), Ok(Packet::Legacy(2, true, _))));
        for n in 0..data.len() {
            assert!(decode(&data[..n]).is_err());
        }
    }
}
