use tinyrand::Rand;

pub fn ps_rand(rng: &mut impl Rand) -> u32 {
    rng.next_u32() & 0x7fff
}