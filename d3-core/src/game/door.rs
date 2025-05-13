use super::{context::BindingStore, prelude::*, room::Room};

// IMPORTANT!!!!!!!!!!!
// "Doors" refers to a predefined door that is in memory
// "Doorways" are specific doors that are in the mine
// So, there can be several Doorways that all point to the same Door
// Get it?  If not, talk to Samir or Jason

// Door Object -> Room -> Doorways

#[derive(Debug, Clone)]
pub struct DoorInfo {
    pub name: D3String,
    pub is_seethrough: bool,
    /// Blastable
    pub hit_points: Option<i16>,
    pub total_open_time: f32,
    pub total_close_time: f32,
    pub total_time_open: f32,
    pub drawable_model: (),
    pub open_sound: (),
    pub close_sound: (),
    pub script_name: Option<D3String>,
}

impl DoorInfo {
    pub fn load_polymodel(&mut self, filename: D3String) {
        todo!();
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DoorwayState {
    /// Door is not moving
    Stopped,
    /// Door is opening
    Opening,
    /// Door is closing
    Closing,
    /// Door is waiting to be closed
    Waiting,
    /// Door is opening and will automatically close
    OpeningAuto,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    /// Flags representing various properties of a doorway.
    pub struct DoorwayFlags: u32 {
        const NONE               = 0x00000000;
        /// It's been blasted away.
        const BLASTED            = 0x00000001;
        /// Doorway closes after time.
        const AUTO               = 0x00000002;
        /// Doorway can't open for now.
        const LOCKED             = 0x00000004;
        /// Only one key is needed to open (not all keys).
        const KEY_ONLY_ONE       = 0x00000008;
        /// The Guide-bot ignores the locked state of this door.
        const GB_IGNORE_LOCKED   = 0x00000010;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    /// Flags representing various keys in the key mask set in the door/object.
    pub struct KeyFlags: u32 {
        const NONE = 0x00000000;
        /// Key 1.
        const KEY1 = 0x00000001;
        /// Key 2.
        const KEY2 = 0x00000002;
        /// Key 3.
        const KEY3 = 0x00000004;
        /// Key 4.
        const KEY4 = 0x00000008;
    }
}

#[derive(Debug, Clone, GameType)]
pub struct Doorway {
    pub assigned_room: Option<SharedMutRef<Room>>,
    pub assigned_door: Option<SharedMutRef<DoorInfo>>,
    pub state: DoorwayState,
    pub flags: DoorwayFlags,
    /// Used by trigger system.  These bits need to be set to activate the door.
    pub keys_needed: KeyFlags,
    pub is_active: bool,
    pub position: f32,
    pub dest_pos: f32,
    /// TODO: handle of last sound played...
    pub sound_handle: Option<()>
}

impl Default for Doorway {
    fn default() -> Self {
        Self {
            assigned_room: None,
            assigned_door: None,
            state: DoorwayState::Stopped,
            flags: DoorwayFlags::AUTO,
            keys_needed: KeyFlags::NONE,
            position: 0.0,
            dest_pos: 0.0,
            sound_handle: None,
            is_active: false,
        }
    }
}

impl Doorway {
    pub fn is_locked(&self) -> bool {
        self.flags.contains(DoorwayFlags::LOCKED)
    }

    pub fn state(&self) -> DoorwayState {
        self.state
    }

    pub fn set_lock_state(&mut self, value: bool) {
        if value {
            self.flags.insert(DoorwayFlags::LOCKED);
        } else {
            self.flags.remove(DoorwayFlags::LOCKED);
        }
    }

    pub fn position(&self) -> f32 {
        self.position
    }

    pub fn is_blasted(&self) -> bool {
        self.flags.contains(DoorwayFlags::BLASTED)
    }

    fn set_position(&mut self, position: f32) {
        self.dest_pos = position;

        if self.position == position {
            return;
        }

        if self.dest_pos > self.position {
            self.state = DoorwayState::Opening;
        } else {
            self.state = DoorwayState::Closing;
        }

        self.is_active = true;
    }

    fn stop(&mut self) {
        self.dest_pos = self.position;
        self.state = DoorwayState::Stopped;
    }

    fn activate(&mut self) {
        if self.is_blasted() {
            return;
        }

        match self.state {
            DoorwayState::Opening => return,
            DoorwayState::Waiting => return,
            DoorwayState::OpeningAuto => return,
            _ => {}
        }

        self.dest_pos = 1.0;

        self.state = if self.flags.contains(DoorwayFlags::AUTO) {
            DoorwayState::OpeningAuto
        } else {
            DoorwayState::Opening
        };

        self.is_active = true;
    }
}

impl GameBoundedType<Doorway>  {
    pub fn activate_with_sound_fx_and_trigger_event(&self, door_object: SharedMutRef<Object>) {
        let mut doorway = self.inner().borrow_mut();
        let mut context_ref = self.context();
        let mut context = context_ref.borrow_mut();

        if doorway.is_blasted() {
            return;
        }

        self.play_sound(&door_object.borrow());

        context.script_runtime.signal_event(
            super::scripting::EventType::DoorActivate, None, door_object
        );
    }

    pub fn play_sound(&self, object: &Object) {
        todo!()
    }

    pub fn set_position_with_sound_fx(&self, position: f32) {
        let mut doorway = self.inner().borrow_mut();

        doorway.set_position(position);

        self.play_sound(todo!());
    }

    pub fn stop_with_sound_fx(&self) {
        let mut doorway = self.inner().borrow_mut();
        let mut context_ref = self.context();
        let mut context = context_ref.borrow_mut();

        context.audio_system.stop_sound_immediate(doorway.sound_handle.unwrap());

        doorway.is_active = false;
    }
}