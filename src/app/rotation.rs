use std::ops::Mul;
use std::ops::{Index, IndexMut};

#[derive(Debug, Clone, Copy, PartialEq)]
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
        Self::try_new(w, x, y, z)
            .expect("Quaternion is zero, did you mean to create [0.0,0.0,0.0,1.0]?")
    }

    /// Try to create a new unit quaternion. Returns Err for zero quaternions.
    pub fn try_new(w: f32, x: f32, y: f32, z: f32) -> Result<Self, String> {
        let norm = (w * w + x * x + y * y + z * z).sqrt();
        if norm == 0.0 {
            return Err("Quaternion is zero".to_string());
        }
        Ok(Self {
            w: w / norm,
            x: x / norm,
            y: y / norm,
            z: z / norm,
        })
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

impl From<RotationMatrix> for Quaternion {
    fn from(matrix: RotationMatrix) -> Self {
        let mut quat = Quaternion::default();
        quat.w = (1.0 + matrix[0][0] + matrix[1][1] + matrix[2][2]) / 2.0;
        quat.x = (matrix[2][1] - matrix[1][2]) / (4.0 * quat.w);
        quat.y = (matrix[0][2] - matrix[2][0]) / (4.0 * quat.w);
        quat.z = (matrix[1][0] - matrix[0][1]) / (4.0 * quat.w);
        quat
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisAngle {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub angle: f32,
}

impl AxisAngle {
    pub fn new(x: f32, y: f32, z: f32, angle: f32) -> Self {
        Self { x, y, z, angle }
    }

    pub fn as_quaternion<T: From<Quaternion>>(&self) -> T {
        let half = self.angle / 2.0;
        let s = half.sin();
        Quaternion::new(half.cos(), self.x * s, self.y * s, self.z * s).into()
    }

    pub fn from_quaternion(quat: Quaternion) -> Self {
        let angle = 2.0 * quat.w.acos();
        let s = (1.0 - quat.w * quat.w).sqrt();
        if s < 1e-6 {
            Self::new(1.0, 0.0, 0.0, 0.0)
        } else {
            Self::new(quat.x / s, quat.y / s, quat.z / s, angle)
        }
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
        AxisAngle::from_quaternion(self.quat)
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
        Rotation {
            quat: axis_angle.as_quaternion(),
        }
    }
}

impl From<RotationMatrix> for Rotation {
    fn from(matrix: RotationMatrix) -> Self {
        Rotation {
            quat: (matrix).into(),
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
