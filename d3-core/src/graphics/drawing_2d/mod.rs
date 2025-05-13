pub mod text;
pub mod font;


use bitfield::bitfield;

// TODO: hold off on the 2dlib stuff (editor related)
// Could just replace it with Skia


// Define the bitfield part
bitfield! {
    struct MemBitmapFields(u16);
    u16;
    alloced, set_alloced: 1, 0;  // 2-bit field (bits 0 and 1)
    flag, set_flag: 15, 2;       // 14-bit field (bits 2 to 15)
}

// Define the main struct
struct MemBitmap {
    data: *mut u8,
    bpp: i16,
    rowsize: i32,
    fields: MemBitmapFields,
}

impl MemBitmap {
    fn new() -> Self {
        MemBitmap {
            data: std::ptr::null_mut(),
            bpp: 0,
            rowsize: 0,
            fields: MemBitmapFields(0),
        }
    }

    fn set_alloced(&mut self, value: u16) {
        self.fields.set_alloced(value);
    }

    fn alloced(&self) -> u16 {
        self.fields.alloced()
    }

    fn set_flag(&mut self, value: u16) {
        self.fields.set_flag(value);
    }

    fn flag(&self) -> u16 {
        self.fields.flag()
    }
}