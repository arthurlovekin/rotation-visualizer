use std::ops::{Mul};
use std::cmp::PartialEq;
use std::ops::{Index, IndexMut};

/// When sin(angle/2) < this, we treat the quaternion as near-identity (angle ≈ 2π)
/// to avoid division by near-zero. Using 4× EPSILON (~4.8e-7) preserves more precision
/// than 1e-6 while remaining numerically stable for f32.
const NEAR_IDENTITY_S_THRESHOLD: f32 = 1.0 * f32::EPSILON;

/// When trace < this in matrix-to-quaternion conversion, use alternative branches to avoid
/// division by near-zero (trace ≈ 0 occurs for ~180° rotations). Minimal epsilon for f32
/// numerical stability.
const MATRIX_TO_QUAT_TRACE_THRESHOLD: f32 = 4.0 * f32::EPSILON;

#[derive(Debug, Clone, Copy)]
pub struct Quaternion {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Quaternion {
    /// Create a new unit quaternion (normalizes the input).
    /// Panics if the input is a zero quaternion.
    pub fn new(w: f32, x: f32, y: f32, z: f32) -> Self {
        Self::try_new(w, x, y, z).unwrap_or_else(|e| panic!("{}", e))
    }

    /// Try to create a new unit quaternion. Returns Err if the norm is zero.
    pub fn try_new(w: f32, x: f32, y: f32, z: f32) -> Result<Self, String> {
        let norm_sq = w * w + x * x + y * y + z * z;
        if norm_sq == 0.0 {
            return Err("Quaternion is zero".to_string());
        }
        let norm = norm_sq.sqrt();
        Ok(Self {
            w: w / norm,
            x: x / norm,
            y: y / norm,
            z: z / norm,
        })
    }

    // Each quaternion has a dual that represents the same rotation 
    pub fn dual(&self) -> Self {
        Self { w: -self.w, x: -self.x, y: -self.y, z: -self.z }
    }
}

impl Default for Quaternion {
    /// The identity quaternion (no rotation).
    fn default() -> Self {
        Self { w: 1.0, x: 0.0, y: 0.0, z: 0.0 }
    }
}

impl Mul for Quaternion {
    type Output = Quaternion;

    fn mul(self, other: Quaternion) -> Quaternion {
        Quaternion::new(
            self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        )
    }
}

impl PartialEq for Quaternion {
    fn eq(&self, other: &Self) -> bool {
        (self.w == other.w && self.x == other.x && self.y == other.y && self.z == other.z) ||
        (self.w == -other.w && self.x == -other.x && self.y == -other.y && self.z == -other.z)
    }
}

impl From<AxisAngle> for Quaternion {
    fn from(axis_angle: AxisAngle) -> Self {
        let half = axis_angle.angle / 2.0;
        let s = half.sin();
        Self::new(half.cos(), axis_angle.x * s, axis_angle.y * s, axis_angle.z * s)
    }
}

impl From<RotationVector> for Quaternion {
    fn from(vector: RotationVector) -> Self {
        let norm = (vector.x * vector.x + vector.y * vector.y + vector.z * vector.z).sqrt();
        let axis_angle = AxisAngle::new(vector.x, vector.y, vector.z, norm);
        Self::from(axis_angle)
    }
}

impl From<RotationMatrix> for Quaternion {
    /// Numerically stable conversion. Uses the branch with largest denominator
    /// to avoid division by near-zero when trace ≈ 0 (180° rotations).
    fn from(matrix: RotationMatrix) -> Self {
        let (m00, m01, m02) = (matrix[0][0], matrix[0][1], matrix[0][2]);
        let (m10, m11, m12) = (matrix[1][0], matrix[1][1], matrix[1][2]);
        let (m20, m21, m22) = (matrix[2][0], matrix[2][1], matrix[2][2]);
        let trace = 1.0 + m00 + m11 + m22;
        let (w, x, y, z) = if trace > MATRIX_TO_QUAT_TRACE_THRESHOLD {
            let s = 2.0 * trace.sqrt();
            (
                s / 4.0,
                (m21 - m12) / s,
                (m02 - m20) / s,
                (m10 - m01) / s,
            )
        } else if m00 > m11 && m00 > m22 {
            let s = 2.0 * (1.0 + m00 - m11 - m22).sqrt();
            ((m21 - m12) / s, s / 4.0, (m01 + m10) / s, (m02 + m20) / s)
        } else if m11 > m22 {
            let s = 2.0 * (1.0 + m11 - m00 - m22).sqrt();
            ((m02 - m20) / s, (m01 + m10) / s, s / 4.0, (m12 + m21) / s)
        } else {
            let s = 2.0 * (1.0 + m22 - m00 - m11).sqrt();
            ((m10 - m01) / s, (m02 + m20) / s, (m12 + m21) / s, s / 4.0)
        };
        Self::new(w, x, y, z)
    }
}

impl From<EulerAngles> for Quaternion {
    fn from(e: EulerAngles) -> Self {
        Self::from(RotationMatrix::from(e))
    }
}

/// Euler rotation sequence (scipy convention).
/// - **Uppercase** {'X','Y','Z'}: intrinsic rotations (body frame)
/// - **Lowercase** {'x','y','z'}: extrinsic rotations (fixed frame)
/// - Extrinsic and intrinsic cannot be mixed in one call.
/// Macro: match canonical key string to EulerSequence variant.
/// Uses stringify! so the string is always in sync with the variant name.
macro_rules! match_euler_key {
    ($key:expr, $seq:expr, $($variant:ident),* $(,)?) => {
        match $key {
            $(stringify!($variant) => Ok(Self::$variant),)*
            _ => Err(format!("Unknown Euler sequence {:?}", $seq)),
        }
    };
}

/// Equivalent sequences share one variant: e.g. intrinsic XYZ ≡ extrinsic zyx → `XYZ_zyx`.
/// Tait-Bryan: all three axes different. Proper Euler: two axes the same.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EulerSequence {
    // Tait-Bryan
    XYZ_zyx,
    XZY_yzx,
    YXZ_zxy,
    YZX_xzy,
    ZXY_yxz,
    ZYX_xyz,
    // Proper Euler (intrinsic ABC = extrinsic cba)
    XYX_xyx,
    XZX_xzx,
    YXY_yxy,
    YZY_yzy,
    ZXZ_zxz,
    ZYZ_zyz,
}

