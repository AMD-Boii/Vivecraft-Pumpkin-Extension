# Fixture provenance

`vectors.json` contains **synthetic, independently generated** bytes, not packet captures.
The oracle is [`scripts/protocol_vectors.py`](../../../../scripts/protocol_vectors.py), using
Python's standard-library big-endian `struct` implementation, not the Rust codec. Run it
with `python -I scripts/protocol_vectors.py` to print the expected corpus for comparison.

Current-protocol layouts were inspected in Vivecraft commit
`7e610caf672c67e4501ba8675ba0d620ffeb17ee`:
`common/src/main/java/org/vivecraft/common/network/{Pose,VrPlayerState}.java`,
`packet/c2s/VersionPayloadC2S.java`, `packet/s2c/UberPacketPayloadS2C.java`,
and `CommonNetworkHelper.java` (w,x,y,z quaternion wire order).

Historical VERSION and head-fragment layouts are derived from this repository's Java
`core/src/main/java/org/vivecraft/network/packet/c2s/` reference, retained under its LGPLv3 license.
No historical Minecraft client was run. Current live admission is intentionally protocol
7..10 because older clients cannot represent the preview's explicit disabled teleport policy.
Historical decoder/coordinate-transform tests do not imply live historical-client support.

The first pose has position (1,2,3), identity quaternion, standing mode and left handedness.
The feet/joints vectors add 3/7 identical trackers; this makes layout and byte-count failures
visible. `relay_arms` uses UUID `01010101-0101-0101-0101-010101010101` and unit scales.

Remaining acceptance work includes captured real-client traffic, asymmetric tracker/rotation
vectors, and older-client generation provenance beyond these Java-reference examples.
