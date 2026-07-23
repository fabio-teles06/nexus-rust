use glam::{Quat, Vec3};
use rapier3d::math::{Rotation, Vector};

pub(crate) fn to_rapier_vector(value: Vec3) -> Vector {
    Vector::new(value.x, value.y, value.z)
}

pub(crate) fn from_rapier_vector(value: Vector) -> Vec3 {
    Vec3::new(value.x, value.y, value.z)
}

pub(crate) fn from_rapier_rotation(value: &Rotation) -> Quat {
    Quat::from_xyzw(value.x, value.y, value.z, value.w)
}
