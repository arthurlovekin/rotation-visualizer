#!/usr/bin/env python3
"""
Generate rotation conversion test cases using scipy for validating Rust rotation code.

Outputs a Rust test module that can be copy-pasted into rotation.rs.

Usage:
    python tests/scipy_test_case_generator.py > src/app/rotation_tests_generated.rs
    # or
    python tests/scipy_test_case_generator.py -o src/app/rotation_tests_generated.rs
"""

import math
import argparse
import sys
from pathlib import Path

import numpy as np
from scipy.spatial.transform import Rotation

# f32 epsilon ~= 1.19e-7; use slightly larger for "near zero" / "near limit" cases
F32_EPS = np.float32(1.1920929e-7)
# Tolerance for "approximately equal" in test comparisons (allow for round-trip error)
F32_TOL = 1e-5
# Minimum tolerance for quat_components_near_limits (max diff ~4.9e-5 in w)
LOOSE_TOL_QUAT_COMPONENTS = 5.5e-5

# Rust uses angle in [0, 2*pi) for AxisAngle and RotationVector
TWO_PI = 2.0 * math.pi


def _to_f32(x):
    """Convert to f32 for consistency with Rust."""
    return float(np.float32(x))


def _rust_f32_literal(x):
    """Format a float as a Rust f32 literal."""
    v = _to_f32(x)
    if v == 0.0 and x != 0:
        return "0.0_f32"
    if v == 1.0:
        return "1.0_f32"
    if v == -1.0:
        return "-1.0_f32"
    s = repr(v)
    if "e" in s or "E" in s:
        return f"{s}_f32"
    return f"{s}_f32"


def _quat_scipy_to_rust(q):
    """scipy (x,y,z,w) -> Rust (w,x,y,z)."""
    return {"w": _to_f32(q[3]), "x": _to_f32(q[0]), "y": _to_f32(q[1]), "z": _to_f32(q[2])}


def _normalize_angle_0_2pi(angle):
    """Bring angle into [0, 2*pi) to match Rust's AxisAngle/RotationVector convention."""
    a = angle % TWO_PI
    if a < 0:
        a += TWO_PI
    return _to_f32(a)


def _axis_from_rotvec(rv):
    """Extract unit axis and angle from rotation vector."""
    norm = np.linalg.norm(rv)
    if norm < 1e-12:
        return np.array([0.0, 0.0, 0.0]), 0.0
    return rv / norm, _normalize_angle_0_2pi(norm)


def rotation_to_test_case(r: Rotation, label: str) -> dict:
    """
    Convert a scipy Rotation to a test case dict with all representations.
    Uses Rust conventions: quat (w,x,y,z), axis_angle angle in [0,2π), etc.
    """
    # Quaternion (Rust: w,x,y,z)
    q_xyzw = r.as_quat(scalar_first=False)
    quat = _quat_scipy_to_rust([q_xyzw[0], q_xyzw[1], q_xyzw[2], q_xyzw[3]])

    # Rotation vector (axis * angle, norm = angle in [0, 2π))
    rv = r.as_rotvec()
    rv_norm = np.linalg.norm(rv)
    if rv_norm < 1e-12:
        rotvec = {"x": 0.0, "y": 0.0, "z": 0.0}
    else:
        angle = _normalize_angle_0_2pi(rv_norm)
        axis = rv / rv_norm
        rotvec = {
            "x": _to_f32(axis[0] * angle),
            "y": _to_f32(axis[1] * angle),
            "z": _to_f32(axis[2] * angle),
        }

    # Axis-angle (unit axis + angle in [0, 2π))
    axis, angle = _axis_from_rotvec(rv)
    axis_angle = {
        "x": _to_f32(axis[0]),
        "y": _to_f32(axis[1]),
        "z": _to_f32(axis[2]),
        "angle": _to_f32(angle),
    }

    # Rotation matrix 3x3
    mat = r.as_matrix()
    rotation_matrix = [
        [_to_f32(mat[0, 0]), _to_f32(mat[0, 1]), _to_f32(mat[0, 2])],
        [_to_f32(mat[1, 0]), _to_f32(mat[1, 1]), _to_f32(mat[1, 2])],
        [_to_f32(mat[2, 0]), _to_f32(mat[2, 1]), _to_f32(mat[2, 2])],
    ]

    return {
        "label": label,
        "quaternion": quat,
        "axis_angle": axis_angle,
        "rotation_vector": rotvec,
        "rotation_matrix": rotation_matrix,
    }