impl EulerSequence {
    /// Human-readable label for dropdown, e.g. "XYZ (intrinsic) = zyx (extrinsic)".
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::XYZ_zyx => "XYZ (intrinsic) = zyx (extrinsic)",
            Self::XZY_yzx => "XZY (intrinsic) = yzx (extrinsic)",
            Self::YXZ_zxy => "YXZ (intrinsic) = zxy (extrinsic)",
            Self::YZX_xzy => "YZX (intrinsic) = xzy (extrinsic)",
            Self::ZXY_yxz => "ZXY (intrinsic) = yxz (extrinsic)",
            Self::ZYX_xyz => "ZYX (intrinsic) = xyz (extrinsic)",
            Self::XYX_xyx => "XYX (intrinsic) = xyx (extrinsic)",
            Self::XZX_xzx => "XZX (intrinsic) = xzx (extrinsic)",
            Self::YXY_yxy => "YXY (intrinsic) = yxy (extrinsic)",
            Self::YZY_yzy => "YZY (intrinsic) = yzy (extrinsic)",
            Self::ZXZ_zxz => "ZXZ (intrinsic) = zxz (extrinsic)",
            Self::ZYZ_zyz => "ZYZ (intrinsic) = zyz (extrinsic)",
        }
    }

    /// Parse a scipy-style sequence string.
    /// - Uppercase "XYZ", "ZYX", etc.: intrinsic (body frame)
    /// - Lowercase "xyz", "zyx", etc.: extrinsic (fixed frame)
    /// - Returns error if intrinsic and extrinsic are mixed (e.g. "XyZ").
    pub fn from_string(seq: &str) -> Result<Self, String> {
        let s = seq.trim();
        if s.is_empty() || s.len() > 3 {
            return Err(format!("Euler sequence must be 1-3 chars, got {:?}", seq));
        }
        let chars: Vec<char> = s.chars().collect();
        let all_upper = chars.iter().all(|c| matches!(c, 'X' | 'Y' | 'Z'));
        let all_lower = chars.iter().all(|c| matches!(c, 'x' | 'y' | 'z'));
        if !all_upper && !all_lower {
            return Err("Extrinsic and intrinsic rotations cannot be mixed in one sequence".to_string());
        }
        let normalized = if all_upper {
            s.to_uppercase()
        } else {
            s.to_lowercase()
        };
        // Build canonical key: intrinsic_extrinsic (e.g. XYZ_zyx).
        // Intrinsic = extrinsic reversed; so "XYZ" or "zyx" both yield "XYZ_zyx".
        let rev: String = normalized.chars().rev().collect();
        let key = if all_upper {
            format!("{}_{}", normalized, rev.to_lowercase())
        } else {
            format!("{}_{}", rev.to_uppercase(), normalized)
        };
        match_euler_key!(key.as_str(), seq,
            XYZ_zyx, XZY_yzx, YXZ_zxy, YZX_xzy, ZXY_yxz, ZYX_xyz,
            XYX_xyx, XZX_xzx, YXY_yxy, YZY_yzy, ZXZ_zxz, ZYZ_zyz
        )
    }
}

/// Euler angles in radians. Angles are (a, b, c) corresponding to the three axes in `sequence`.
#[derive(Debug, Clone, Copy)]
pub struct EulerAngles {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub sequence: EulerSequence,
}

impl EulerAngles {
    /// Create Euler angles from radians.
    pub fn new(a: f32, b: f32, c: f32, sequence: EulerSequence) -> Self {
        Self { a, b, c, sequence }
    }

    /// Create from angles in degrees. Converts to radians internally.
    pub fn from_degrees(a_deg: f32, b_deg: f32, c_deg: f32, sequence: EulerSequence) -> Self {
        Self::new(
            a_deg.to_radians(),
            b_deg.to_radians(),
            c_deg.to_radians(),
            sequence,
        )
    }

    /// Returns (a, b, c) with angles in degrees.
    pub fn as_degrees(&self) -> (f32, f32, f32) {
        (
            self.a.to_degrees(),
            self.b.to_degrees(),
            self.c.to_degrees(),
        )
    }

    /// Convert to the same rotation expressed in a different Euler sequence.
    /// Uses the rotation matrix as an intermediate representation to preserve the rotation.
    pub fn as_sequence(&self, new_sequence: EulerSequence) -> Self {
        let matrix = RotationMatrix::from(*self);
        Self::from_rotation_matrix(matrix, new_sequence)
    }

