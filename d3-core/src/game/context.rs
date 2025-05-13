use core::{borrow::{Borrow, BorrowMut}, fmt::{self, Debug}};
use std::{cell::{Ref, RefCell, RefMut}, collections::HashSet, ops::{Deref, DerefMut}, path::{Path, PathBuf}, rc::{Rc, Weak}};
use crate::{common::SharedMutRef, graphics::{ lightmap::LightMap16}};

use super::{audio::AudioSystem, node::Node, object_dynamic_behavior::ScriptedRuntime, scripting::NewOsirusScriptSystem, D3String, GameMode, Object};

// TODO: Support options passed in as args, but not dealing with this now

pub struct GameContext {
    base_directory: PathBuf,
    debug_mode: bool,
    min_allowed_framecap: i32,
    min_allowed_frametime: i32,
    gametime: f32,
    frametime: f32,
    pub mode: GameMode,

    pub player_object_ref: SharedMutRef<Object>,

    pub script_runtime: Box<dyn NewOsirusScriptSystem>,
    pub audio_system: Box<dyn AudioSystem>,

    pub objects: BindingStore<super::object::Object>,
    pub doorways: BindingStore<super::door::Doorway>,


    pub rooms: BindingStore<super::room::Room>,
    
    // Only putting this here for a debug condition
    pub room_highest_index: usize,

    /// Global mask for all keys held by all players
    pub world_keys: super::door::KeyFlags,

    pub terrain: BindingStore<super::terrain::Terrain>,
    pub terrain_nodes: Vec<Vec<Node>>,
    pub weather: BindingStore<super::weather::Weather>,


    /* Resource sections:
     * This is where simple resources are stored that do not need bindings
     */
    pub lightmaps: Vec<SharedMutRef<LightMap16>>,
    pub textures: Vec<SharedMutRef<super::super::graphics::texture::Texture16>>,
    pub texture_set: HashSet<D3String, SharedMutRef<super::super::graphics::texture::Texture16>>
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            // base_directory: asset_directory_path.to_path_buf(),
            base_directory: Default::default(),
            debug_mode: false,
            min_allowed_framecap: ((1.0f32 / 60.0f32) as i32) * 1000,
            min_allowed_frametime: 0,
            gametime: 0.0,
            terrain_nodes: vec![Vec::default(); 8],
            ..Default::default()
        }
    }
}


impl GameContext {
    fn debugging(&mut self, debug_mode: bool) -> &mut Self {
        self.debug_mode = debug_mode;
        self
    }
}

// For the setters and getters
impl GameContext {
    pub fn gametime(&self) -> f32 {
        self.gametime
    }

    pub fn frametime(&self) -> f32 {
        self.frametime
    }
}

pub type GC = SharedMutRef<GameContext>;

pub trait GameType {}

pub type GR<T> = SharedMutRef<GameBoundedType<T>>;

#[derive(Debug, Clone)]
pub struct GameBoundedType<T: GameType> {
    context: Weak<RefCell<GameContext>>,
    inner: SharedMutRef<T>
}

impl <T: GameType> GameBoundedType<T> {
    pub fn context(&self) -> Rc<RefCell<GameContext>>  {
        self.context.upgrade().unwrap()
    }

    pub fn inner(&self) -> &SharedMutRef<T> {
        &self.inner
    }

    pub fn swap_and_drop(&mut self, new_value: SharedMutRef<T>) {
        let mut inner_borrow = self.inner.borrow_mut();
        *inner_borrow = new_value;
    }
}

#[derive(Clone)]
pub struct BindingStore<T : GameType> {
    bindings: Vec<GameBoundedType<T>>
}

impl<T: GameType + Debug> fmt::Debug for BindingStore<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Bindings");

        for (i, binding) in self.bindings.iter().enumerate() {
            let instance = binding.inner.as_ref();
            debug_struct.field(&format!("instance_{}", i), instance);
        }

        debug_struct.finish()
    }
}

impl<T: GameType > Default for BindingStore<T> {
    fn default() -> Self {
        Self { bindings: Vec::new() }
    }
}

impl<T: GameType> BindingStore<T> {

    pub fn push<P>(&mut self, value: T, parent: &Rc<P>) {
        todo!();
    }

    pub fn bindings(&self) -> &Vec<GameBoundedType<T>> {
        &self.bindings
    }

    pub fn only_one(&self) -> &GameBoundedType<T> {
        assert!(self.bindings.len() == 1);

        &self.bindings[0]
    }

    pub fn only_one_mut(&mut self) -> &mut GameBoundedType<T> {
        assert!(self.bindings.len() == 1);

        &mut self.bindings[0]
    }


    pub fn remove_by_index(&mut self, i: usize) {
        self.bindings.remove(i);
    }

    pub fn remove_by_ref(&mut self, the_ref: &SharedMutRef<T>) {
        for (i, binding) in self.bindings().iter().enumerate() {
            if Rc::ptr_eq(&binding.inner, the_ref) {
                self.bindings.remove(i);
                break;
            }
        }
    }
}
