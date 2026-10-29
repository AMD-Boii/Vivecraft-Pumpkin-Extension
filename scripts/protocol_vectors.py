"""Independent, synthetic fixture oracle. Prints vectors; never edits fixture files.
Layouts: pinned Vivecraft 26.3-1.3.17 VrPlayerState/Pose/UBERPACKET.
These bytes are generated, NOT captured from a Minecraft client.
"""
import json
import struct


def pose(x=1.0, y=2.0, z=3.0):
    return struct.pack('>7f', x, y, z, 1.0, 0.0, 0.0, 0.0)


base = b'\x00' + pose() + b'\x01' + pose() + b'\x01' + pose()
vectors = {
    'version_10': b'\x00Vivecraft 26.3 1.3.17 VR\n10\n0',
    'version_nonvr': b'\x00Vivecraft 26.3 1.3.17 NONVR\n10\n0',
    'version_legacy': b'\x00Vivecraft 1.12 VR',
    'pose_arms': b'\x12' + base,
    'pose_feet': b'\x12' + base + b'\x01' + pose() * 3,
    'pose_joints': b'\x12' + base + b'\x02' + pose() * 7,
    'relay_arms': b'\x08' + bytes([1]) * 16 + base + struct.pack('>2f', 1.0, 1.0),
    'legacy_head': b'\x02\x00' + pose(),
    'world_scale': b'\x05' + struct.pack('>f', 2.0),
    'height_scale': b'\x0c' + struct.pack('>f', 1.0),
}
print(json.dumps({name: value.hex() for name, value in vectors.items()}, indent=2))
