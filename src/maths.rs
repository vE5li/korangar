pub use cgmath::{ Matrix4, Matrix3, Vector4, Vector3, Vector2, Rad, Deg, SquareMatrix };
pub use std::cmp::{ min, max };

macro_rules! vector3 {
    ($value:expr) => {
        Vector3::new($value, $value, $value)
    };
    ($vector2:expr, $z:expr) => {
        Vector3::new($vector2.x, $vector2.y, $z)
    };
    ($x:expr, $y:expr, $z:expr) => {
        Vector3::new($x, $y, $z)
    }
}

macro_rules! vector4 {
    ($value:expr) => {
        Vector4::new($value, $value, $value, $value)
    };
    ($vector3:expr, $w:expr) => {
        Vector4::new($vector3.x, $vector3.y, $vector3.z, $w)
    };
    ($x:expr, $y:expr, $z:expr, $w:expr) => {
        Vector4::new($x, $y, $z, $w)
    }
}
