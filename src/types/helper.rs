macro_rules! vector2 {
    ($value:expr) => {
        cgmath::Vector2::new($value, $value)
    };
    ($x:expr, $y:expr) => {
        cgmath::Vector2::new($x, $y)
    };
}

macro_rules! vector3 {
    ($value:expr) => {
        cgmath::Vector3::new($value, $value, $value)
    };
    ($vector2:expr, $z:expr) => {
        cgmath::Vector3::new($vector2.x, $vector2.y, $z)
    };
    ($x:expr, $y:expr, $z:expr) => {
        cgmath::Vector3::new($x, $y, $z)
    }
}

macro_rules! vector4 {
    ($value:expr) => {
        cgmath::Vector4::new($value, $value, $value, $value)
    };
    ($vector3:expr, $w:expr) => {
        cgmath::Vector4::new($vector3.x, $vector3.y, $vector3.z, $w)
    };
    ($x:expr, $y:expr, $z:expr, $w:expr) => {
        cgmath::Vector4::new($x, $y, $z, $w)
    }
}
