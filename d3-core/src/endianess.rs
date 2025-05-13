pub type TargetEndian = byteorder::LittleEndian;

#[cfg(target_endian = "big")]
pub type TargetEndian = byteorder::BigEndian;