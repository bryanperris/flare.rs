/// For now this module is based on the "DirectDraw" video interface

use crate::{game_client};

trait video_client {
    fn init(game_client: &dyn game_client::game_client, driver: &'static str);
    fn close();
    fn set_video_mode(&mut self, width: usize, height: usize, color_depth: i32, is_paged: bool);
    fn set_screen_handle(&mut self, handle: u32);
    fn get_video_properties(&self) -> (usize, usize, i32); // returns width, height, color_depth
    fn get_aspect_ratio(&self) -> f32;
    /*
    void ddvid_LockFrameBuffer(ubyte **data, int *pitch);
     void ddvid_UnlockFrameBuffer();
     */
    fn swap_buffers(&mut self);
}