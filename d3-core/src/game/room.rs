use core::sync::atomic::{AtomicUsize, Ordering};
use std::cell::RefCell;
use std::collections::LinkedList;
use std::{ops::Range, rc::Rc};
use crate::common::SharedMutRef;
use crate::graphics::UVCoord;
use crate::string::D3String;
use crate::{graphics::lightmap::LightMap16, math::vector::Vector};
use bitflags::bitflags;
use super::context::GameType;

use super::door::{DoorInfo, DoorwayState};
use super::node::Node;
use super::object::Object;
use super::visual_effects::VisualEffect;
use super::{context::BindingStore, door::Doorway};

pub const MAX_ROOMS: usize = 400;

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

pub struct RoomChanges {
    room: Rc<Room>,
    has_fog: bool,
    vec: Range<Vector>,
    depth: Range<f32>,
    start_time: f32,
    total_time: f32,
}

// TODO: room collection structt
// TODO: track index of highest numbered room

bitflags! {
    /// Flags representing various properties of a face.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct FaceFlags: u16 {
        /// Render this face with a lightmap on top.
        const LIGHTMAP           = 0x0001;
        /// This face has vertex alpha blending.
        const VERTEX_ALPHA       = 0x0002;
        /// This face has a lighting corona.
        const CORONA             = 0x0004;
        /// The texture on this face has changed.
        const TEXTURE_CHANGED    = 0x0008;
        /// This face has a trigger.
        const HAS_TRIGGER        = 0x0010;
        /// This face needs to be not rendered during specularity pass.
        const SPEC_INVISIBLE     = 0x0020;
        /// This face only exists as a floating trigger.
        const FLOATING_TRIG      = 0x0040;
        /// This face has been blown up.
        const DESTROYED          = 0x0080;
        /// This face is a volumetric face.
        const VOLUMETRIC         = 0x0100;
        /// ???
        const TRIANGULATED       = 0x0200;
        /// This face is visible this frame (Valid only during render).
        const VISIBLE            = 0x0400;
        /// This face is not part of the room shell.
        const NOT_SHELL          = 0x0800;
        /// This face has been touched by `fvi_QuickDistFaceList`.
        const TOUCHED            = 0x1000;
        /// This face is a goal texture face.
        const GOALFACE           = 0x2000;
        /// This face is not facing us this frame (Valid only during render).
        const NOT_FACING         = 0x4000;
        /// This face has one or more scorch marks.
        const SCORCHED           = 0x8000;
    }
}

bitflags! {
    /// Flags representing various properties of a portal.
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct PortalFlags: u32 {
        /// Render the face(s) in the portal.
        const RENDER_FACES            = 0x0001;
        /// Allow flythrough of rendered faces.
        const RENDERED_FLYTHROUGH     = 0x0002;
        /// Too small for a robot to use for path following (like a small window).
        const TOO_SMALL_FOR_ROBOT     = 0x0004;
        /// This portal has been combined with another for rendering purposes.
        const COMBINED                = 0x0008;
        /// Used for multiplayer - this portal has been changed.
        const CHANGED                 = 0x0010;
        /// This portal is blocked.
        const BLOCK                   = 0x0020;
        /// This portal is blocked and removable.
        const BLOCK_REMOVABLE         = 0x0040;
    }
}


