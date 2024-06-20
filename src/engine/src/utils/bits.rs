use std::ops::{BitAnd, BitOr, Not, Shl, Shr};

pub fn set_msb<T>(num: T) -> T
where
    T: BitOr<Output = T> + Shl<usize, Output = T> + From<u8>,
{
    let num_bits = std::mem::size_of::<T>() * 8;
    num | (T::from(1) << (num_bits - 1))
}

pub fn clear_msb<T>(num: T) -> T
where
    T: BitAnd<Output = T> + Not<Output = T> + Shl<usize, Output = T> + From<u8>,
{
    let num_bits = std::mem::size_of::<T>() * 8;
    num & !(T::from(1) << (num_bits - 1))
}

pub fn get_msb<T>(num: T) -> bool
where
    T: BitAnd<Output = T> + Shl<usize, Output = T> + Shr<usize, Output = T> + From<u8> + PartialEq,
{
    let num_bits = std::mem::size_of::<T>() * 8;
    ((num >> (num_bits - 1)) & T::from(1)) == T::from(1)
}