def generate_all_test_cases() -> list[dict]:
    """Generate comprehensive test cases covering all conversions and edge cases."""
    cases = []

    # -------------------------------------------------------------------------
    # 1. Identity and near-identity
    # -------------------------------------------------------------------------
    cases.append(
        rotation_to_test_case(Rotation.identity(), "identity")
    )

    # Near-zero rotation (radians) - stress f32 precision
    for label, angle_rad in [
        ("near_zero_1e-6", 1e-6),
        ("near_zero_1e-7", 1e-7),
        ("near_zero_f32_eps", float(F32_EPS)),
    ]:
        r = Rotation.from_rotvec([angle_rad, 0.0, 0.0])
        cases.append(rotation_to_test_case(r, label))

    # -------------------------------------------------------------------------
    # 2. Standard angles (radians)
    # -------------------------------------------------------------------------
    for label, angle_rad in [
        ("angle_pi_over_4", math.pi / 4),
        ("angle_pi_over_2", math.pi / 2),
        ("angle_pi", math.pi),
        ("angle_3pi_over_2", 3 * math.pi / 2),
        ("angle_2pi_minus_eps", TWO_PI - 1e-6),
    ]:
        r = Rotation.from_rotvec([angle_rad, 0.0, 0.0])
        cases.append(rotation_to_test_case(r, label))

    # -------------------------------------------------------------------------
    # 3. Standard angles (degrees) - converted to radians for storage
    # -------------------------------------------------------------------------
    for label, angle_deg in [
        ("degrees_45", 45.0),
        ("degrees_90", 90.0),
        ("degrees_180", 180.0),
        ("degrees_270", 270.0),
        ("degrees_360", 360.0),
    ]:
        angle_rad = math.radians(angle_deg)
        r = Rotation.from_rotvec([angle_rad, 0.0, 0.0])
        cases.append(rotation_to_test_case(r, label))

    # -------------------------------------------------------------------------
    # 4. Arbitrary axes (not just X)
    # -------------------------------------------------------------------------
    axes = [
        ([0, 1, 0], "axis_y"),
        ([0, 0, 1], "axis_z"),
        ([1, 1, 0], "axis_xy"),
        ([1, 1, 1], "axis_xyz"),
        ([1, -0.5, 0.3], "axis_arbitrary"),
    ]
    for axis, ax_label in axes:
        axis = np.array(axis, dtype=float)
        axis = axis / np.linalg.norm(axis)
        r = Rotation.from_rotvec(axis * (math.pi / 3))
        cases.append(rotation_to_test_case(r, ax_label))

    # -------------------------------------------------------------------------
    # 5. Quaternion edge cases
    # -------------------------------------------------------------------------
    # w ≈ 1 (identity-like)
    r = Rotation.from_quat([F32_EPS, 0, 0, 1.0], scalar_first=False)
    cases.append(rotation_to_test_case(r, "quat_w_near_1"))

    # w ≈ 0 (angle = π) - 180° rotation
    r = Rotation.from_quat([1.0, 0, 0, 0.0], scalar_first=False)  # 180° around X
    cases.append(rotation_to_test_case(r, "quat_w_zero_180deg_x"))

    # w ≈ -1 (angle ≈ 2π) - near identity
    r = Rotation.from_quat([-1.0 + F32_EPS, 0, 0, 0], scalar_first=False)
    cases.append(rotation_to_test_case(r, "quat_w_near_minus_1"))

    # -------------------------------------------------------------------------
    # 6. Rotation matrix edge cases
    # -------------------------------------------------------------------------
    # Create from matrix directly (90° around Z)
    mat_90z = np.array([
        [0, -1, 0],
        [1, 0, 0],
        [0, 0, 1],
    ], dtype=float)
    r = Rotation.from_matrix(mat_90z)
    cases.append(rotation_to_test_case(r, "matrix_90deg_z"))

    # -------------------------------------------------------------------------
    # 7. From Euler angles (degrees) - common in robotics
    # -------------------------------------------------------------------------
    for label, euler_deg in [
        ("euler_xyz_30_45_60", [30, 45, 60]),
        ("euler_xyz_90_0_0", [90, 0, 0]),
        ("euler_xyz_0_90_0", [0, 90, 0]),
        ("euler_xyz_0_0_90", [0, 0, 90]),
        ("euler_xyz_gimbal_like", [90, 90, 0]),
    ]:
        r = Rotation.from_euler("xyz", euler_deg, degrees=True)
        cases.append(rotation_to_test_case(r, label))

    # -------------------------------------------------------------------------
    # 8. f32 precision limits
    # -------------------------------------------------------------------------
    # Smallest non-zero rotation we can represent
    r = Rotation.from_rotvec([F32_EPS, 0, 0])
    cases.append(rotation_to_test_case(r, "f32_min_rotation"))

    # Angle just below 2π
    r = Rotation.from_rotvec([TWO_PI - F32_EPS, 0, 0])
    cases.append(rotation_to_test_case(r, "f32_angle_near_2pi"))

    # Components near f32 limits (avoid overflow)
    r = Rotation.from_quat([0.9999999, 0.0001, 0.0001, 0.0001], scalar_first=False)
    cases.append(rotation_to_test_case(r, "quat_components_near_limits"))

    # -------------------------------------------------------------------------
    # 9. Round-trip stress: use each representation as source
    # -------------------------------------------------------------------------
    # Start from rotation vector with non-canonical angle (> 2π)
    r = Rotation.from_rotvec([TWO_PI + math.pi / 4, 0, 0])  # scipy normalizes
    cases.append(rotation_to_test_case(r, "rotvec_angle_gt_2pi"))

    # Zero rotation vector
    r = Rotation.from_rotvec([0, 0, 0])
    cases.append(rotation_to_test_case(r, "rotvec_zero"))

    # -------------------------------------------------------------------------
    # 10. Random rotations (fixed seed for reproducibility)
    # -------------------------------------------------------------------------
    rng = np.random.default_rng(42)
    for i in range(10):
        r = Rotation.random(rng=rng)
        cases.append(rotation_to_test_case(r, f"random_{i}"))

    return cases


