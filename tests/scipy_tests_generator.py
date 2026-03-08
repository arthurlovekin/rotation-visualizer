"""
Use the scipy library to generate sets of RotationMatrix, Quaternion,
AxisAngle, RotationVector, and EulerAngles (for each of the 12 EulerSequences).
This is saved as json and can be used to test the rotation.rs library.
Each json test case has a name and all 28 different representations of the same rotation.

Quaternions should be in the form of [w, x, y, z].
AxisAngles should be in the form of [x, y, z, angle (radians)].
RotationVectors should be in the form of [x, y, z] (norm is in radians).
EulerAngles should be in the form of [x, y, z] (radians).
EulerSequences should include:
  The 6 intrinsic Tait-Bryan angles: XYZ, XZY, YXZ, YZX, ZXY, ZYX
  The 6 extrinsic Tait-Bryan angles: xyz, yzx, zxy, xzy, yxz, zyx
  The 6 intrinsic Proper Euler angles: XYX, XZX, YXY, YZY, ZXZ, ZYZ
  The 6 extrinsic Proper Euler angles: xyx, zxz, yxy, yzy, zxz, zyz

The tests include:
1) Axis-angle is created with: Rotation by [0, pi/4, pi/3, pi/2, 2pi/3, 3pi/4, pi, 3pi/2, 2pi]
    about each of the eight primary directions. This is converted to a scipy Rotation
    by converting to a rotvec and using from_rotvec.
2) For each of the following functions, 10 random (with known seed) rotations are created:
    - from_quat, from_matrix, from_rotvec, from_euler
3) 10 more tests are also generated using a small tolerance (1e-7) to check that nothing weird happens to small rotations.
"""

import json
import math
import warnings

import numpy as np
from scipy.spatial.transform import Rotation as R

# Scipy Euler sequence strings (intrinsic uppercase)
EULER_SEQUENCES = [
    "XYZ", "XZY", "YXZ", "YZX", "ZXY", "ZYX",
    "XYX", "XZX", "YXY", "YZY", "ZXZ", "ZYZ",
]

# Eight primary directions: diagonals of a cube (±1, ±1, ±1) normalized
PRIMARY_DIRECTIONS = [
    (1, 1, 1),
    (1, 1, -1),
    (1, -1, 1),
    (1, -1, -1),
    (-1, 1, 1),
    (-1, 1, -1),
    (-1, -1, 1),
    (-1, -1, -1),
]

ANGLES_RAD = [0, math.pi / 4, math.pi / 3, math.pi / 2, 2 * math.pi / 3,
              3 * math.pi / 4, math.pi, 3 * math.pi / 2, 2 * math.pi]


def rotation_to_test_case(r, name: str) -> dict:
    """Convert a scipy Rotation to a test case dict with all representations."""
    # Quaternion [w, x, y, z] - scipy uses scalar_first=True for w,x,y,z
    quat = r.as_quat(scalar_first=True)
    quaternion = [float(quat[0]), float(quat[1]), float(quat[2]), float(quat[3])]

    # Rotation matrix: 3x3 row-major
    mat = r.as_matrix()
    rotation_matrix = [[float(mat[i, j]) for j in range(3)] for i in range(3)]

    # Rotation vector [x, y, z], norm = angle in radians
    rotvec = r.as_rotvec()
    rotation_vector = [float(rotvec[0]), float(rotvec[1]), float(rotvec[2])]

    # Axis-angle [x, y, z, angle]: unit axis + angle in radians
    norm = np.linalg.norm(rotvec)
    if norm < 1e-10:
        axis_angle = [1.0, 0.0, 0.0, 0.0]  # identity: arbitrary axis, zero angle
    else:
        axis = rotvec / norm
        axis_angle = [float(axis[0]), float(axis[1]), float(axis[2]), float(norm)]

    # Euler angles for each of the 12 sequences
    euler_angles = {}
    for seq in EULER_SEQUENCES:
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)  # gimbal lock
            euler = r.as_euler(seq, degrees=False)
        euler_angles[seq] = [float(euler[0]), float(euler[1]), float(euler[2])]

    return {
        "name": name,
        "quaternion": quaternion,
        "rotation_matrix": rotation_matrix,
        "axis_angle": axis_angle,
        "rotation_vector": rotation_vector,
        "euler_angles": euler_angles,
    }


def generate_axis_angle_tests() -> list[dict]:
    """Generate tests from axis-angle: 8 directions × 9 angles."""
    cases = []
    for i, (dx, dy, dz) in enumerate(PRIMARY_DIRECTIONS):
        axis = np.array([dx, dy, dz], dtype=float) / math.sqrt(3)
        for j, angle in enumerate(ANGLES_RAD):
            rotvec = axis * angle
            r = R.from_rotvec(rotvec)
            name = f"axis_angle_dir{i}_angle{j}"
            cases.append(rotation_to_test_case(r, name))
    return cases


def generate_random_tests(seed: int = 42, n: int = 10) -> dict:
    """Generate random rotations from each of from_quat, from_matrix, from_rotvec, from_euler."""
    rng = np.random.default_rng(seed)
    cases = []

    # from_quat
    for i in range(n):
        q = rng.standard_normal(4)
        q = q / np.linalg.norm(q)
        r = R.from_quat(q, scalar_first=True)
        cases.append(rotation_to_test_case(r, f"from_quat_{i}"))

    # from_matrix
    for i in range(n):
        r = R.random(random_state=rng)
        cases.append(rotation_to_test_case(r, f"from_matrix_{i}"))

    # from_rotvec
    for i in range(n):
        rotvec = rng.standard_normal(3) * math.pi
        r = R.from_rotvec(rotvec)
        cases.append(rotation_to_test_case(r, f"from_rotvec_{i}"))

    # from_euler (use ZYX / roll-pitch-yaw as representative)
    for i in range(n):
        angles = rng.uniform(-math.pi, math.pi, 3)
        r = R.from_euler("ZYX", angles)
        cases.append(rotation_to_test_case(r, f"from_euler_{i}"))

    return cases


def generate_small_tolerance_tests(tol: float = 1e-7, n: int = 10, seed: int = 123) -> list[dict]:
    """Generate small rotations to check numerical stability."""
    rng = np.random.default_rng(seed)
    cases = []
    for i in range(n):
        rotvec = rng.standard_normal(3) * tol
        r = R.from_rotvec(rotvec)
        cases.append(rotation_to_test_case(r, f"small_tol_{i}"))
    return cases


def main():
    all_cases = []

    # 1) Axis-angle tests
    all_cases.extend(generate_axis_angle_tests())

    # 2) Random tests from each constructor
    all_cases.extend(generate_random_tests(seed=42, n=10))

    # 3) Small tolerance tests
    all_cases.extend(generate_small_tolerance_tests(tol=1e-7, n=10, seed=123))

    output = {"test_cases": all_cases}

    out_path = "tests/scipy_test_cases.json"
    with open(out_path, "w") as f:
        json.dump(output, f, indent=2)

    print(f"Generated {len(all_cases)} test cases -> {out_path}")


if __name__ == "__main__":
    main()
