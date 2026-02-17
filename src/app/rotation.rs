use std::ops::{Mul};
use std::cmp::PartialEq;
use std::ops::{Index, IndexMut};

/// When sin(angle/2) < this, we treat the quaternion as near-identity (angle ≈ 2π)
/// to avoid division by near-zero. Using 4× EPSILON (~4.8e-7) preserves more precision
/// than 1e-6 while remaining numerically stable for f32.
const NEAR_IDENTITY_S_THRESHOLD: f32 = 4.0 * f32::EPSILON;

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

        if new_angle == 0.0 {
            return Ok(Self { x: 1.0, y: 0.0, z: 0.0, angle: 0.0 });
        }
        let axis_norm_sq = ax * ax + ay * ay + az * az;
        if axis_norm_sq == 0.0 {
            return Err("Axis norm cannot be zero unless angle is zero".to_string());
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
        let angle = 2.0 * quat.w.acos();
        if angle == 0.0 {
            return Self::new(0.0, 0.0, 0.0, 0.0);
        }
        let s = (1.0 - quat.w * quat.w).sqrt(); // = sin(angle/2)
        if s < NEAR_IDENTITY_S_THRESHOLD {
            return Self::new(0.0, 0.0, 0.0, 0.0);
        }
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

impl Mul for Rotation {
    type Output = Rotation;

    fn mul(self, other: Rotation) -> Rotation {
        Rotation {
            quat: self.quat * other.quat,
        }
    }
}


#[cfg(test)]
include!("rotation_tests_generated.rs");