    /// Extract Euler angles from a rotation matrix in the given sequence.
    /// Handles gimbal lock by setting the third angle to zero at singularities.
    pub fn from_rotation_matrix(matrix: RotationMatrix, sequence: EulerSequence) -> Self {
        let m = &matrix.0;
        let (a, b, c) = match sequence {
            // XYZ: R_x*R_y*R_z — b=asin(r02), a=atan2(-r12,r22), c=atan2(-r01,r00)
            EulerSequence::XYZ_zyx => {
                let sb = m[0][2].clamp(-1.0, 1.0);
                let b = sb.asin();
                let cb = b.cos();
                if cb.abs() > 1e-6 {
                    let a = (-m[1][2]).atan2(m[2][2]);
                    let c = (-m[0][1]).atan2(m[0][0]);
                    (a, b, c)
                } else if m[0][2] > 0.0 {
                    let a = m[1][0].atan2(m[1][1]);
                    (a, std::f32::consts::FRAC_PI_2, 0.0)
                } else {
                    let a = (-m[1][0]).atan2(m[1][1]);
                    (a, -std::f32::consts::FRAC_PI_2, 0.0)
                }
            }
            // XZY: R_x*R_z*R_y — b=asin(-r01), a=atan2(r21,r11), c=atan2(r02,r00)
            EulerSequence::XZY_yzx => {
                let sb = (-m[0][1]).clamp(-1.0, 1.0);
                let b = sb.asin();
                let cb = b.cos();
                if cb.abs() > 1e-6 {
                    let a = m[2][1].atan2(m[1][1]);
                    let c = m[0][2].atan2(m[0][0]);
                    (a, b, c)
                } else if m[0][1] < 0.0 {
                    let a = (-m[2][0]).atan2(m[2][2]);
                    (a, std::f32::consts::FRAC_PI_2, 0.0)
                } else {
                    let a = m[2][0].atan2(-m[2][2]);
                    (a, -std::f32::consts::FRAC_PI_2, 0.0)
                }
            }
            // YXZ: R_y*R_x*R_z — b=asin(-r12), a=atan2(r02,r22), c=atan2(r10,r11)
            EulerSequence::YXZ_zxy => {
                let sb = (-m[1][2]).clamp(-1.0, 1.0);
                let b = sb.asin();
                let cb = b.cos();
                if cb.abs() > 1e-6 {
                    let a = m[0][2].atan2(m[2][2]);
                    let c = m[1][0].atan2(m[1][1]);
                    (a, b, c)
                } else if m[1][2] < 0.0 {
                    let a = m[0][1].atan2(m[0][0]);
                    (a, std::f32::consts::FRAC_PI_2, 0.0)
                } else {
                    let a = (-m[0][1]).atan2(m[0][0]);
                    (a, -std::f32::consts::FRAC_PI_2, 0.0)
                }
            }
            // YZX: R_y*R_z*R_x — b=asin(r10), a=atan2(-r20,r00), c=atan2(-r12,r11)
            EulerSequence::YZX_xzy => {
                let sb = m[1][0].clamp(-1.0, 1.0);
                let b = sb.asin();
                let cb = b.cos();
                if cb.abs() > 1e-6 {
                    let a = (-m[2][0]).atan2(m[0][0]);
                    let c = (-m[1][2]).atan2(m[1][1]);
                    (a, b, c)
                } else if m[1][0] > 0.0 {
                    let a = m[2][1].atan2(m[2][2]);
                    (a, std::f32::consts::FRAC_PI_2, 0.0)
                } else {
                    let a = (-m[2][1]).atan2(m[2][2]);
                    (a, -std::f32::consts::FRAC_PI_2, 0.0)
                }
            }
            // ZXY: R_z*R_x*R_y — b=asin(r21), a=atan2(-r01,r11), c=atan2(-r20,r22)
            EulerSequence::ZXY_yxz => {
                let sb = m[2][1].clamp(-1.0, 1.0);
                let b = sb.asin();
                let cb = b.cos();
                if cb.abs() > 1e-6 {
                    let a = (-m[0][1]).atan2(m[1][1]);
                    let c = (-m[2][0]).atan2(m[2][2]);
                    (a, b, c)
                } else if m[2][1] > 0.0 {
                    let a = m[0][2].atan2(m[0][0]);
                    (a, std::f32::consts::FRAC_PI_2, 0.0)
                } else {
                    let a = (-m[0][2]).atan2(m[0][0]);
                    (a, -std::f32::consts::FRAC_PI_2, 0.0)
                }
            }
            // ZYX: R_z*R_y*R_x — b=asin(-r20), a=atan2(r10,r00), c=atan2(r21,r22)
            EulerSequence::ZYX_xyz => {
                let sb = (-m[2][0]).clamp(-1.0, 1.0);
                let b = sb.asin();
                let cb = b.cos();
                if cb.abs() > 1e-6 {
                    let a = m[1][0].atan2(m[0][0]);
                    let c = m[2][1].atan2(m[2][2]);
                    (a, b, c)
                } else if m[2][0] < 0.0 {
                    // North pole: m02,m22≈0; use a=atan2(-m01,m11) with c=0
                    let a = if m[0][2].abs() < 1e-6 && m[2][2].abs() < 1e-6 {
                        (-m[0][1]).atan2(m[1][1])
                    } else {
                        m[0][2].atan2(m[2][2])
                    };
                    (a, std::f32::consts::FRAC_PI_2, 0.0)
                } else {
                    // South pole
                    let a = if m[0][2].abs() < 1e-6 && m[2][2].abs() < 1e-6 {
                        m[0][1].atan2(m[1][1])
                    } else {
                        (-m[0][2]).atan2(m[2][2])
                    };
                    (a, -std::f32::consts::FRAC_PI_2, 0.0)
                }
            }
            // Proper Euler: R = R_x(a)*R_y(b)*R_x(c)
            EulerSequence::XYX_xyx => {
                let cb = m[0][0].clamp(-1.0, 1.0);
                let b = cb.acos();
                let sb = b.sin();
                if sb.abs() > 1e-6 {
                    let a = m[1][0].atan2(-m[2][0]);
                    let c = m[0][1].atan2(m[0][2]);
                    (a, b, c)
                } else {
                    let a = (-cb * m[1][2]).atan2(m[1][1]);
                    let c = 0.0;
                    (a, b, c)
                }
            }
            // R = R_x(a)*R_z(b)*R_x(c)
            EulerSequence::XZX_xzx => {
                let cb = m[0][0].clamp(-1.0, 1.0);
                let b = cb.acos();
                let sb = b.sin();
                if sb.abs() > 1e-6 {
                    let a = m[2][0].atan2(m[1][0]);
                    let c = m[0][2].atan2(-m[0][1]);
                    (a, b, c)
                } else {
                    let a = (-cb * m[1][2]).atan2(m[1][1]);
                    let c = 0.0;
                    (a, b, c)
                }
            }
            // R = R_y(a)*R_x(b)*R_y(c)
            EulerSequence::YXY_yxy => {
                let cb = m[1][1].clamp(-1.0, 1.0);
                let b = cb.acos();
                let sb = b.sin();
                if sb.abs() > 1e-6 {
                    let a = m[0][1].atan2(m[2][1]);
                    let c = m[1][0].atan2(-m[1][2]);
                    (a, b, c)
                } else {
                    let a = 0.0;
                    let c = (-m[2][0]).atan2(m[0][0]);
                    (a, b, c)
                }
            }
            // R = R_y(a)*R_z(b)*R_y(c)
            EulerSequence::YZY_yzy => {
                let cb = m[1][1].clamp(-1.0, 1.0);
                let b = cb.acos();
                let sb = b.sin();
                if sb.abs() > 1e-6 {
                    let a = m[2][1].atan2(-m[0][1]);
                    let c = m[1][2].atan2(m[1][0]);
                    (a, b, c)
                } else {
                    let a = 0.0;
                    let c = m[2][0].atan2(m[0][0]);
                    (a, b, c)
                }
            }
            // R = R_z(a)*R_x(b)*R_z(c)
            EulerSequence::ZXZ_zxz => {
                let cb = m[2][2].clamp(-1.0, 1.0);
                let b = cb.acos();
                let sb = b.sin();
                if sb.abs() > 1e-6 {
                    let a = m[0][2].atan2(-m[1][2]);
                    let c = m[2][0].atan2(m[2][1]);
                    (a, b, c)
                } else {
                    let a = 0.0;
                    let c = m[0][1].atan2(m[0][0]);
                    (a, b, c)
                }
            }
            // R = R_z(a)*R_y(b)*R_z(c)
            EulerSequence::ZYZ_zyz => {
                let cb = m[2][2].clamp(-1.0, 1.0);
                let b = cb.acos();
                let sb = b.sin();
                if sb.abs() > 1e-6 {
                    let a = m[1][2].atan2(m[0][2]);
                    let c = m[2][1].atan2(-m[2][0]);
                    (a, b, c)
                } else {
                    let a = (-cb * m[2][0]).atan2(m[2][2]);
                    let c = 0.0;
                    (a, b, c)
                }
            }
        };
        Self::new(a, b, c, sequence)
    }
}

/// Default Euler sequence used by `From<RotationMatrix> for EulerAngles`.
/// Intrinsic ZYX = extrinsic xyz; common for roll-pitch-yaw / aerospace.
pub const DEFAULT_EULER_SEQUENCE: EulerSequence = EulerSequence::ZYX_xyz;

impl From<RotationMatrix> for EulerAngles {
    /// Convert rotation matrix to Euler angles using [`DEFAULT_EULER_SEQUENCE`].
    /// Use [`EulerAngles::from_rotation_matrix`] to specify a different sequence.
    fn from(matrix: RotationMatrix) -> Self {
        EulerAngles::from_rotation_matrix(matrix, DEFAULT_EULER_SEQUENCE)
    }
}

