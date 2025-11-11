use std::ops::{
    Add,
    Mul,
    Sub,
};

pub trait MoreMath {
    fn mix<T: Copy + Sub<Output = T> + Add<Output = T> + Mul<f64, Output = T>>(self, a: T, b: T) -> T;
}

impl MoreMath for f64 {
    fn mix<T: Copy + Sub<Output = T> + Add<Output = T> + Mul<f64, Output = T>>(self, a: T, b: T) -> T {
        return (b - a) * self.clamp(0., 1.) + a;
    }
}
