use super::GpuMemoryResource;

#[derive(Debug, Clone)]
pub struct BumpMap16 {
    width: usize,
    height: usize,
    data: Vec<u16>,
    is_updated: bool
}

impl BumpMap16 {
    pub fn new(width: usize, height: usize) -> Self {
        BumpMap16 {
            width: width,
            height: height,
            data: vec![0; width * height],
            is_updated: true
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn data(&self) -> &[u16] {
        self.data.as_slice()
    }

    pub fn data_mut(&mut self) -> &mut [u16] {
        self.is_updated = true;
        self.data.as_mut_slice()
    }
}

impl GpuMemoryResource for BumpMap16 {
    fn mark_updated(&mut self) {
        self.is_updated = true;
    }

    fn is_updated(&self) -> bool {
        self.is_updated
    }
}