impl From<Quaternion> for EulerAngles {
    /// Convert quaternion to Euler angles using [`DEFAULT_EULER_SEQUENCE`] (roll-pitch-yaw).
    /// Use [`EulerAngles::from_rotation_matrix`] with a rotation matrix to specify a different sequence.
    fn from(quat: Quaternion) -> Self {
        EulerAngles::from(RotationMatrix::from(quat))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AxisAngle {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub angle: f32,
}

impl AxisAngle {
    /// Create a new axis-angle representation, where the axis is a unit vector and the angle is in radians from [0, π].
    pub fn new(x: f32, y: f32, z: f32, angle: f32) -> Self {
        Self::try_new(x, y, z, angle).unwrap_or_else(|e| panic!("{}", e))
    }

    /// Create from axis and angle in degrees. Converts to radians internally.
    pub fn from_degrees(x: f32, y: f32, z: f32, angle_deg: f32) -> Self {
        Self::new(x, y, z, angle_deg.to_radians())
    }

    /// Returns (x, y, z, angle) with the angle in degrees.
    pub fn as_degrees(&self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.z, self.angle.to_degrees())
    }

    pub fn try_new(x: f32, y: f32, z: f32, angle: f32) -> Result<Self, String> {
        // First bring angle into range [0, 2*pi) with modulo.
        // Rust's % is remainder (truncating division): result has same sign as dividend.
        let mut new_angle = angle % (2.0 * std::f32::consts::PI);
        if new_angle < 0.0 {
            new_angle += 2.0 * std::f32::consts::PI;
        }

        // Axis-angle is a double-cover: (axis, angle) with angle in [pi, 2*pi) is equivalent to
        // (-axis, 2*pi - angle) with angle in (0, pi]. Flip axis when angle >= pi to get [0, pi].
        let mut ax = x;
        let mut ay = y;
        let mut az = z;
        if new_angle >= std::f32::consts::PI {
            new_angle = 2.0 * std::f32::consts::PI - new_angle;
            ax = -x;
            ay = -y;
            az = -z;
        }

        let axis_norm_sq = ax * ax + ay * ay + az * az;
        if axis_norm_sq == 0.0 {
            if new_angle == 0.0 {
                // axis doesn't matter, so pick an arbitrary unit-vector so axis will be continuous
                return Ok(Self { x: 1.0, y: 0.0, z: 0.0, angle: 0.0 });
            } else {
                return Err("Axis norm cannot be zero unless angle is zero".to_string());
            }
        }
        let axis_norm = axis_norm_sq.sqrt();
        Ok(
            Self { 
                x: ax / axis_norm, 
                y: ay / axis_norm, 
                z: az / axis_norm, 
                angle: new_angle 
            }
        )
    }
}

impl From<Quaternion> for AxisAngle {
    fn from(quat: Quaternion) -> Self {
        // sin(angle/2) = (1.0 - quat.w * quat.w).sqrt()
        let p = 1.0 - quat.w * quat.w;
        if p - NEAR_IDENTITY_S_THRESHOLD <= 0.0 {
            return Self::new(0.0, 0.0, 0.0, 0.0);
        }
        let s = p.sqrt();
        let angle = 2.0 * quat.w.acos();
        Self::new(quat.x / s, quat.y / s, quat.z / s, angle)
    }
}

impl PartialEq for AxisAngle {
    fn eq(&self, other: &Self) -> bool {
       (self.angle == 0.0 && other.angle == 0.0) ||
       (self.angle == other.angle && self.x == other.x && self.y == other.y && self.z == other.z)
    }
}

// Rotation Vector: 3-dimensional vector which is co-directional to the axis of rotation and whose norm gives the angle of rotation
#[derive(Debug, Clone, Copy)]
pub struct RotationVector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl RotationVector {
    /// Create a rotation vector where the norm is the angle in radians.
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        // find the norm that is between 0 and 2π
        let norm_sq = x * x + y * y + z * z;
        if norm_sq == 0.0 {
            Self { x: 0.0, y: 0.0, z: 0.0 }
        }
        else {
            let norm = (norm_sq).sqrt();
            let new_norm = (norm) % (2.0 * std::f32::consts::PI);
            let norm_ratio = new_norm / norm;
            Self {
                x: x * norm_ratio,
                y: y * norm_ratio,
                z: z * norm_ratio,
            }
        }
    }

    /// Create from components where the norm is the angle in degrees. Converts to radians internally.
    pub fn from_degrees(x: f32, y: f32, z: f32) -> Self {
        const DEG_TO_RAD: f32 = std::f32::consts::PI / 180.0;
        Self::new(x * DEG_TO_RAD, y * DEG_TO_RAD, z * DEG_TO_RAD)
    }

    /// Returns a rotation vector where the norm is the angle in degrees.
    pub fn as_degrees(&self) -> Self {
        const RAD_TO_DEG: f32 = 180.0 / std::f32::consts::PI;
        Self {
            x: self.x * RAD_TO_DEG,
            y: self.y * RAD_TO_DEG,
            z: self.z * RAD_TO_DEG,
        }
    }
}

impl Index<usize> for RotationVector {
    type Output = f32;

    #[inline]
    fn index(&self, row: usize) -> &Self::Output {
        match row {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => panic!("index out of bounds: the len is 3 but the index is {}", row),
        }
    }
}

impl IndexMut<usize> for RotationVector {
    #[inline]
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        match row {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => panic!("index out of bounds: the len is 3 but the index is {}", row),
        }
    }
}

impl Default for RotationVector {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }
}

impl From<Quaternion> for RotationVector {
    fn from(quat: Quaternion) -> Self {
        let axis_angle = AxisAngle::from(quat);
        Self::new(
            axis_angle.x*axis_angle.angle,
            axis_angle.y*axis_angle.angle,
            axis_angle.z*axis_angle.angle
        )
    }
}

#[derive(Clone, Copy)]
pub struct RotationMatrix(pub [[f32; 3]; 3]);

impl Index<usize> for RotationMatrix {
    type Output = [f32; 3];

    #[inline]
    fn index(&self, row: usize) -> &Self::Output {
        &self.0[row]
    }
}

impl IndexMut<usize> for RotationMatrix {
    #[inline]
    fn index_mut(&mut self, row: usize) -> &mut Self::Output {
        &mut self.0[row]
    }
}

impl Default for RotationMatrix {
    fn default() -> Self {
        Self([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]])
    }
}

impl Mul for RotationMatrix {
    type Output = RotationMatrix;

    fn mul(self, other: RotationMatrix) -> RotationMatrix {
        let mut result = RotationMatrix([[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]]);
        for i in 0..3 {
            for j in 0..3 {
                result[i][j] = 0.0;
                for k in 0..3 {
                    result[i][j] += self[i][k] * other[k][j];
                }
            }
        }
        result
    }
}

impl From<Quaternion> for RotationMatrix {
    fn from(quat: Quaternion) -> Self {
        RotationMatrix([
            [
                1.0 - 2.0 * (quat.y * quat.y + quat.z * quat.z),
                2.0 * (quat.x * quat.y - quat.w * quat.z),
                2.0 * (quat.x * quat.z + quat.w * quat.y),
            ],
            [
                2.0 * (quat.x * quat.y + quat.w * quat.z),
                1.0 - 2.0 * (quat.x * quat.x + quat.z * quat.z),
                2.0 * (quat.y * quat.z - quat.w * quat.x),
            ],
            [
                2.0 * (quat.x * quat.z - quat.w * quat.y),
                2.0 * (quat.y * quat.z + quat.w * quat.x),
                1.0 - 2.0 * (quat.x * quat.x + quat.y * quat.y),
            ],
        ])
    }
}