bitflags! {
    /// Flags representing various properties of a room.
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct RoomFlags: u32 {
        /// Room is a refueling center.
        const FUELCEN                = 0x00000001;
        /// A 3D door is here.
        const DOOR                   = 0x00000002;
        /// This is an external room (i.e., a building).
        const EXTERNAL               = 0x00000004;
        /// This room is goal 1.
        const GOAL1                  = 0x00000008;
        /// This room is goal 2.
        const GOAL2                  = 0x00000010;
        /// This room should receive lighting from satellites.
        const TOUCHES_TERRAIN        = 0x00000020;
        /// Faces are sorted with increasing y.
        const SORTED_INC_Y           = 0x00000040;
        /// This room is goal 3.
        const GOAL3                  = 0x00000080;
        /// This room is goal 4.
        const GOAL4                  = 0x00000100;
        /// This room is fogged.
        const FOG                    = 0x00000200;
        /// This room is a special room.
        const SPECIAL1               = 0x00000400;
        const SPECIAL2               = 0x00000800;
        const SPECIAL3               = 0x00001000;
        const SPECIAL4               = 0x00002000;
        const SPECIAL5               = 0x00004000;
        const SPECIAL6               = 0x00008000;
        /// The mirror in this room is visible.
        const MIRROR_VISIBLE         = 0x00010000;
        /// All the faces in this room should be drawn with triangulation on.
        const TRIANGULATE            = 0x00020000;
        /// This room strobes with pulse lighting.
        const STROBE                 = 0x00040000;
        /// This room flickers with pulse lighting.
        const FLICKER                = 0x00080000;
        /// Mine index of this room (supports up to 32 individual mines).
        const MINE                   = 0x01F00000;
        /// Informs the level goal system on player relinking to this room.
        const INFORM_RELINK_TO_LG    = 0x02000000;
        /// The room path point has been set manually (i.e., by the designer).
        const MANUAL_PATH_PNT        = 0x04000000;
        /// This room has a waypoint in it.
        const WAYPOINT               = 0x08000000;
        /// This room is a secret room.
        const SECRET                 = 0x10000000;
        /// This room does not get lit.
        const NO_LIGHT               = 0x20000000;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RoomLightUV {
    pub u: f32,
    pub v: f32,
    pub u2: f32,
    pub v2: f32,
    pub alpha: u8
}

#[derive(Debug, Clone)]
pub struct Face {
    pub flags: FaceFlags,
    pub num_verts: usize,
    pub portal: Option<Rc<Portal>>,
    pub face_verts: Vec<usize>,
    pub face_uvls: Vec<UVCoord>,
    pub normal: Vector,
    pub lightmap: Option<Rc<LightMap16>>,
    pub special_faces: (),
    pub render_frame: (),
    pub tmap: (),
    pub light_muliple: u8,
    pub min_xyz: Vector,
    pub max_xyz: Vector
}

#[derive(Debug, Clone)]
pub struct Portal {
    pub flags: PortalFlags,
    pub portal_face: Option<SharedMutRef<Face>>,
    pub connected_room: Option<SharedMutRef<Room>>,
    pub connected_portal: Option<SharedMutRef<Self>>,
    pub bnode_index: (),
    /// For rendering combined portals
    pub combine_master: (),
    /// Point used by the path system
    pub path_point: Vector,
}

#[derive(Debug, Clone)]
pub struct VecRange {
    pub min: Vector,
    pub max: Vector
}

#[derive(Debug, Clone)]
pub struct BoundingBoxFaceList {
    pub faces: Vec<usize>,
    pub range: VecRange,
    pub sector: u8,
}

#[derive(Debug, Clone)]
pub struct BoundingBoxHierarchy {
    pub range: VecRange,
    pub regions: Vec<BoundingBoxFaceList>,
}

/* notes on some confusing stuff 
 ------------
      short *num_structs_per_room = (short *)mem_malloc((Highest_room_index + 1) * sizeof(short));
      rp->num_bbf_regions = 27 + num_structs_per_room[i] - 1;
 */

#[derive(Debug, GameType)]
pub struct Room {
    id: usize,
    pub flags: RoomFlags,

    /// Polygon count
    pub face_count: usize,
    /// Connection count
    pub portal_count: usize,
    /// Vertex count
    pub vert_count: usize,

    pub faces: Vec<Face>,
    pub portals: Vec<Portal>,
    pub vertices: Vec<Vector>,

    pub assigned_door_data: Option<RoomDoorData>,

    pub name: Option<D3String>,
    // pub objects: LinkedList<SharedMutRef<Object>>
    pub objects: Vec<SharedMutRef<Object>>,
    
    // For external room visibility checking
    pub max_xyz: Vector,
    pub min_xyz: Vector,

    pub last_drawn: f32,

    pub bounding_box: BoundingBoxHierarchy,

    pub nodes: SharedMutRef<Vec<Node>>,
    pub is_outside: bool,

    pub visual_effects: Vec<Box<dyn VisualEffect>>
}

impl Default for Room {
    fn default() -> Self {
        Self { 
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            assigned_door_data: None,
            ..Default::default()
        }
    }
}

impl Room {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn assign_door(&mut self, value: RoomDoorData) {
        self.assigned_door_data = Some(value);
    }

    pub fn destroy_door(&mut self) {
        self.assigned_door_data = None;
    }

    pub fn clear_nodes(&mut self) {
        let mut nodes = self.nodes.borrow_mut();
        nodes.clear();
    }
}

#[derive(Debug, Clone)]
pub struct RoomDoorData {
    object: SharedMutRef<Object>,
    info: SharedMutRef<DoorInfo>,
    doorway: SharedMutRef<Doorway>
}

impl RoomDoorData {
    pub fn new(
        object: SharedMutRef<Object>,
        info: SharedMutRef<DoorInfo>,
        doorway: SharedMutRef<Doorway>
    ) -> Self {
        Self {
            object: object,
            info: info,
            doorway: doorway
        }
    }

    pub fn door_obj(&self) -> &SharedMutRef<Object> {
        &self.object
    }

    pub fn door_info(&self) -> &SharedMutRef<DoorInfo> {
        &self.info
    }

    pub fn doorway(&self) -> &SharedMutRef<Doorway> {
        &self.doorway
    }
}

// TODO: These functions I dislike, but need for now...
pub fn is_room_outside(x: usize) -> bool {
    (x & 0x80000000) != 0
}

/// Returns indeces of the two elements of points on a face to use as a 2d projection
/// Parameters:	normal - the surface normal of the face
///					ii,jj - filled in with elements numbers (0,1, or 2)]
// TODO: Better name for this function
pub fn get_ij(normal: &Vector, ii: &mut usize, jj: &mut usize) {
    // To project onto 2d, find the largest element of the surface normal
    if normal.x.abs() > normal.y.abs() {
        if normal.x.abs() > normal.z.abs() {
            if normal.x > 0.0 {
                *ii = 2;
                *jj = 1; // x > y, x > z
            }
            else {
                *ii = 1;
                *jj = 2;
            }
        }
        else {
            if normal.z > 0.0 {
                *ii = 1;
                *jj = 0; // z > x > y
            }
            else {
                *ii = 0;
                *jj = 1;
            }
        }
    }
    else { // y > x
        if normal.y.abs() > normal.z.abs() {
            if normal.y > 0.0 {
                *ii = 0;
                *jj = 2; // y > x, y > z
            }
            else {
                *ii = 2;
                *jj = 0;
            }
        }
        else {
            if normal.z > 0.0 {
                *ii = 1;
                *jj = 0; // z > y > x
            }
            else {
                *ii = 0;
                *jj = 1;
            }
        }
    }
}