def _rust_case(c: dict) -> str:
    """Generate Rust code for a single test case."""
    q = c["quaternion"]
    aa = c["axis_angle"]
    rv = c["rotation_vector"]
    mat = c["rotation_matrix"]
    label = c["label"].replace("-", "_")
    tol = LOOSE_TOL_QUAT_COMPONENTS if c["label"] == "quat_components_near_limits" else F32_TOL

    lines = [
        f"    #[test]",
        f"    fn scipy_{label}() {{",
        f"        const TOL: f32 = {tol}_f32;",
        f"        let expected_quat = Quaternion::new(",
        f"            {_rust_f32_literal(q['w'])}, {_rust_f32_literal(q['x'])}, ",
        f"            {_rust_f32_literal(q['y'])}, {_rust_f32_literal(q['z'])}",
        f"        );",
        f"        let expected_aa = AxisAngle::new(",
        f"            {_rust_f32_literal(aa['x'])}, {_rust_f32_literal(aa['y'])}, ",
        f"            {_rust_f32_literal(aa['z'])}, {_rust_f32_literal(aa['angle'])}",
        f"        );",
        f"        let expected_rv = RotationVector::new(",
        f"            {_rust_f32_literal(rv['x'])}, {_rust_f32_literal(rv['y'])}, ",
        f"            {_rust_f32_literal(rv['z'])}",
        f"        );",
        f"        let expected_mat = RotationMatrix([",
        f"            [{_rust_f32_literal(mat[0][0])}, {_rust_f32_literal(mat[0][1])}, {_rust_f32_literal(mat[0][2])}],",
        f"            [{_rust_f32_literal(mat[1][0])}, {_rust_f32_literal(mat[1][1])}, {_rust_f32_literal(mat[1][2])}],",
        f"            [{_rust_f32_literal(mat[2][0])}, {_rust_f32_literal(mat[2][1])}, {_rust_f32_literal(mat[2][2])}],",
        f"        ]);",
        f"",
        f"        // From Quaternion -> all others",
        f"        let r = Rotation::from(expected_quat);",
        f"        assert_axis_angle_near(&r.as_axis_angle(), &expected_aa, TOL);",
        f"        assert_rotation_vector_near(&r.as_rotation_vector(), &expected_rv, TOL);",
        f"        assert_rotation_matrix_near(&r.as_rotation_matrix(), &expected_mat, TOL);",
        f"",
        f"        // From AxisAngle -> Quaternion",
        f"        let r = Rotation::from(expected_aa);",
        f"        assert_quaternion_near(&r.as_quaternion(), &expected_quat, TOL);",
        f"",
        f"        // From RotationVector -> Quaternion",
        f"        let r = Rotation::from(expected_rv);",
        f"        assert_quaternion_near(&r.as_quaternion(), &expected_quat, TOL);",
        f"",
        f"        // From RotationMatrix -> Quaternion",
        f"        let r = Rotation::from(expected_mat);",
        f"        assert_quaternion_near(&r.as_quaternion(), &expected_quat, TOL);",
        f"    }}",
    ]
    return "\n".join(lines)