fn rot_x(a: f32) -> RotationMatrix {
    let (c, s) = (a.cos(), a.sin());
    RotationMatrix([
        [1.0, 0.0, 0.0],
        [0.0, c, -s],
        [0.0, s, c],
    ])
}

fn rot_y(a: f32) -> RotationMatrix {
    let (c, s) = (a.cos(), a.sin());
    RotationMatrix([
        [c, 0.0, s],
        [0.0, 1.0, 0.0],
        [-s, 0.0, c],
    ])
}

fn rot_z(a: f32) -> RotationMatrix {
    let (c, s) = (a.cos(), a.sin());
    RotationMatrix([
        [c, -s, 0.0],
        [s, c, 0.0],
        [0.0, 0.0, 1.0],
    ])
}

impl From<EulerAngles> for RotationMatrix {
    fn from(e: EulerAngles) -> Self {
        match e.sequence {
            EulerSequence::XYZ_zyx => rot_x(e.a) * rot_y(e.b) * rot_z(e.c),
            EulerSequence::XZY_yzx => rot_x(e.a) * rot_z(e.b) * rot_y(e.c),
            EulerSequence::YXZ_zxy => rot_y(e.a) * rot_x(e.b) * rot_z(e.c),
            EulerSequence::YZX_xzy => rot_y(e.a) * rot_z(e.b) * rot_x(e.c),
            EulerSequence::ZXY_yxz => rot_z(e.a) * rot_x(e.b) * rot_y(e.c),
            EulerSequence::ZYX_xyz => rot_z(e.a) * rot_y(e.b) * rot_x(e.c),
            EulerSequence::XYX_xyx => rot_x(e.a) * rot_y(e.b) * rot_x(e.c),
            EulerSequence::XZX_xzx => rot_x(e.a) * rot_z(e.b) * rot_x(e.c),
            EulerSequence::YXY_yxy => rot_y(e.a) * rot_x(e.b) * rot_y(e.c),
            EulerSequence::YZY_yzy => rot_y(e.a) * rot_z(e.b) * rot_y(e.c),
            EulerSequence::ZXZ_zxz => rot_z(e.a) * rot_x(e.b) * rot_z(e.c),
            EulerSequence::ZYZ_zyz => rot_z(e.a) * rot_y(e.b) * rot_z(e.c),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotation {
    quat: Quaternion,
}

impl Default for Rotation {
    fn default() -> Self {
        Self { quat: Quaternion::default() }
    }
}

impl Rotation {
    pub fn as_quaternion(&self) -> Quaternion {
        self.quat
    }

    pub fn as_axis_angle(&self) -> AxisAngle {
        AxisAngle::from(self.quat)
    }

    pub fn as_rotation_vector(&self) -> RotationVector {
        RotationVector::from(self.quat)
    }

    pub fn as_rotation_matrix(&self) -> RotationMatrix {
        RotationMatrix::from(self.quat)
    }

    /// Extract Euler angles in the given sequence.
    pub fn as_euler_angles(&self, sequence: EulerSequence) -> EulerAngles {
        let matrix = RotationMatrix::from(self.quat);
        EulerAngles::from_rotation_matrix(matrix, sequence)
    }
}

impl From<Quaternion> for Rotation {
    fn from(quat: Quaternion) -> Self {
        Rotation { quat }
    }
}

impl From<AxisAngle> for Rotation {
    fn from(axis_angle: AxisAngle) -> Self {
        Rotation { quat: Quaternion::from(axis_angle) }
    }
}

impl From<RotationMatrix> for Rotation {
    fn from(matrix: RotationMatrix) -> Self {
        Rotation {
            quat: Quaternion::from(matrix),
        }
    }
}

impl From<RotationVector> for Rotation {
    fn from(vector: RotationVector) -> Self {
        Rotation { quat: Quaternion::from(vector) }
    }
}

impl From<EulerAngles> for Rotation {
    fn from(euler: EulerAngles) -> Self {
        Rotation {
            quat: Quaternion::from(euler),
        }
    }
}

impl Mul for Rotation {
    type Output = Rotation;

    fn mul(self, other: Rotation) -> Rotation {
        Rotation {
            quat: self.quat * other.quat,
        }
    }
}


#[cfg(test)]
mod euler_tests {
    use super::*;

    const TOL: f32 = 1e-5;

    /// Assert that two Euler angle representations describe the same rotation
    /// (by converting both to quaternion and comparing).
    fn assert_euler_same_rotation(e1: &EulerAngles, e2: &EulerAngles) {
        let q1 = Rotation::from(*e1).as_quaternion();
        let q2 = Rotation::from(*e2).as_quaternion();
        let q_ok = (q1.w - q2.w).abs() <= TOL && (q1.x - q2.x).abs() <= TOL
            && (q1.y - q2.y).abs() <= TOL && (q1.z - q2.z).abs() <= TOL;
        let dual_ok = (q1.w + q2.w).abs() <= TOL && (q1.x + q2.x).abs() <= TOL
            && (q1.y + q2.y).abs() <= TOL && (q1.z + q2.z).abs() <= TOL;
        assert!(q_ok || dual_ok, "Euler {:?} vs {:?} produce different rotations", e1, e2);
    }

    #[test]
    fn euler_from_degrees_as_degrees() {
        let e_rad = EulerAngles::new(
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::FRAC_PI_4,
            std::f32::consts::PI / 3.0,
            EulerSequence::ZYX_xyz,
        );
        let e_deg = EulerAngles::from_degrees(90.0, 45.0, 60.0, EulerSequence::ZYX_xyz);
        assert_euler_same_rotation(&e_rad, &e_deg);
        let (a, b, c) = e_deg.as_degrees();
        assert!((a - 90.0).abs() <= TOL, "a deg: got {} expected 90", a);
        assert!((b - 45.0).abs() <= TOL, "b deg: got {} expected 45", b);
        assert!((c - 60.0).abs() <= TOL, "c deg: got {} expected 60", c);
    }

    #[test]
    fn euler_round_trip_zyx() {
        let e_orig = EulerAngles::new(0.5, 0.3, 0.2, EulerSequence::ZYX_xyz);
        let r = Rotation::from(e_orig);
        let e_extracted = r.as_euler_angles(EulerSequence::ZYX_xyz);
        assert_euler_same_rotation(&e_orig, &e_extracted);
    }

    #[test]
    fn euler_round_trip_xyz() {
        let e_orig = EulerAngles::new(0.5, 0.3, 0.2, EulerSequence::XYZ_zyx);
        let r = Rotation::from(e_orig);
        let e_extracted = r.as_euler_angles(EulerSequence::XYZ_zyx);
        assert_euler_same_rotation(&e_orig, &e_extracted);
    }

    /// Round-trip for all Tait-Bryan sequences.
    #[test]
    fn euler_round_trip_all_tait_bryan() {
        let sequences = [
            EulerSequence::XYZ_zyx,
            EulerSequence::XZY_yzx,
            EulerSequence::YXZ_zxy,
            EulerSequence::YZX_xzy,
            EulerSequence::ZXY_yxz,
            EulerSequence::ZYX_xyz,
        ];
        let angles = (0.5_f32, 0.3, 0.2);
        for seq in sequences {
            let e_orig = EulerAngles::new(angles.0, angles.1, angles.2, seq);
            let r = Rotation::from(e_orig);
            let e_extracted = r.as_euler_angles(seq);
            assert_euler_same_rotation(&e_orig, &e_extracted);
        }
    }

    /// Round-trip for all Proper Euler sequences.
    #[test]
    fn euler_round_trip_proper_euler() {
        let sequences = [
            EulerSequence::XYX_xyx,
            EulerSequence::XZX_xzx,
            EulerSequence::YXY_yxy,
            EulerSequence::YZY_yzy,
            EulerSequence::ZXZ_zxz,
            EulerSequence::ZYZ_zyz,
        ];
        let angles = (0.4_f32, 0.6, 0.2);
        for seq in sequences {
            let e_orig = EulerAngles::new(angles.0, angles.1, angles.2, seq);
            let r = Rotation::from(e_orig);
            let e_extracted = r.as_euler_angles(seq);
            assert_euler_same_rotation(&e_orig, &e_extracted);
        }
    }

    #[test]
    fn euler_as_sequence_preserves_rotation() {
        let e_zyx = EulerAngles::new(0.5, 0.3, 0.2, EulerSequence::ZYX_xyz);
        let e_xyz = e_zyx.as_sequence(EulerSequence::XYZ_zyx);
        assert_euler_same_rotation(&e_zyx, &e_xyz);
    }

    #[test]
    fn euler_from_rotation_matrix_identity() {
        for seq in [
            EulerSequence::ZYX_xyz,
            EulerSequence::XYZ_zyx,
            EulerSequence::ZXZ_zxz,
        ] {
            let mat = RotationMatrix::default();
            let e = EulerAngles::from_rotation_matrix(mat, seq);
            let r = Rotation::from(e);
            assert_quaternion_near(&r.as_quaternion(), &Quaternion::default(), TOL);
        }
    }

    #[test]
    fn euler_from_rotation_matrix_90z() {
        let mat = RotationMatrix([
            [0.0, -1.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
        ]);
        let e = EulerAngles::from_rotation_matrix(mat, EulerSequence::ZYX_xyz);
        let r = Rotation::from(e);
        let expected = Quaternion::new(
            0.70710677,
            0.0,
            0.0,
            0.70710677,
        );
        assert_quaternion_near(&r.as_quaternion(), &expected, TOL);
    }

    #[test]
    fn euler_from_scipy_seq() {
        // Scipy: uppercase=intrinsic, lowercase=extrinsic. XYZ_zyx = intrinsic XYZ = extrinsic zyx.
        assert_eq!(
            EulerSequence::from_string("ZYX").unwrap(),
            EulerSequence::ZYX_xyz
        );
        assert_eq!(
            EulerSequence::from_string("xyz").unwrap(),
            EulerSequence::ZYX_xyz
        );
        assert_eq!(
            EulerSequence::from_string("XYZ").unwrap(),
            EulerSequence::XYZ_zyx
        );
        assert_eq!(
            EulerSequence::from_string("zyx").unwrap(),
            EulerSequence::XYZ_zyx
        );
        assert!(EulerSequence::from_string("XyZ").is_err());
        assert!(EulerSequence::from_string("").is_err());
    }

    #[test]
    fn euler_gimbal_lock_extraction() {
        // Rotation that causes gimbal lock in ZYX: (90, 90, 0) degrees
        let e_orig = EulerAngles::from_degrees(90.0, 90.0, 0.0, EulerSequence::ZYX_xyz);
        let r = Rotation::from(e_orig);
        let e_extracted = r.as_euler_angles(EulerSequence::ZYX_xyz);
        // At gimbal lock, third angle is set to 0; rotation should still match
        assert_euler_same_rotation(&e_orig, &e_extracted);
    }

    fn assert_quaternion_near(actual: &Quaternion, expected: &Quaternion, tol: f32) {
        let q_ok = (actual.w - expected.w).abs() <= tol && (actual.x - expected.x).abs() <= tol
            && (actual.y - expected.y).abs() <= tol && (actual.z - expected.z).abs() <= tol;
        let dual_ok = (actual.w + expected.w).abs() <= tol && (actual.x + expected.x).abs() <= tol
            && (actual.y + expected.y).abs() <= tol && (actual.z + expected.z).abs() <= tol;
        assert!(q_ok || dual_ok, "Quaternion: got {:?}, expected {:?}", actual, expected);
    }

    /// Focused test: YZX round-trip. EulerAngles (a,b,c) -> RotationMatrix -> from_rotation_matrix -> same rotation?
    #[test]
    fn from_rotation_matrix_yzx_round_trip() {
        let e_orig = EulerAngles::new(0.5, 0.3, 0.2, EulerSequence::YZX_xzy);
        let mat = RotationMatrix::from(e_orig);
        let e_extracted = EulerAngles::from_rotation_matrix(mat, EulerSequence::YZX_xzy);
        assert_euler_same_rotation(&e_orig, &e_extracted);
    }

    /// Focused test: ZYZ round-trip (proper Euler).
    #[test]
    fn from_rotation_matrix_zyz_round_trip() {
        let e_orig = EulerAngles::new(0.4, 0.6, 0.2, EulerSequence::ZYZ_zyz);
        let mat = RotationMatrix::from(e_orig);
        let e_extracted = EulerAngles::from_rotation_matrix(mat, EulerSequence::ZYZ_zyz);
        assert_euler_same_rotation(&e_orig, &e_extracted);
    }

    /// Scipy convention: EulerAngles (a,b,c) for sequence ABC produces R = R_A(a)*R_B(b)*R_C(c).
    /// This test verifies ZYX matches scipy's from_euler('ZYX') for axis_angle_dir0_angle1.
    #[test]
    fn euler_from_scipy_zyx_convention() {
        let zyx_angles = (0.5612027282726585_f32, 0.3158423005321318, 0.5612027282726585);
        let expected_quat = Quaternion::new(0.9238795, 0.22094238, 0.22094238, 0.22094238);
        let e = EulerAngles::new(zyx_angles.0, zyx_angles.1, zyx_angles.2, EulerSequence::ZYX_xyz);
        let r = Rotation::from(e);
        assert_quaternion_near(&r.as_quaternion(), &expected_quat, TOL);
    }
}

/// Tests for RotationMatrix -> Quaternion conversion (numerically stable Shepperd's method).
#[cfg(test)]
mod matrix_to_quat_tests {
    use super::*;

    /// Standard tolerance for most cases.
    const TOL: f32 = 1e-5;
    /// Relaxed tolerance for near-180° rotations (numerically sensitive).
    const TOL_NEAR_180: f32 = 3e-4;

    fn assert_quat_same_rotation(actual: &Quaternion, expected: &Quaternion, tol: f32) {
        let q_ok = (actual.w - expected.w).abs() <= tol && (actual.x - expected.x).abs() <= tol
            && (actual.y - expected.y).abs() <= tol && (actual.z - expected.z).abs() <= tol;
        let dual_ok = (actual.w + expected.w).abs() <= tol && (actual.x + expected.x).abs() <= tol
            && (actual.y + expected.y).abs() <= tol && (actual.z + expected.z).abs() <= tol;
        assert!(
            q_ok || dual_ok,
            "Quaternions represent different rotations: actual {:?}, expected {:?}",
            actual,
            expected
        );
    }

    fn assert_matrix_equals_quat(matrix: &RotationMatrix, quat: &Quaternion, tol: f32) {
        let reconstructed = RotationMatrix::from(*quat);
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (matrix[i][j] - reconstructed[i][j]).abs() <= tol,
                    "Matrix mismatch at [{},{}]: got {} expected {}",
                    i,
                    j,
                    reconstructed[i][j],
                    matrix[i][j]
                );
            }
        }
    }

    #[test]
    fn matrix_to_quat_identity() {
        let mat = RotationMatrix::default();
        let q = Quaternion::from(mat);
        assert_quat_same_rotation(&q, &Quaternion::default(), TOL);
    }

    #[test]
    fn matrix_to_quat_90_about_axes() {
        // 90° about X, Y, Z (ZYX: a=Z, b=Y, c=X so c=π/2 gives X, b=π/2 gives Y, a=π/2 gives Z)
        let seq = EulerSequence::ZYX_xyz;
        let half = std::f32::consts::FRAC_PI_2;

        // 90° about X: (a=0, b=0, c=π/2)
        let mat_x = RotationMatrix::from(EulerAngles::new(0.0, 0.0, half, seq));
        let q_x = Quaternion::from(mat_x);
        let expected_x = Quaternion::new(0.70710677, 0.70710677, 0.0, 0.0);
        assert_quat_same_rotation(&q_x, &expected_x, TOL);
        assert_matrix_equals_quat(&mat_x, &q_x, TOL);

        // 90° about Y: (a=0, b=π/2, c=0)
        let mat_y = RotationMatrix::from(EulerAngles::new(0.0, half, 0.0, seq));
        let q_y = Quaternion::from(mat_y);
        let expected_y = Quaternion::new(0.70710677, 0.0, 0.70710677, 0.0);
        assert_quat_same_rotation(&q_y, &expected_y, TOL);
        assert_matrix_equals_quat(&mat_y, &q_y, TOL);

        // 90° about Z: (a=π/2, b=0, c=0)
        let mat_z = RotationMatrix::from(EulerAngles::new(half, 0.0, 0.0, seq));
        let q_z = Quaternion::from(mat_z);
        let expected_z = Quaternion::new(0.70710677, 0.0, 0.0, 0.70710677);
        assert_quat_same_rotation(&q_z, &expected_z, TOL);
        assert_matrix_equals_quat(&mat_z, &q_z, TOL);
    }

    /// Critical case: 180° rotations have trace = 0, triggering alternative branches.
    #[test]
    fn matrix_to_quat_180_about_axes() {
        let pi = std::f32::consts::PI;
        let seq = EulerSequence::ZYX_xyz;

        // 180° about X: trace = 0, uses m00 branch
        let mat_x = RotationMatrix::from(EulerAngles::new(0.0, 0.0, pi, seq));
        let q_x = Quaternion::from(mat_x);
        let expected_x = Quaternion::new(0.0, 1.0, 0.0, 0.0);
        assert_quat_same_rotation(&q_x, &expected_x, TOL);
        assert_matrix_equals_quat(&mat_x, &q_x, TOL);

        // 180° about Y: trace = 0, uses m11 branch
        let mat_y = RotationMatrix::from(EulerAngles::new(0.0, pi, 0.0, seq));
        let q_y = Quaternion::from(mat_y);
        let expected_y = Quaternion::new(0.0, 0.0, 1.0, 0.0);
        assert_quat_same_rotation(&q_y, &expected_y, TOL);
        assert_matrix_equals_quat(&mat_y, &q_y, TOL);

        // 180° about Z: trace = 0, uses m22 branch
        let mat_z = RotationMatrix::from(EulerAngles::new(pi, 0.0, 0.0, seq));
        let q_z = Quaternion::from(mat_z);
        let expected_z = Quaternion::new(0.0, 0.0, 0.0, 1.0);
        assert_quat_same_rotation(&q_z, &expected_z, TOL);
        assert_matrix_equals_quat(&mat_z, &q_z, TOL);
    }

    /// Round-trip: quaternion -> matrix -> quaternion must preserve the rotation.
    #[test]
    fn matrix_to_quat_round_trip() {
        let test_quats = [
            (Quaternion::new(1.0, 0.0, 0.0, 0.0), TOL),
            (Quaternion::new(0.9238795, 0.22094238, 0.22094238, 0.22094238), TOL),
            (Quaternion::new(0.5, 0.5, 0.5, 0.5), TOL),
            (Quaternion::new(0.0, 1.0, 0.0, 0.0), TOL),
            (Quaternion::new(0.0, 0.0, 1.0, 0.0), TOL),
            (Quaternion::new(0.0, 0.0, 0.0, 1.0), TOL),
            (Quaternion::new(0.001, 0.999, 0.001, 0.001), TOL_NEAR_180), // Near 180° about X
            (Quaternion::new(0.001, 0.001, 0.999, 0.001), TOL_NEAR_180), // Near 180° about Y
            (Quaternion::new(0.001, 0.001, 0.001, 0.999), TOL_NEAR_180), // Near 180° about Z
        ];
        for (q_orig, tol) in test_quats {
            let mat = RotationMatrix::from(q_orig);
            let q_reconstructed = Quaternion::from(mat);
            assert_quat_same_rotation(&q_reconstructed, &q_orig, tol);
            assert_matrix_equals_quat(&mat, &q_reconstructed, tol);
        }
    }

    /// Stress test: angles near 180° to exercise branch boundaries.
    #[test]
    fn matrix_to_quat_near_180_degrees() {
        let seq = EulerSequence::ZYX_xyz;
        for angle_deg in [179.0_f32, 179.5, 179.9, 180.0, 180.1, 181.0] {
            let angle_rad = angle_deg.to_radians();
            // Pure X, Y, Z rotations via Euler (a=Z, b=Y, c=X)
            let euler_configs = [
                (0.0, 0.0, angle_rad),   // X
                (0.0, angle_rad, 0.0),   // Y
                (angle_rad, 0.0, 0.0),   // Z
            ];
            for (a, b, c) in euler_configs {
                let mat = RotationMatrix::from(EulerAngles::new(a, b, c, seq));
                let q = Quaternion::from(mat);
                let q_via_euler = Quaternion::from(EulerAngles::new(a, b, c, seq));
                assert_quat_same_rotation(&q, &q_via_euler, TOL_NEAR_180);
                assert_matrix_equals_quat(&mat, &q, TOL_NEAR_180);
            }
        }
    }

    /// Sweep over many axis-angle rotations to ensure all branches behave correctly.
    #[test]
    fn matrix_to_quat_axis_angle_sweep() {
        let axes = [(1.0_f32, 0.0, 0.0), (0.0, 1.0, 0.0), (0.0, 0.0, 1.0)];
        let angles: Vec<f32> = (0..=36)
            .map(|i| (i as f32) * std::f32::consts::PI / 18.0)
            .chain([std::f32::consts::PI - 0.01, std::f32::consts::PI, std::f32::consts::PI + 0.01])
            .collect();
        for (ax, ay, az) in axes {
            for &angle in &angles {
                let aa = AxisAngle::new(ax, ay, az, angle);
                let q_orig = Quaternion::from(aa);
                let mat = RotationMatrix::from(q_orig);
                let q_reconstructed = Quaternion::from(mat);
                let tol = if (angle - std::f32::consts::PI).abs() < 0.1 {
                    TOL_NEAR_180
                } else {
                    TOL
                };
                assert_quat_same_rotation(&q_reconstructed, &q_orig, tol);
                assert_matrix_equals_quat(&mat, &q_reconstructed, tol);
            }
        }
    }

    /// Arbitrary rotations via Euler angles to hit all four branches of the conversion.
    #[test]
    fn matrix_to_quat_euler_sweep() {
        let seq = EulerSequence::ZYX_xyz;
        for a in [0.0, 0.5, 1.0, 2.0, std::f32::consts::PI] {
            for b in [0.0, 0.3, 0.8, std::f32::consts::FRAC_PI_2, std::f32::consts::PI] {
                for c in [0.0, 0.2, 1.5] {
                    let e = EulerAngles::new(a, b, c, seq);
                    let q_orig = Quaternion::from(e);
                    let mat = RotationMatrix::from(q_orig);
                    let q_reconstructed = Quaternion::from(mat);
                    assert_quat_same_rotation(&q_reconstructed, &q_orig, TOL);
                }
            }
        }
    }
}

/// Tests rotation conversions against scipy-generated reference data.
#[cfg(test)]
mod scipy_tests {
    use super::*;
    use serde::Deserialize;

