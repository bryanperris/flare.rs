use super::*;
use core::{marker::PhantomData, ops::Mul};

/// A sampler that uses nearest-neighbor sampling.
pub struct Nearest<T, I = f32> {
    pub(crate) texture: T,
    pub(crate) phantom: PhantomData<I>,
}

impl<T, I, const N: usize> Sampler<N> for Nearest<T, I>
where
    T: Texture<N>,
    I: Clone + Mul<Output = I> + Denormalize<T::Index>,
{
    type Index = I;

    type Sample = T::Texel;

    type Texture = T;

    #[inline(always)]
    fn raw_texture(&self) -> &Self::Texture {
        &self.texture
    }

    #[inline(always)]
    fn sample(&self, index: [Self::Index; N]) -> Self::Sample {
        unsafe {
            self.raw_texture()
                .read_unchecked(I::denormalize_array(index, self.raw_texture().size()))
        }
    }

    #[inline(always)]
    unsafe fn sample_unchecked(&self, index: [Self::Index; N]) -> Self::Sample {
        self.raw_texture()
            .read_unchecked(I::denormalize_array(index, self.raw_texture().size()))
    }
}
