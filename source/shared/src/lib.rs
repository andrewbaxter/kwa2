use {
    std::mem::swap,
};

pub mod interface;

pub fn steal<T: Default>(x: &mut T) -> T {
    let mut x1 = T::default();
    swap(x, &mut x1);
    return x1;
}
