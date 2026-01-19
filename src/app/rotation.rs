use std::ops::Mul;

#[derive(Debug, Clone, Copy)]
pub struct Quaternion {w: f32, x: f32, y: f32, z: f32}

impl Quaternion {
    pub fn new(w: f32, x: f32, y: f32, z: f32) -> Self {
        let norm = (w * w + x * x + y * y + z * z).sqrt();
        if norm == 0.0 { 
            panic!("Quaternion is zero, did you mean to create [0.0,0.0,0.0,1.0]?");
        }
        Self { w: w / norm, x: x / norm, y: y / norm, z: z / norm }
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

#[derive(Debug, Clone, Copy)]
pub struct AxisAngle {
    x: f32,
    y: f32,
    z: f32,
    angle: f32,
}

impl AxisAngle {
    pub fn new(x: f32, y: f32, z: f32, angle: f32) -> Self {
        Self { x, y, z, angle }
    }

    pub fn as_quaternion<T: From<Quaternion>>(&self) -> T {
        let half = self.angle / 2.0;
        let s = half.sin(); 
        Quaternion::new(
            half.cos(),
            self.x * s,
            self.y * s,
            self.z * s,
        ).into()
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

pub struct Rotation {
    quat: Quaternion,
}

impl From<Quaternion> for Rotation {
    fn from(quat: Quaternion) -> Self {
        Rotation { quat }
    }
}

impl From<AxisAngle> for Rotation {
    fn from(axis_angle: AxisAngle) -> Self {
        Rotation { quat: axis_angle.as_quaternion() }
    }
}

impl Rotation {
    pub fn as_quaternion(&self) -> Quaternion {
        self.quat
    }

    pub fn as_axis_angle(&self) -> AxisAngle {
        AxisAngle::from_quaternion(self.quat)
    }
}

impl Mul for Rotation {
    type Output = Rotation;

    fn mul(self, other: Rotation) -> Rotation {
        Rotation { quat: self.quat * other.quat }
    }
}