    const TOL: f32 = 1e-5;

    #[derive(Deserialize)]
    struct ScipyTestCase {
        name: String,
        quaternion: [f64; 4],
        rotation_matrix: [[f64; 3]; 3],
        axis_angle: [f64; 4],
        rotation_vector: [f64; 3],
        euler_angles: std::collections::HashMap<String, [f64; 3]>,
    }

    #[derive(Deserialize)]
    struct ScipyTestCases {
        test_cases: Vec<ScipyTestCase>,
    }

    fn assert_quat_near(
        actual: &Quaternion,
        expected: &Quaternion,
        tol: f32,
        case_name: &str,
        representation: &str,
    ) {
        let q_ok = (actual.w - expected.w).abs() <= tol && (actual.x - expected.x).abs() <= tol
            && (actual.y - expected.y).abs() <= tol && (actual.z - expected.z).abs() <= tol;
        let dual_ok = (actual.w + expected.w).abs() <= tol && (actual.x + expected.x).abs() <= tol
            && (actual.y + expected.y).abs() <= tol && (actual.z + expected.z).abs() <= tol;
        assert!(
            q_ok || dual_ok,
            "FAILED: test_case=\"{}\" representation=\"{}\"\n  actual quaternion:   {:?}\n  expected quaternion: {:?}\n  (q·q' vs -q·q' dual both exceeded tol={})",
            case_name,
            representation,
            actual,
            expected,
            tol
        );
    }