def _generate_rust_module(cases: list[dict]) -> str:
    """Generate the full Rust test module."""
    header = '''// Generated by tests/scipy_test_case_generator.py
// Do not edit these tests by hand. Regenerate with:
//   python tests/scipy_test_case_generator.py -o src/app/rotation_tests_generated.rs
//
// This file is included from src/app/rotation.rs via include!().
// Kept in src/app/ (not tests/) so Cargo does not compile it as a separate integration test.

#[cfg(test)]
mod scipy_tests {
    use super::*;

    fn assert_quaternion_near(rust_actual: &Quaternion, scipy_expected: &Quaternion, tol: f32) {
        let q_ok = (rust_actual.w - scipy_expected.w).abs() <= tol
            && (rust_actual.x - scipy_expected.x).abs() <= tol
            && (rust_actual.y - scipy_expected.y).abs() <= tol
            && (rust_actual.z - scipy_expected.z).abs() <= tol;
        let dual_ok = (rust_actual.w + scipy_expected.w).abs() <= tol
            && (rust_actual.x + scipy_expected.x).abs() <= tol
            && (rust_actual.y + scipy_expected.y).abs() <= tol
            && (rust_actual.z + scipy_expected.z).abs() <= tol;
        assert!(
            q_ok || dual_ok,
            "Quaternion: Rust got {:?}, Scipy expected {:?}",
            rust_actual,
            scipy_expected
        );
    }

    fn assert_axis_angle_near(rust_actual: &AxisAngle, scipy_expected: &AxisAngle, tol: f32) {
        let angle_ok = (rust_actual.angle - scipy_expected.angle).abs() <= tol;
        let axis_ok = (rust_actual.x - scipy_expected.x).abs() <= tol
            && (rust_actual.y - scipy_expected.y).abs() <= tol
            && (rust_actual.z - scipy_expected.z).abs() <= tol;
        let equiv_ok = (rust_actual.angle - (2.0 * std::f32::consts::PI - scipy_expected.angle)).abs() <= tol
            && (rust_actual.x + scipy_expected.x).abs() <= tol
            && (rust_actual.y + scipy_expected.y).abs() <= tol
            && (rust_actual.z + scipy_expected.z).abs() <= tol;
        let zero_ok = rust_actual.angle <= tol && scipy_expected.angle <= tol;
        assert!(
            (angle_ok && (axis_ok || zero_ok)) || (equiv_ok && !zero_ok),
            "AxisAngle: Rust got {:?}, Scipy expected {:?}",
            rust_actual,
            scipy_expected
        );
    }

    fn assert_rotation_vector_near(rust_actual: &RotationVector, scipy_expected: &RotationVector, tol: f32) {
        let norm_a = (rust_actual.x * rust_actual.x + rust_actual.y * rust_actual.y + rust_actual.z * rust_actual.z).sqrt();
        let norm_b = (scipy_expected.x * scipy_expected.x + scipy_expected.y * scipy_expected.y + scipy_expected.z * scipy_expected.z).sqrt();
        let zero_ok = norm_a <= tol && norm_b <= tol;
        let vec_ok = (rust_actual.x - scipy_expected.x).abs() <= tol
            && (rust_actual.y - scipy_expected.y).abs() <= tol
            && (rust_actual.z - scipy_expected.z).abs() <= tol;
        // v ≡ -v when |v| = π (rotation of π around axis = rotation of π around -axis)
        let pi = std::f32::consts::PI;
        let near_pi = (norm_a - pi).abs() <= tol && (norm_b - pi).abs() <= tol;
        let vec_neg_ok = near_pi
            && (rust_actual.x + scipy_expected.x).abs() <= tol
            && (rust_actual.y + scipy_expected.y).abs() <= tol
            && (rust_actual.z + scipy_expected.z).abs() <= tol;
        assert!(
            zero_ok || vec_ok || vec_neg_ok,
            "RotationVector: Rust got {:?}, Scipy expected {:?}",
            rust_actual,
            scipy_expected
        );
    }

    fn assert_rotation_matrix_near(rust_actual: &RotationMatrix, scipy_expected: &RotationMatrix, tol: f32) {
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (rust_actual[i][j] - scipy_expected[i][j]).abs() <= tol,
                    "RotationMatrix[{i}][{j}]: Rust got {}, Scipy expected {}",
                    rust_actual[i][j],
                    scipy_expected[i][j]
                );
            }
        }
    }

'''
    test_bodies = "\n\n".join(_rust_case(c) for c in cases)
    return header + test_bodies + "\n}\n"


def main():
    parser = argparse.ArgumentParser(
        description="Generate rotation test cases using scipy for Rust validation."
    )
    parser.add_argument(
        "-o", "--output",
        type=Path,
        default=None,
        help="Output Rust file path. Default: stdout.",
    )
    args = parser.parse_args()

    cases = generate_all_test_cases()
    rust_code = _generate_rust_module(cases)

    if args.output:
        args.output.write_text(rust_code)
        print(f"Wrote {len(cases)} test cases to {args.output}", file=sys.stderr)
    else:
        print(rust_code)


if __name__ == "__main__":
    main()
