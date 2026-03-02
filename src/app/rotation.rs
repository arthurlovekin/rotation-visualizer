use std::ops::{Mul};
use std::cmp::PartialEq;
use std::ops::{Index, IndexMut};

/// When sin(angle/2) < this, we treat the quaternion as near-identity (angle ≈ 2π)
/// to avoid division by near-zero. Using 4× EPSILON (~4.8e-7) preserves more precision
/// than 1e-6 while remaining numerically stable for f32.
const NEAR_IDENTITY_S_THRESHOLD: f32 = 1.0 * f32::EPSILON;

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
    fn from(matrix: RotationMatrix) -> Self {
        let (m00, m01, m02) = (matrix[0][0], matrix[0][1], matrix[0][2]);
        let (m10, m11, m12) = (matrix[1][0], matrix[1][1], matrix[1][2]);
        let (m20, m21, m22) = (matrix[2][0], matrix[2][1], matrix[2][2]);
        let trace = 1.0 + m00 + m11 + m22;
        let (w, x, y, z) = if trace > 0.0 {
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

/// Davenport rotation sequence: order of axes for Euler angles.
/// Notation: XYZ = extrinsic (fixed frame), xyz = intrinsic (body frame).
/// Equivalent sequences share one variant: extrinsic XYZ ≡ intrinsic zyx.
/// Tait-Bryan: all three axes are different
/// Proper Euler: two axes are the same
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
    // Proper Euler
    XYX_yxy,
    XZX_zxz,
    YXY_xyx,
    YZY_zyz,
    ZXZ_xzx,
    ZYZ_yzy,
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
            // Tait-Bryan: R = R_z(a)*R_y(b)*R_x(c)
            EulerSequence::XYZ_zyx => {
                let sb = (-m[2][0]).clamp(-1.0, 1.0);
                let b = sb.asin();
                let cb = b.cos();
                if cb.abs() > 1e-6 {
                    let a = m[1][0].atan2(m[0][0]);
                    let c = m[2][1].atan2(m[2][2]);
                    (a, b, c)
                } else if m[2][0] < 0.0 {
                    let a = m[0][2].atan2(-m[0][1]);
                    (a, std::f32::consts::FRAC_PI_2, 0.0)
                } else {
                    let a = (-m[0][2]).atan2(m[0][1]);
                    (a, -std::f32::consts::FRAC_PI_2, 0.0)
                }
            }
            // R = R_y(a)*R_z(b)*R_x(c)
            EulerSequence::XZY_yzx => {
                let sb = m[1][0].clamp(-1.0, 1.0);
                let b = sb.asin();
                let cb = b.cos();
                if cb.abs() > 1e-6 {
                    let a = (-m[2][0]).atan2(m[0][0]);
                    let c = (-m[1][2]).atan2(m[1][1]);
                    (a, b, c)
                } else if m[1][0] > 0.0 {
                    let a = (-m[2][0]).atan2(m[0][0]);
                    (a, std::f32::consts::FRAC_PI_2, 0.0)
                } else {
                    let a = m[2][0].atan2(-m[0][0]);
                    (a, -std::f32::consts::FRAC_PI_2, 0.0)
                }
            }
            // R = R_z(a)*R_x(b)*R_y(c)
            EulerSequence::YXZ_zxy => {
                let sb = m[2][1].clamp(-1.0, 1.0);
                let b = sb.asin();
                let cb = b.cos();
                if cb.abs() > 1e-6 {
                    let a = (-m[0][1]).atan2(m[1][1]);
                    let c = (-m[2][0]).atan2(m[2][2]);
                    (a, b, c)
                } else if m[2][1] > 0.0 {
                    let a = m[2][0].atan2(m[2][2]);
                    (a, std::f32::consts::FRAC_PI_2, 0.0)
                } else {
                    let a = (-m[2][0]).atan2(m[2][2]);
                    (a, -std::f32::consts::FRAC_PI_2, 0.0)
                }
            }
            // R = R_x(c)*R_z(b)*R_y(a) — Eberly RxRzRy: θz=asin(-r01), θx=atan2(r21,r11), θy=atan2(r02,r00)
            EulerSequence::YZX_xzy => {
                let sb = (-m[0][1]).clamp(-1.0, 1.0);
                let b = sb.asin();
                let cb = b.cos();
                if cb.abs() > 1e-6 {
                    let a = m[0][2].atan2(m[0][0]);
                    let c = m[2][1].atan2(m[1][1]);
                    (a, b, c)
                } else if m[0][1] < 0.0 {
                    let a = (-m[2][0]).atan2(m[2][2]);
                    (a, std::f32::consts::FRAC_PI_2, 0.0)
                } else {
                    let a = (-m[2][0]).atan2(m[2][2]);
                    (a, -std::f32::consts::FRAC_PI_2, 0.0)
                }
            }
            // R = R_y(a)*R_x(b)*R_z(c) — Eberly RyRxRz: θx=asin(-r12), θy=atan2(r02,r22), θz=atan2(r10,r11)
            EulerSequence::ZXY_yxz => {
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
            // R = R_x(a)*R_y(b)*R_z(c)
            EulerSequence::ZYX_xyz => {
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
            // Proper Euler: R = R_y(a)*R_x(b)*R_y(c)
            EulerSequence::XYX_yxy => {
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
            // R = R_z(a)*R_x(b)*R_z(c)
            EulerSequence::XZX_zxz => {
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
            // R = R_x(a)*R_y(b)*R_x(c) — Eberly Rx0RyRx1: θy=acos(r00), θx0=atan2(r10,-r20), θx1=atan2(r01,r02)
            EulerSequence::YXY_xyx => {
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
            // R = R_z(a)*R_y(b)*R_z(c) — Eberly Rz0RyRz1: θy=acos(r22), θz0=atan2(r12,r02), θz1=atan2(r21,-r20)
            EulerSequence::YZY_zyz => {
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
            // R = R_x(a)*R_z(b)*R_x(c) — Eberly Rx0RzRx1: θz=acos(r00), θx0=atan2(r20,r10), θx1=atan2(r02,-r01)
            EulerSequence::ZXZ_xzx => {
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
            // R = R_y(a)*R_z(b)*R_y(c)
            EulerSequence::ZYZ_yzy => {
                let cb = m[2][2].clamp(-1.0, 1.0);
                let b = cb.acos();
                let sb = b.sin();
                if sb.abs() > 1e-6 {
                    let a = m[1][2].atan2(m[0][2]);
                    let c = m[2][1].atan2(-m[2][0]);
                    (a, b, c)
                } else {
                    let a = 0.0;
                    let c = m[1][0].atan2(m[1][1]);
                    (a, b, c)
                }
            }
        };
        Self::new(a, b, c, sequence)
    }
}

/// Default Euler sequence used by `From<RotationMatrix> for EulerAngles`.
/// ZYX (intrinsic xyz) is common for roll-pitch-yaw / aerospace conventions.
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
        let (ra, rb, rc) = (e.a, e.b, e.c);
        let (r1, r2, r3) = match e.sequence {
            EulerSequence::XYZ_zyx => (rot_z(ra), rot_y(rb), rot_x(rc)),
            EulerSequence::XZY_yzx => (rot_y(ra), rot_z(rb), rot_x(rc)),
            EulerSequence::YXZ_zxy => (rot_z(ra), rot_x(rb), rot_y(rc)),
            EulerSequence::YZX_xzy => (rot_x(ra), rot_z(rb), rot_y(rc)),
            EulerSequence::ZXY_yxz => (rot_y(ra), rot_x(rb), rot_z(rc)),
            EulerSequence::ZYX_xyz => (rot_x(ra), rot_y(rb), rot_z(rc)),
            EulerSequence::XYX_yxy => (rot_y(ra), rot_x(rb), rot_y(rc)),
            EulerSequence::XZX_zxz => (rot_z(ra), rot_x(rb), rot_z(rc)),
            EulerSequence::YXY_xyx => (rot_x(ra), rot_y(rb), rot_x(rc)),
            EulerSequence::YZY_zyz => (rot_z(ra), rot_y(rb), rot_z(rc)),
            EulerSequence::ZXZ_xzx => (rot_x(ra), rot_z(rb), rot_x(rc)),
            EulerSequence::ZYZ_yzy => (rot_y(ra), rot_z(rb), rot_y(rc)),
        };
        r1 * r2 * r3
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
            EulerSequence::XYX_yxy,
            EulerSequence::XZX_zxz,
            EulerSequence::YXY_xyx,
            EulerSequence::YZY_zyz,
            EulerSequence::ZXZ_xzx,
            EulerSequence::ZYZ_yzy,
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
            EulerSequence::ZXZ_xzx,
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
}

#[cfg(test)]
include!("rotation_tests_generated.rs");