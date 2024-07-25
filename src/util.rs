pub fn to_usize<T>(x: T) -> usize
where
    T: TryInto<usize>,
    <T as std::convert::TryInto<usize>>::Error: std::fmt::Debug,
{
    x.try_into().unwrap()
}

pub fn to_u32<T>(x: T) -> u32
where
    T: TryInto<u32>,
    <T as std::convert::TryInto<u32>>::Error: std::fmt::Debug,
{
    x.try_into().unwrap()
}

pub fn table_index_mask(index_bits: u32) -> u32 {
    assert!(index_bits <= 32);
    ((1u64 << index_bits) - 1).try_into().unwrap()
}

pub fn table_size(index_bits: u32) -> usize {
    1usize << index_bits
}