    fn euler_seq_from_key(key: &str) -> Option<EulerSequence> {
        EulerSequence::from_string(key).ok()
    }

    #[test]
    fn scipy_test_cases_quaternion_matrix_axis_angle_rotation_vector() {
        let json = include_str!("../../tests/scipy_test_cases.json");
        let data: ScipyTestCases = serde_json::from_str(json)
            .expect("Failed to parse scipy_test_cases.json");

        for tc in &data.test_cases {
            let q_ref = Quaternion::new(
                tc.quaternion[0] as f32,
                tc.quaternion[1] as f32,
                tc.quaternion[2] as f32,
                tc.quaternion[3] as f32,
            );

            // From quaternion
            let r_quat = Rotation::from(q_ref);
            assert_quat_near(
                &r_quat.as_quaternion(),
                &q_ref,
                TOL,
                &tc.name,
                "quaternion",
            );

            // From rotation matrix
            let mat = RotationMatrix([
                [
                    tc.rotation_matrix[0][0] as f32,
                    tc.rotation_matrix[0][1] as f32,
                    tc.rotation_matrix[0][2] as f32,
                ],
                [
                    tc.rotation_matrix[1][0] as f32,
                    tc.rotation_matrix[1][1] as f32,
                    tc.rotation_matrix[1][2] as f32,
                ],
                [
                    tc.rotation_matrix[2][0] as f32,
                    tc.rotation_matrix[2][1] as f32,
                    tc.rotation_matrix[2][2] as f32,
                ],
            ]);
            let r_mat = Rotation::from(mat);
            assert_quat_near(
                &r_mat.as_quaternion(),
                &q_ref,
                TOL,
                &tc.name,
                "rotation_matrix",
            );

            // From axis-angle [x, y, z, angle]
            let aa = AxisAngle::new(
                tc.axis_angle[0] as f32,
                tc.axis_angle[1] as f32,
                tc.axis_angle[2] as f32,
                tc.axis_angle[3] as f32,
            );
            let r_aa = Rotation::from(aa);
            assert_quat_near(
                &r_aa.as_quaternion(),
                &q_ref,
                TOL,
                &tc.name,
                "axis_angle",
            );

            // From rotation vector
            let rv = RotationVector::new(
                tc.rotation_vector[0] as f32,
                tc.rotation_vector[1] as f32,
                tc.rotation_vector[2] as f32,
            );
            let r_rv = Rotation::from(rv);
            assert_quat_near(
                &r_rv.as_quaternion(),
                &q_ref,
                TOL,
                &tc.name,
                "rotation_vector",
            );
        }
    }

    #[test]
    fn scipy_test_cases_euler_angles() {
        let json = include_str!("../../tests/scipy_test_cases.json");
        let data: ScipyTestCases = serde_json::from_str(json)
            .expect("Failed to parse scipy_test_cases.json");

        for tc in &data.test_cases {
            let q_ref = Quaternion::new(
                tc.quaternion[0] as f32,
                tc.quaternion[1] as f32,
                tc.quaternion[2] as f32,
                tc.quaternion[3] as f32,
            );

            for (seq_key, angles) in &tc.euler_angles {
                if let Some(seq) = euler_seq_from_key(seq_key) {
                    let euler = EulerAngles::new(
                        angles[0] as f32,
                        angles[1] as f32,
                        angles[2] as f32,
                        seq,
                    );
                    let r_euler = Rotation::from(euler);
                    let representation = format!("euler_angles[{}]", seq_key);
                    assert_quat_near(
                        &r_euler.as_quaternion(),
                        &q_ref,
                        TOL,
                        &tc.name,
                        &representation,
                    );
                }
            }
        }
    }
}
