//! Axis-angle flag visualization: pole along the rotation axis, triangular flag indicates the angle.

use three_d::*;

use crate::app::rotation::AxisAngle;

/// Pole length and flag size chosen to fit with the axes (length ~1.5).
const POLE_LENGTH: f32 = 2.0;
const POLE_RADIUS: f32 = 0.02;
const FLAG_SIZE: f32 = 0.5;
const FLAG_BASE_WIDTH: f32 = 0.15;

/// Axis-angle flag: pole along axis, triangular flag indicates the angle.
/// Create with `new`, call `update` each frame, and add `pole()` and `flag()` to the render list.
pub struct AxisAngleFlag {
    pole: Gm<InstancedMesh, PhysicalMaterial>,
    flag: Gm<Mesh, PhysicalMaterial>,
}

impl AxisAngleFlag {
    /// Creates the axis-angle flag visualization (pole + triangular flag).
    pub fn new(context: &Context) -> Self {
        let cylinder = CpuMesh::cylinder(12);
        let pole_material = PhysicalMaterial::new_opaque(
            context,
            &CpuMaterial {
                albedo: Srgba::new_opaque(255, 200, 50),
                roughness: 0.5,
                metallic: 0.2,
                ..Default::default()
            },
        );
        let pole = Gm::new(
            InstancedMesh::new(
                context,
                &Instances {
                    transformations: vec![Mat4::identity()],
                    ..Default::default()
                },
                &cylinder,
            ),
            pole_material,
        );

        let flag_material = PhysicalMaterial::new_opaque(
            context,
            &CpuMaterial {
                albedo: Srgba::new_opaque(255, 180, 50),
                roughness: 0.5,
                metallic: 0.2,
                ..Default::default()
            },
        );
        let flag = Gm::new(
            Mesh::new(context, &triangle_mesh()),
            flag_material,
        );

        Self { pole, flag }
    }

    /// Updates the flag transform from the current axis-angle.
    pub fn update(&mut self, aa: &AxisAngle) {
        self.pole.geometry.set_instances(&Instances {
            transformations: vec![pole_transform(aa)],
            ..Default::default()
        });
        self.flag.geometry.set_transformation(flag_transform(aa));
    }

    /// The pole object to add to the render list.
    pub fn pole(&self) -> &Gm<InstancedMesh, PhysicalMaterial> {
        &self.pole
    }

    /// The flag object to add to the render list.
    pub fn flag(&self) -> &Gm<Mesh, PhysicalMaterial> {
        &self.flag
    }
}

/// CpuMesh for the triangular flag. Base geometry: pole along +X, flag in XY plane with apex at +Y.
fn triangle_mesh() -> CpuMesh {
    let positions = vec![
        vec3(POLE_LENGTH, 0.0, 0.0),                         // base corner (at pole tip)
        vec3(POLE_LENGTH - FLAG_BASE_WIDTH, 0.0, 0.0),      // base corner (back along pole)
        vec3(POLE_LENGTH, FLAG_SIZE, 0.0),                  // apex
    ];
    let normals = vec![
        vec3(0.0, 0.0, 1.0),
        vec3(0.0, 0.0, 1.0),
        vec3(0.0, 0.0, 1.0),
    ];
    let indices = vec![0u16, 2, 1];
    CpuMesh {
        positions: Positions::F32(positions),
        indices: Indices::U16(indices),
        normals: Some(normals),
        tangents: None,
        uvs: None,
        colors: None,
    }
}

fn axis_alignment_transform(axis: (f32, f32, f32)) -> Mat4 {
    let ax = vec3(axis.0, axis.1, axis.2);
    Mat4::from(Quat::from_arc(vec3(1.0, 0.0, 0.0), ax, None))
}

fn flag_transform(aa: &AxisAngle) -> Mat4 {
    let align = axis_alignment_transform((aa.x, aa.y, aa.z));
    let rotate_angle = Mat4::from_angle_x(radians(aa.angle));
    align * rotate_angle
}

fn pole_transform(aa: &AxisAngle) -> Mat4 {
    let align = axis_alignment_transform((aa.x, aa.y, aa.z));
    let scale = Mat4::from_nonuniform_scale(POLE_LENGTH, POLE_RADIUS, POLE_RADIUS);
    align * scale
}
