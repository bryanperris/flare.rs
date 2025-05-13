use core::{any::Any, ptr::addr_of};
use std::{
    collections::{HashSet, VecDeque},
    os::unix::process,
    rc, vec,
};

use angle::Angle;
use matrix::Matrix;
use vector::Vector;
use vector2d::Vector2D;

use crate::{
    game::{object_dynamic_behavior::MovementType, room::FaceFlags, terrain::TERRAIN_SIZE},
    graphics::polymodel::PolyModel,
};

use super::{
    super::prelude::*,
    super::room::{self, Face, Room, MAX_ROOMS},
    super::terrain::{Terrain, TERRAIN_DEPTH, TERRAIN_WIDTH},
};

#[derive(Debug, Copy, Clone)]
pub enum HitType {
    /// We hit nothing
    None,
    /// We hit a wall
    Wall,
    /// We hit an object
    Object,
    /// We hit the terrain
    Terrain,
    /// Start point is not in the specified segment
    BadP0,
    /// End point is outside of the terrain bounds
    OutOfTerrainBounds,
    /// We hit the backface of a wall
    Backface,
    /// Hit a sphere to a real polygon
    SphereToPolyObject,
    /// Object hit the ceiling
    Ceiling,
    /// Hit a corner wall
    CornerWall,
    /// Hit an edge wall
    EdgeWall,
    /// Hit a face wall
    FaceWall,
}

bitflags! {
    #[derive(Debug, Copy, Clone)]
    /// Flags for various query options
    pub struct FqFlags: u32 {
        /// Check against objects?
        const CHECK_OBJS = 1;
        /// Hit the backfaces of poly objects
        const OBJ_BACKFACE = 1 << 1;
        /// Go through trans wall if hit point is transparent
        const TRANSPOINT = 1 << 2;
        /// Ignore powerups
        const IGNORE_POWERUPS = 1 << 3;
        /// Check for collisions with backfaces, usually they are ignored
        const BACKFACE = 1 << 4;
        /// Makes connectivity disappear for FVI
        const SOLID_PORTALS = 1 << 5;
        /// Records faces that should be recorded
        const RECORD = 1 << 6;
        /// Records faces that should be recorded (new list)
        const NEW_RECORD_LIST = 1 << 7;
        /// Ignores all objects that move
        const IGNORE_MOVING_OBJECTS = 1 << 8;
        /// Ignores all objects that are not associated with lightmaps
        const IGNORE_NON_LIGHTMAP_OBJECTS = 1 << 9;
        /// Ignores all objects besides the player
        const ONLY_PLAYER_OBJ = 1 << 10;
        /// Ignores all walls (it will still hit OBJ_ROOMS)
        const IGNORE_WALLS = 1 << 11;
        /// Checks if object hits the imaginary ceiling
        const CHECK_CEILING = 1 << 12;
        /// Ignores all objects except doors
        const ONLY_DOOR_OBJ = 1 << 13;
        /// Does not determine the hit segment
        const NO_RELINK = 1 << 14;
        /// Treats external rooms as spheres
        const EXTERNAL_ROOMS_AS_SPHERE = 1 << 15;
        /// Enable multi-point collision
        const MULTI_POINT = 1 << 16;
        /// Lighting-only optimizations
        const LIGHTING = 1 << 17;
        /// Computes movement time
        const COMPUTE_MOVEMENT_TIME = 1 << 18;
        /// Ignores external rooms
        const IGNORE_EXTERNAL_ROOMS = 1 << 19;
        /// Ignores weapons
        const IGNORE_WEAPONS = 1 << 20;
        /// Ignores terrain
        const IGNORE_TERRAIN = 1 << 21;
        /// Treats players as spheres
        const PLAYERS_AS_SPHERE = 1 << 22;
        /// Treats robots as spheres
        const ROBOTS_AS_SPHERE = 1 << 23;
        /// Ignores clutter collisions
        const IGNORE_CLUTTER_COLLISIONS = 1 << 24;
        /// Ignores rendering through portals
        const IGNORE_RENDER_THROUGH_PORTALS = 1 << 25;
    }
}

const PLAYER_SIZE_SCALAR: f32 = 0.8;
const CELLS_PER_COL_CELL: usize = 1;
const COL_TERRAIN_SIZE: f32 = super::super::terrain::TERRAIN_SIZE * CELLS_PER_COL_CELL as f32;
const MIN_BIG_OBJ_RAD: f32 = COL_TERRAIN_SIZE;
const MAX_CELLS_VISITED: usize = TERRAIN_DEPTH * TERRAIN_WIDTH;

// XXX:
// chrishack -- we could turn backface, on and off so that we
// can individually use backface checking on object and/or wall...  :)

const MAX_SEGS: usize = 100;
const MAX_HITS: usize = 2;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum QuadType {
    Right,
    Left,
    Middle,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IntersectionType {
    None,
    Face,
    Edge,
    Vertex,
}

#[derive(Debug, Clone, Copy)]
struct FaceRoomRecord {
    pub face_index: usize,
    pub room_index: usize,
}

#[derive(Debug, Clone)]
pub struct IntersectionFinder {
    pub ceiling_height: f32,
    pub always_check_ceiling: bool,

    /// Bit field for fast checking if a terrain segment has been visited.
    terrain_visit_list: Vec<u8>,

    /// Bit field for fast checking if a terrain object has been visited.
    terrain_obj_visit_list: Vec<u8>,

    /// Whether to perform a terrain check. If true, only one full terrain check is performed.
    check_terrain: bool,

    /// Whether the FVI call has zero radius for collision checks.
    zero_rad: bool,

    /// Unordered list of terrain cells visited during this FVI call.
    cells_visited: Vec<u16>,

    /// Unordered list of terrain cells with objects visited during this FVI call.
    cells_obj_visited: Vec<u16>,

    /// Radius of the wall sphere used in wall collision detection.
    wall_sphere_rad: f32,

    /// Offset of the wall sphere used in wall collision detection.
    wall_sphere_offset: Vector,

    /// Starting position of the wall sphere.
    wall_sphere_p0: Vector,

    /// Ending position of the wall sphere.
    wall_sphere_p1: Vector,

    /// Radius of the animation sphere used in animation collision detection.
    anim_sphere_rad: f32,

    /// Offset of the animation sphere used in animation collision detection.
    anim_sphere_offset: Vector,

    /// Starting position of the animation sphere.
    anim_sphere_p0: Vector,

    /// Ending position of the animation sphere.
    anim_sphere_p1: Vector,

    /// Pointer to hit data for the FVI call. This contains detailed information about the collision.
    hit_data: Option<IntersectionFinderResult>,

    /// Pointer to query data for the FVI call. This contains the original query parameters.
    query: Option<Query>,

    /// The best distance of the collision found during this FVI call.
    collision_dist: f32,

    /// Maximum bounds for movement in Axis-Aligned Bounding Box (AABB) format.
    max_xyz: Vector,

    /// Minimum bounds for movement in Axis-Aligned Bounding Box (AABB) format.
    min_xyz: Vector,

    /// Movement delta for this FVI call, representing how much movement occurred.
    movement_delta: Vector,

    /// Maximum bounds for wall movement in Axis-Aligned Bounding Box (AABB) format.
    wall_max_xyz: Vector,

    /// Minimum bounds for wall movement in Axis-Aligned Bounding Box (AABB) format.
    wall_min_xyz: Vector,

    /// The current object being processed by the FVI call.
    curobj: i32,

    /// The object being moved during the FVI call.
    moveobj: i32,

    /// List of recorded faces (room and terrain cells) visited during the FVI call.
    recorded_faces: Vec<()>,
}

impl Default for IntersectionFinder {
    fn default() -> Self {
        Self {
            ceiling_height: super::super::terrain::MAX_TERRAIN_HEIGHT,
            always_check_ceiling: false,
            terrain_visit_list: vec![0u8; (TERRAIN_DEPTH * TERRAIN_WIDTH) / 8 + 1],
            terrain_obj_visit_list: vec![0u8; (TERRAIN_DEPTH * TERRAIN_WIDTH) / 8 + 1],
            // rooms_visited: vec![0usize; MAX_ROOMS],
            cells_visited: vec![0u16; MAX_CELLS_VISITED],
            cells_obj_visited: vec![0u16; MAX_CELLS_VISITED],
            recorded_faces: vec![(); 200],
            ..Default::default()
        }
    }
}

impl IntersectionFinder {
    pub fn compute_movement_AABB(&mut self, query: &Query) {
        let delta_movement = self.hit_data.as_ref().unwrap().hit_point - query.p0;

        self.min_xyz = query.p0.clone();
        self.max_xyz = query.p0.clone();

        if delta_movement.x > 0.0 {
            self.max_xyz.x += delta_movement.x;
        } else {
            self.max_xyz.x += delta_movement.x;
        }

        if delta_movement.y > 0.0 {
            self.max_xyz.y += delta_movement.y;
        } else {
            self.max_xyz.y += delta_movement.y;
        }

        if delta_movement.z > 0.0 {
            self.max_xyz.z += delta_movement.z;
        } else {
            self.max_xyz.z += delta_movement.z;
        }

        self.wall_min_xyz = self.min_xyz.clone();
        self.wall_max_xyz = self.max_xyz.clone();

        if !self.zero_rad {
            if query.this_obj.is_none() {
                let offset_vec = Vector {
                    x: query.rad,
                    y: query.rad,
                    z: query.rad,
                };

                self.min_xyz -= offset_vec;
                self.max_xyz += offset_vec;

                self.wall_min_xyz = self.min_xyz.clone();
                self.wall_max_xyz = self.max_xyz.clone();
            } else {
                let object_ref = query.this_obj.as_ref().unwrap();
                let object = object_ref.borrow();

                let max_offset = object.max_xzy - object.position;
                let min_offset = object.min_xzy - object.position;

                self.max_xyz += max_offset;
                self.min_xyz += min_offset;

                self.wall_min_xyz = self.min_xyz.clone();
                self.wall_max_xyz = self.max_xyz.clone();
            }
        }
    }

    pub fn object_movement_AABB(&self, obj: &Object) -> bool {
        if obj.max_xzy.x < self.min_xyz.x
            || self.max_xyz.x < obj.min_xzy.x
            || obj.max_xzy.z < self.min_xyz.z
            || self.max_xyz.z < obj.min_xzy.z
            || obj.max_xzy.y < self.min_xyz.y
            || self.max_xyz.y < obj.min_xzy.y
        {
            return false;
        }

        true
    }

    pub fn room_movement_AABB(&self, face: &Face) -> bool {
        if self.wall_max_xyz.y < face.min_xyz.y
            || face.max_xyz.y < self.wall_min_xyz.y
            || self.wall_max_xyz.x < face.min_xyz.x
            || face.max_xyz.x < self.wall_min_xyz.x
            || self.wall_max_xyz.z < face.min_xyz.z
            || face.max_xyz.z < self.wall_min_xyz.z
        {
            return false;
        }

        true
    }

    /// Returns the number of faces that are approximately within the specified radius
    pub fn quick_dist_facelist(
        &mut self,
        initial_room: &mut SharedMutRef<Room>,
        room_list: &[SharedMutRef<Room>],
        position: &Vector,
        rad: f32,
        quick_face_list: Option<&mut [FaceRoomRecord]>,
    ) -> usize {
        debug_assert!(rad >= 0.0);

        // Quick volume
        let min_xyz = Vector {
            x: position.x - rad,
            y: position.y - rad,
            z: position.z - rad,
        };

        let max_xyz = Vector {
            x: position.x + rad,
            y: position.y + rad,
            z: position.z + rad,
        };

        let mut qfl = match quick_face_list {
            Some(x) => Some(x),
            _ => None,
        };

        // Initially this is the only room in the list
        let mut next_rooms: VecDeque<SharedMutRef<Room>> = VecDeque::with_capacity(20);
        next_rooms.push_back(initial_room.clone());

        let mut head = 0usize;

        let mut rooms_visited: HashSet<usize> = HashSet::new();

        rooms_visited.insert(initial_room.borrow().id());

        let mut num_faces = 0;

        // TODO: verify this function still works
        ///      remove the max_elements silly param
        while head < next_rooms.len() {
            let current_room_ref = room_list.get(head).unwrap();
            let mut current_room = current_room_ref.borrow_mut();

            // Sort
            let mut m_sector = 0u8;
            let bb_range = current_room.bounding_box.range.clone();

            if min_xyz.x <= bb_range.min.x {
                m_sector |= 0x01;
            }

            if min_xyz.y <= bb_range.min.y {
                m_sector |= 0x02;
            }

            if min_xyz.z <= bb_range.min.x {
                m_sector |= 0x04;
            }

            if max_xyz.x >= bb_range.max.x {
                m_sector |= 0x08;
            }

            if max_xyz.y >= bb_range.max.y {
                m_sector |= 0x10;
            }

            if max_xyz.z >= bb_range.max.z {
                m_sector |= 0x20;
            }

            let mut faces_touched: Vec<usize> = Vec::new();

            // Do the actual wall collsion stuff here!
            for (i, bbf_list) in current_room.bounding_box.regions.iter().enumerate() {
                let region_range = &bbf_list.range;

                if (bbf_list.sector & m_sector) == bbf_list.sector {
                    if region_range.min.x > max_xyz.x
                        || region_range.min.y > max_xyz.y
                        || region_range.min.z > max_xyz.z
                        || region_range.max.x < min_xyz.x
                        || region_range.max.y < min_xyz.y
                        || region_range.max.z < min_xyz.z
                    {
                        continue;
                    }

                    for face_index in &bbf_list.faces {
                        let face = &current_room.faces[*face_index];

                        if !room_manual_AABB(&face, &min_xyz, &max_xyz) {
                            continue;
                        }

                        if qfl.is_some() {
                            let mut list = qfl.unwrap();

                            if num_faces < list.len() {
                                list[num_faces] = FaceRoomRecord {
                                    face_index: i,
                                    room_index: current_room.id(),
                                };

                                qfl = Some(list);
                            } else {
                                qfl = None;
                                break;
                            }
                        } else {
                            qfl = None;
                            num_faces += 1;
                        }

                        faces_touched.push(i);

                        let portal = current_room.faces[i].portal.as_ref();

                        if portal.is_some() {
                            let portal = portal.unwrap();
                            let connected_room = portal.connected_room.as_ref();

                            if connected_room.is_some() && next_rooms.len() < next_rooms.capacity()
                            {
                                let connected_room_ref = connected_room.unwrap();
                                let connected_room = connected_room_ref.borrow();

                                if !rooms_visited.contains(&connected_room.id()) {
                                    rooms_visited.insert(connected_room.id());

                                    next_rooms.push_back(connected_room_ref.clone());
                                }
                            }
                        }
                    }
                }
            }

            for touched in faces_touched {
                current_room.faces[touched].flags.insert(FaceFlags::TOUCHED);
            }

            head += 1;
        }

        num_faces
    }

    /// Returns the number of faces that are approximately within the specified radius
    pub fn quick_dist_cell_list(
        initial_cell: usize,
        position: &Vector,
        rad: f32,
        quick_cell_list: &mut [usize],
        terrain: &Terrain,
    ) -> usize {
        debug_assert!(rad >= 0.0);

        let mut num_cells = 0;

        process_cells(
            initial_cell,
            position,
            rad,
            terrain,
            |current_node: usize| -> bool {
                // Conditional check
                if terrain.segments[current_node].y >= position.y - rad
                    || terrain.segments[current_node + TERRAIN_WIDTH + 1].y >= position.y - rad
                    || terrain.segments[current_node + 1].y >= position.y - rad
                    || terrain.segments[current_node + TERRAIN_WIDTH].y >= position.y - rad
                {
                    // Add to quick_cell_list
                    quick_cell_list[num_cells] = current_node;
                    num_cells += 1;

                    // Check if we've reached the limit
                    if num_cells >= quick_cell_list.len() {
                        return false; // Exit the loop
                    }
                }

                true // Continue looping
            },
        );

        num_cells
    }

    pub fn quick_dist_object_list(
        &mut self,
        position: &Vector,
        initial_room_ref: (&SharedMutRef<Room>, usize),
        rad: f32,
        object_list: &[usize],
        lightmap_only: bool,
        only_players_and_ais: bool,
        include_non_collide_objects: bool,
        stop_at_closed_doors: bool,
        terrain: &Terrain
    ) {
        //Quick volume
        let delta = Vector {
            x: rad,
            y: rad,
            z: rad,
        };

        self.min_xyz = position.clone() - delta;
        self.max_xyz = position.clone() + delta;
        self.wall_min_xyz = self.min_xyz.clone();
        self.wall_max_xyz = self.max_xyz.clone();

        let initial_room = initial_room_ref.0.borrow();


        let mut num_objects = 0;

        if initial_room.is_outside {
            process_cells(
                initial_room_ref.1,
                position,
                rad,
                terrain,
                |current_node: usize| -> bool {
                    let mut current_object_optional_ref = terrain.segments[current_node].object_ref.clone();

                    while current_object_optional_ref.is_some() {
                        let current_object_ref = current_object_optional_ref.unwrap();
                        let current_object = current_object_ref.borrow();

                        if num_objects >= object_list.len() {
                            return false; // Stop if we've reached the max number of objects
                        }
        
                        todo!();

                        if include_non_collide_objects {
                            
                        }
        
                        // if f_include_non_collide_objects || object.collision_result != RESULT_NOTHING {
                        //     if !f_only_players_and_ais || object.object_type == OBJ_PLAYER || object.ai_info.is_some() {
                        //         if !(f_lightmap_only && object.lighting_render_type != LRT_LIGHTMAPS && object.object_type != OBJ_ROOM) {
                        //             if object_movement_AABB(object) && (object.flags & OF_BIG_OBJECT) == 0 {
                        //                 // Add object to the list
                        //                 object_index_list[*num_objects] = cur_obj_index;
                        //                 *num_objects += 1;
        
                        //                 // Ensure we haven't exceeded the limit
                        //                 assert!(*num_objects <= max_elements);
                        //             }
                        //         }
                        //     }
                        // }
        
                        current_object_optional_ref = current_object.link_next_obj.clone(); // Move to the next object
                    }
        
                    true // Continue processing cells
                },
            );
        }

        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct IntersectionFinderResult {
    // Results
    /// Centerpoint when we hit
    pub hit_point: Vector,
    /// What room hit_point is in
    pub hit_room: Option<SharedMutRef<Room>>,
    /// Distance of the hit
    pub hit_distance: f32,

    /// Number of recorded hits,
    pub hit_count: usize,

    /// What sort of intersection
    pub hit_type: Vec<HitType>,
    /// Actual collision point (edge of rad)
    pub hit_face_point: Vec<Vector>,

    /// What room the fit face is in
    pub hit_face_room: Vec<Option<SharedMutRef<Room>>>,
    /// If hit wall, which face
    pub hit_face: Vec<usize>,
    /// If hit wall, ptr to its surface normal
    pub hit_wall_normal: Vec<Vector>,

    /// If object hit, which one
    pub hit_object: Vec<Option<SharedMutRef<Object>>>,
    /// if a POLY_2_SPHERE hit, then it has a the poly involved
    pub hit_sub_object: Vec<Option<()>>,

    /// How many segs we went through
    pub room_count: usize,
    // List of segs vector went through
    pub room_list: Vec<Option<()>>,
}

impl Default for IntersectionFinderResult {
    fn default() -> Self {
        Self {
            hit_type: vec![HitType::None; MAX_HITS],
            hit_face_point: vec![Vector::ZERO; MAX_HITS],
            hit_face_room: vec![None; MAX_HITS],
            hit_face: vec![0usize; MAX_HITS],
            hit_wall_normal: vec![Vector::ZERO; MAX_HITS],
            hit_object: vec![None; MAX_HITS],
            hit_sub_object: vec![None; MAX_HITS],
            room_list: vec![None; MAX_SEGS],
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Query {
    pub p0: Vector,
    pub p1: Vector,
    pub start_room: SharedMutRef<Room>,
    pub rad: f32,
    pub this_obj: Option<SharedMutRef<Object>>,
    pub ignore_obj_list: (),
    pub flags: FqFlags,

    pub bbox_orientation: Matrix,
    pub bbox_rotvel: Vector,
    pub bbox_rotthrust: Vector,
    pub bbox_velocity: Vector,
    pub bbox_turnroll: Angle,
    pub bbox_thrust: Vector,
    pub frametime: f32,
}

// find the point on the specified plane where the line intersects
// returns true if point found, false if line parallel to plane
// new_pnt is the found point on the plane
// plane_pnt & plane_norm describe the plane
// p0 & p1 are the ends of the line
// Assumes that the initial point is not intersecting the plane
pub fn find_plane_line_intersection(
    intp: &mut Vector,
    colp: &mut Vector,
    plane_point: &Vector,
    plane_normal: &Vector,
    p0: &Vector,
    p1: &Vector,
    rad: f32,
) -> bool {
    assert!(rad >= 0.0);

    // Line direction
    let line_vec = *p1 - *p0;

    // Compute the distance to the plane and the distance the line travels in the direction of the normal
    // Negative because if the object is moving toward the plane, it is moving in the opposite direction of the normal
    let proj_dist_line = plane_normal.dot(line_vec);

    if proj_dist_line >= 0.0 {
        return false;
    }

    //  Vector from p0 to a point on the plane
    let point_plane_vec = *plane_point - *p0;
    let mut proj_dist_point_plane = plane_normal.dot(point_plane_vec);

    // Throw out any sphere who's centerpoint is initially behind the face
    if proj_dist_point_plane > 0.0 {
        return false;
    }

    // Use the distance from the edge of the sphere to the plane.  If the new proj_dist_point_plane is
    // negative, then the sphere pokes thru the edge at the initial position
    proj_dist_point_plane += rad;

    if proj_dist_point_plane > 0.0 && proj_dist_line < 0.0 {
        *intp = p0.clone();
        *colp = *intp + *plane_normal * (-rad + proj_dist_point_plane);

        return true;
    }

    // cannot intersect wall if we are more than a rad away (closest point check)
    if proj_dist_point_plane <= proj_dist_line {
        return false;
    }

    // If we are moving almost parallal to the plane, then make sure we are a rad away form it
    // I picked .00000001 from my head.  It would be pretty parallel of a pretty short movement and
    // the linear combination below might not product a nice answer
    if proj_dist_line.abs() <= 0.00000000001 {
        let plane_dist = (*p1 - *plane_point).dot(*plane_normal);

        if plane_dist >= rad {
            return false;
        }

        *intp = *p1 + (rad - plane_dist) * *plane_normal;

        // Make sure the computed new position is not behind the wall.
        assert!((*intp - *plane_point).dot(*plane_normal) >= -0.01);
    } else {
        // The intersection of the line and the plane is a simple linear combination
        *intp = *p0 + (proj_dist_point_plane / proj_dist_line) * line_vec;
    }

    // Collision point is a rad. closer in the direction of the normal
    *colp = *intp + *plane_normal * -rad;

    true
}

// Find out if a vector intersects with anything.
// Fills in hit_data, an fvi_info structure (see above).
// Parms:
//  p0 & startseg 	describe the start of the vector
//  p1 					the end of the vectorub
//  rad 					the radius of the cylinder
//  thisobjnum 		used to prevent an object with colliding with itself
//  ingore_obj_list	NULL, or ptr to a list of objnums to ignore, terminated with -1
//  check_obj_flag	determines whether collisions with objects are checked
// Returns the hit_data->hit_type
pub fn find_intersection(query: &Query) -> HitType {
    todo!()
}

pub fn fast_vector_bbox(min: &[f32], max: &[f32], origin: &[f32], dir: &[f32]) -> bool {
    let mut quad = [QuadType::Right; 3];
    let mut can_plane = [0f32; 3];
    let mut coord = [0f32; 3];
    let mut max_t = [0f32; 3];
    let mut is_inside = true;

    for i in 0..3 {
        if origin[i] < min[i] {
            quad[i] = QuadType::Left;
            can_plane[i] = min[i];
            is_inside = false;
        } else if origin[i] > max[i] {
            quad[i] = QuadType::Right;
            can_plane[i] = max[i];
            is_inside = false;
        } else {
            quad[i] = QuadType::Middle;
        }
    }

    if is_inside {
        return false;
    }

    for i in 0..3 {
        if quad[i] != QuadType::Middle && dir[i] != 0.0 {
            max_t[i] = (can_plane[i] - origin[i]) / dir[i];
        } else {
            max_t[i] = -1.0;
        }
    }

    let mut which_plane = 0;

    for i in 0..3 {
        if max_t[which_plane] < max_t[i] {
            which_plane = i
        }
    }

    if max_t[which_plane] < 0.0 {
        return false;
    }

    for i in 0..3 {
        if which_plane != i {
            coord[i] = origin[i] + max_t[which_plane] * dir[i];

            if (quad[i] == QuadType::Right && coord[i] < min[i])
                || (quad[i] == QuadType::Left && coord[i] > max[i])
            {
                return false;
            }
        } else {
            coord[i] = can_plane[i];
        }
    }

    true
}

const IJ_TABLE: [[usize; 2]; 3] = [
    [2, 1], // pos x biggest
    [0, 2], // pos y biggest
    [1, 0], // pos z biggest
];

/// See if a point is inside a face by projecting it into 2d
pub fn check_point_to_face(
    colp: &mut Vector,
    face_normal: &mut Vector,
    nv: usize,
    vector_list: &[Vector],
) -> u32 {
    // now do 2d check to see if point is in side

    let biggest: usize;

    // Determine which axis will be normal to the plane the points are projected onto
    if face_normal.x.abs() > face_normal.y.abs() {
        if face_normal.x.abs() > face_normal.z.abs() {
            biggest = 0;
        } else {
            biggest = 2;
        }
    } else if face_normal.y.abs() > face_normal.z.abs() {
        biggest = 1;
    } else {
        biggest = 2;
    }

    let mut normal_arr = face_normal.as_mut_slice();
    let mut colp_arr = colp.as_mut_slice();

    let i: usize;
    let j: usize;

    // For a plane with a normal that is in the opposite direction of the axis,
    // we should circle the other direction -- i.e. always circle in clockwise direction with normal (left-handed)
    if normal_arr[biggest] > 0.0 {
        i = IJ_TABLE[biggest][0];
        j = IJ_TABLE[biggest][1];
    } else {
        i = IJ_TABLE[biggest][1];
        j = IJ_TABLE[biggest][0];
    }

    // Do a simple 2d cross-product between each line segment and the start point to the check point
    // Go in a clockwise direction, if determinant is negative then point is outside of this multi-
    // side polygon.  :)  Only works for concave polygons.
    let mut edgemask = 0;
    for edge in 0..nv {
        let mut edge_vec = Vector2D::default();
        let mut check_vec = Vector2D::default();

        let v0 = vector_list[edge].as_slice();
        let v1 = vector_list[(edge + 1) % nv].as_slice();

        edge_vec.x = v1[i] - v0[i];
        edge_vec.y = v1[j] - v0[j];

        check_vec.x -= v0[i];
        check_vec.y -= v0[j];

        let d = check_vec.x * edge_vec.y - check_vec.y * edge_vec.x;

        if d < 0.0 {
            // we are outside of triangle
            edgemask |= 1 << edge;
        }
    }

    edgemask
}

// decide it it's close enough to hit
// determine if and where a vector intersects with a sphere
// vector defined by p0,p1
// if there is an intersection this function returns 1, fills in intp, and col_dist else it returns 0
// NOTE:  Caller should account for the radius of the vector (i.e. no rad. for the vector is passed
//        to this function -- the 2 radii are additive to it is trial and it saves 1 parameter
pub fn check_vector_to_sphere(
    intp: &mut Vector,
    col_dist: &mut f32,
    p0: &Vector,
    p1: &Vector,
    sphere_pos: &Vector,
    sphere_rad: f32,
    correcting: bool,
    init_collisions: bool,
) -> bool {
    // Vector direction of line from p0 to p1
    let line_vec = *p1 - *p0;

    // Vector from p0 to the center of the sphere
    let point_to_center_vec = *sphere_pos - *p0;

    if line_vec.dot(point_to_center_vec) <= 0.0 {
        return false;
    }

    // Get the magnitude and direction of the line vector
    let mut normalized_line_vec = line_vec.clone();
    let mag_line = Vector::normalize(&mut normalized_line_vec);

    // Compute the location of the point on the line that is perpendicular to the center of the sphere
    let closet_point_dist = normalized_line_vec.dot(point_to_center_vec);

    // We check for an initial hit, so if closest_point is negative distance, it was a miss (think about it)
    // Otherwise, make sure it is not any farther than would for a collision to happen
    if closet_point_dist < 0.0 || closet_point_dist >= mag_line + sphere_rad {
        return false;
    }

    // Is the initial p0 position an intersection?  If so, warn us and collide immediately.
    if point_to_center_vec.dot(point_to_center_vec) < sphere_rad.powi(2) {
        if correcting {
            // point_to_center_vec*point_to_center_vec, sphere_rad*sphere_rad));
            // chrishack this movement intersection fix is a hack...  How do we do correct cylinder/vector interestion?
            let mut n_ptc = point_to_center_vec.clone();
            Vector::normalize(&mut n_ptc);

            *intp = *p0
                - n_ptc
                    * (sphere_rad
                        - (sphere_rad.powi(2) - point_to_center_vec.dot(point_to_center_vec))
                            .sqrt());

            *col_dist = 0.0;
            return true;
        } else {
            // If not correcting, ignore initial point collisions, as they make no sense.
            return false;
        }
    }

    let closet_point = *p0 + closet_point_dist * normalized_line_vec;
    let closest_mag_to_center = Vector::distance(&closet_point, &sphere_pos);

    // We are not moving close enough to collide with the circle
    if closest_mag_to_center >= sphere_rad {
        return false;
    }

    // Pathagorithm Theorom -- the radius is the hypothenus, the other two sides are the distance
    // from the point to the line, and the amount we should subtract from the line to account
    // for the sphere overlapping the line at the closest approach point
    let shorten = sphere_rad.powi(2) - closest_mag_to_center.powi(2);
    *col_dist = closet_point_dist - shorten;

    if *col_dist > mag_line {
        return false;
    }

    // Actual collision point
    *intp = *p0 + *col_dist * normalized_line_vec;

    return true;
}

pub fn is_point_in_cylinder(
    normal: &mut Vector,
    cylinder_point: &Vector,
    edir: &Vector,
    elen: f32,
    rad: f32,
    point: &Vector,
    mdir: &Vector,
    collide: &mut bool,
) -> bool {
    let plen = (*point - *cylinder_point).dot(*edir);

    if plen < 0.0 || plen > elen {
        return false;
    }

    let newp = *cylinder_point + *edir * plen;
    *normal = *point - newp;

    if (Vector::normalize(normal) >= rad) {
        return false;
    }

    if normal.dot(*mdir) >= 0.0 {
        *collide = false;
    } else {
        *collide = true;
    }

    true
}

/// check if a sphere intersects a face -- this can be optimized (only need 2d stuff after rotation)
pub fn check_vector_to_cylinder(
    colp: &mut Vector,
    intp: &mut Vector,
    col_dist: &mut f32,
    wall_norm: &mut Vector,
    p0: &Vector,
    p1: &Vector,
    rad: f32,
    ep0: &Vector,
    ep1: &Vector,
) -> bool {
    let mut edgevec = *ep1 - *ep0;
    let mut mvec3d = *p1 - *p0;
    let vector_len3d = Vector::normalize(&mut mvec3d);
    let edge_len = Vector::normalize(&mut edgevec);

    let mut init_normal = Vector::ZERO;
    let mut init_collide = false;

    if is_point_in_cylinder(
        &mut init_normal,
        &ep0,
        &edgevec,
        edge_len,
        rad,
        p0,
        &mvec3d,
        &mut init_collide,
    ) {
        let edge_orient = Matrix::from_vector(Some(&edgevec), None, None);

        let mut po0 = (*p0 - *ep0) * edge_orient;
        let mut po1 = (*p1 - *ep0) * edge_orient;

        po0.z = 0.0;
        po1.z = 0.0;

        let mut mvec = po1 - po0;
        let vector_len = Vector::normalize(&mut mvec);

        let dist = -(mvec.dot(po0));

        let closet_point = po0 + dist * mvec;

        let dist_from_origin = Vector::magnitude(&closet_point);

        if dist_from_origin >= rad {
            return false;
        }

        let dist_to_intersection = (rad.powi(2) - dist_from_origin.powi(2)).sqrt();

        let mut t = [0f32; 4];
        let mut valid_t = [false; 4];
        let mut valid_hit = false;

        let mut ivertex = [Vector::ZERO; 4];
        let mut cole_dist = [0f32; 4];
        let mut inte = [Vector::ZERO; 4];

        // (0.0 to 1.0) is on line
        t[0] = (dist + dist_to_intersection) / vector_len;
        t[1] = (dist - dist_to_intersection) / vector_len;

        valid_t[0] = t[0] >= 0.0 && t[0] <= 1.0;
        valid_t[1] = t[1] >= 0.0 && t[1] <= 1.0;

        for i in 0..2 {
            if valid_t[i] {
                ivertex[i] = *p0 + mvec3d * (vector_len3d * t[i]);

                let t_edge = (ivertex[i] - *ep0).dot(edgevec) / edge_len;

                if t_edge >= 0.0 && t_edge <= 1.0 {
                    cole_dist[i] = vector_len3d * t[i];
                    inte[i] = *ep0 + (ivertex[i] - *ep0).dot(edgevec) * edgevec;
                    valid_hit = true;
                } else {
                    valid_t[i] = false;
                }
            }
        }

        let mut d_vec: Vector;

        // Check end spheres
        if check_vector_to_sphere(
            &mut ivertex[2],
            &mut cole_dist[2],
            p0,
            p1,
            ep0,
            rad,
            false,
            true,
        ) {
            t[2] = cole_dist[2] / vector_len3d;
            valid_t[2] = true;
            valid_hit = true;
            d_vec = *ep1 - ivertex[2];
            Vector::normalize(&mut d_vec);
            inte[2] = ivertex[2] + rad * d_vec;
        } else {
            valid_t[2] = false;
        }

        if check_vector_to_sphere(
            &mut ivertex[3],
            &mut cole_dist[3],
            p0,
            p1,
            ep1,
            rad,
            false,
            true,
        ) {
            t[3] = cole_dist[3] / vector_len3d;
            valid_t[3] = true;
            valid_hit = true;
            d_vec = *ep1 - ivertex[3];
            Vector::normalize(&mut d_vec);
            inte[3] = ivertex[3] + rad * d_vec;
        } else {
            valid_t[3] = false;
        }

        if !valid_hit {
            return false;
        }

        let mut best_hit_index: Option<usize> = None;

        for i in 0..4 {
            if valid_t[i] {
                match best_hit_index {
                    None => best_hit_index = Some(i),
                    Some(x) => {
                        if cole_dist[i] < cole_dist[x] {
                            best_hit_index = Some(i);
                        }
                    }
                }
            }
        }

        let index = best_hit_index.unwrap();

        *colp = inte[index];
        *intp = ivertex[index];
        *col_dist = cole_dist[index];
        *wall_norm = *intp - *colp;
        Vector::normalize(wall_norm);

        return true;
    } else {
        if init_collide {
            *col_dist = 0.0;
            *wall_norm = init_normal.clone();
            *colp = *p0 - init_normal * rad;
            *intp = p0.clone();

            return true;
        } else {
            return false;
        }
    }
}

/// check if a sphere intersects a face
pub fn check_sphere_to_face(
    colp: &mut Vector,
    intp: &mut Vector,
    col_dist: &mut f32,
    wall_norm: &mut Vector,
    p0: &Vector,
    p1: &Vector,
    face_normal: &mut Vector,
    nv: usize,
    rad: f32,
    vector_list: &[Vector],
) -> bool {
    // Prevent overflow on the edgemask
    // TODO: make things just be usize?
    assert!(nv <= 32);

    let mut edgemask = check_point_to_face(colp, face_normal, nv, vector_list);

    // If we are inside edgemask is 0, we hit the face.
    if edgemask == 0 {
        *col_dist = Vector::distance(p0, intp);
        *wall_norm = *face_normal;
        return true;
    } else {
        // Although the plane collision point is not in the face, we might hit an edge.
        // If the checkpoint collides with the edge of a face, it could
        // go a little farther before hitting anything

        // If we have no radius we could only hit the face and not an edge or point
        if rad == 0.0 {
            return false;
        }

        let mut hit = false;
        let mut c_end = p1.clone();

        // get verts for edge we're behind
        for edgenum in 0..nv {
            if edgemask & 1 != 0 {
                let v0 = vector_list[edgenum];
                let v1 = vector_list[(edgenum + 1) % nv];

                if check_vector_to_cylinder(
                    colp, intp, col_dist, wall_norm, p0, &c_end, rad, &v0, &v1,
                ) {
                    c_end = intp.clone();
                    hit = true
                }
            }

            edgemask >>= 1;
        }

        return hit;
    }
}

// returns true if line intersects with face. fills in newp with intersection
// point on plane, whether or not line intersects side
// facenum determines which of four possible faces we have
// note: the seg parm is temporary, until the face itself has a point field
pub fn check_line_to_face(
    newp: &mut Vector,
    colp: &mut Vector,
    col_dist: &mut f32,
    wall_norm: &mut Vector,
    p0: &Vector,
    p1: &Vector,
    face_normal: &mut Vector,
    vector_list: &[Vector],
    nv: usize,
    rad: f32,
) -> bool {
    let mut test = &vector_list[0];
    let mut vertnum = 0;

    assert!(rad >= 0.0);

    // This is so we always use the same vertex
    for i in 0..nv {
        let vec_ref = &vector_list[i];

        if addr_of!(test) > addr_of!(vec_ref) {
            test = &vector_list[i];
            vertnum = i;
        }
    }

    // Determine the intersection point between the plane(of the face) and the line
    // This point is the center of the circle (not the edge)
    let pli =
        find_plane_line_intersection(newp, colp, &vector_list[vertnum], face_normal, p0, p1, rad);

    if !pli {
        return false;
    }

    return check_sphere_to_face(
        colp,
        newp,
        col_dist,
        wall_norm,
        p0,
        p1,
        face_normal,
        nv,
        rad,
        vector_list,
    );

    todo!()
}

// chrishack -- check this later
// computes the parameters of closest approach of two lines
// fill in two parameters, t0 & t1.  returns 0 if lines are parallel, else 1
pub fn check_line_to_line(
    t1: &mut f32,
    t2: &mut f32,
    p1: &Vector,
    v1: &Vector,
    p2: &Vector,
    v2: &Vector,
) -> bool {
    let mut det = Matrix::ZERO;

    det.right = *p2 - *p1;
    det.forward = v1.cross(&v2);

    let cross_mag2 = det.forward.dot(det.forward);

    if cross_mag2 == 0.0 {
        return false;
        // lines are parallel
    }

    det.up = v2.clone();
    let d = Matrix::compute_determinant(&det);
    *t1 = d / cross_mag2;

    det.up = v1.clone();
    let d = Matrix::compute_determinant(&det);
    *t2 = d / cross_mag2;

    // found point
    true
}

// determine if a vector intersects with an object
// if no intersects, returns 0, else fills in intp and returns dist
pub fn check_vector_to_object(
    query: &Query,
    intp: &mut Vector,
    col_dist: &mut f32,
    p0: &Vector,
    p1: &Vector,
    rad: f32,
    still_object: &Object,
    still_object_poly: Option<&PolyModel>,
    fvi_object: &Object,
) -> bool {
    let mut still_pos = still_object.position.clone();
    let mut still_size = 0f32;

    let class = still_object.typedef().class;
    let beh = &still_object.typedef().behavior;

    if beh.drawable.is_some()
        && still_object.typedef().class != ObjectClass::Powerup
        && still_object.typedef().class != ObjectClass::Weapon
        || still_object.typedef().class != ObjectClass::Debris
        || still_object.typedef().class != ObjectClass::Room
        || still_object.typedef().class != ObjectClass::Player
    {
        still_size = still_object_poly.unwrap().anim_size;
        still_pos = still_pos + still_object.anim_sphere_offset;
    } else {
        still_size = still_object.size;
    }

    // This accounts for relative position vs. relative velocity
    match query.this_obj.as_ref() {
        None => {}
        Some(fvi_obj_ref) => {
            let fvi_obj = fvi_obj_ref.borrow();

            let still_movement = &still_object.dyn_behavior.movement.as_ref();
            let fvi_movement = &fvi_obj.dyn_behavior.movement.as_ref();

            if still_movement.is_some() && fvi_movement.is_some() {
                let still_movement = still_movement.unwrap();
                let fvi_movement = fvi_movement.unwrap();

                match still_movement {
                    MovementType::Physical(x) => match fvi_movement {
                        MovementType::Physical(y) => {
                            if class != ObjectClass::Powerup
                                && fvi_obj.typedef().class != ObjectClass::Powerup
                            {
                                let temp = still_pos - fvi_obj.position;
                                let temp = temp.dot(x.velocity - y.velocity);

                                if temp > 0.0 {
                                    return false;
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    let total_size = still_size + rad;

    return check_vector_to_sphere(intp, col_dist, p0, p1, &still_pos, total_size, false, true);
}

pub fn object_object_AABB(a: &Object, b: &Object) -> bool {
    if a.max_xzy.x < b.min_xzy.x
        || b.max_xzy.x < a.min_xzy.x
        || a.max_xzy.z < b.min_xzy.z
        || b.max_xzy.z < a.min_xzy.z
        || a.max_xzy.y < b.min_xzy.y
        || b.max_xzy.y < a.min_xzy.y
    {
        // no overlap
        return false;
    }

    true
}

pub fn object_room_AABB(obj: &Object, face: &Face) -> bool {
    if obj.max_xzy.y < face.min_xyz.z
        || face.max_xyz.y < obj.min_xzy.y
        || obj.max_xzy.x < face.min_xyz.x
        || face.max_xyz.x < obj.min_xzy.x
        || obj.max_xzy.z < face.min_xyz.z
        || face.max_xyz.z < obj.min_xzy.z
    {
        return false;
    }

    true
}

pub fn room_manual_AABB(face: &Face, min_xyz: &Vector, max_xyz: &Vector) -> bool {
    if max_xyz.y < face.min_xyz.y
        || face.max_xyz.y < min_xyz.y
        || max_xyz.x < face.min_xyz.x
        || face.max_xyz.x < min_xyz.x
        || max_xyz.z < face.min_xyz.z
        || face.max_xyz.z < min_xyz.z
    {
        return false;
    }

    true
}

pub fn process_cells<F>(
    initial_cell: usize,
    position: &Vector,
    rad: f32,
    terrain: &Terrain,
    mut condition: F,
) -> usize
where
    F: FnMut(usize) -> bool,
{
    // Calculate boundaries (x_start, y_start, etc.)
    let check_x = (rad / TERRAIN_SIZE) as usize + 1;
    let check_y = (rad / TERRAIN_SIZE) as usize + 1;

    let remainder = initial_cell % TERRAIN_WIDTH;
    let quotient = initial_cell / TERRAIN_WIDTH;

    let x_start = remainder.saturating_sub(check_x);
    let x_end = (remainder + check_x).min(TERRAIN_WIDTH - 1);
    let y_start = quotient.saturating_sub(check_y);
    let y_end = (quotient + check_y).min(TERRAIN_DEPTH - 1);

    // Initial node position
    let mut current_node = TERRAIN_WIDTH * y_start + x_start;
    let next_y_delta = TERRAIN_WIDTH - (x_end - x_start) - 1;
    let mut num_cells = 0;

    // Iterate over the cells within the calculated boundaries
    for _y in y_start..=y_end {
        for _x in x_start..=x_end {
            // Call the closure with the current node
            if !condition(current_node) {
                break;
            }
            current_node += 1;
        }
        current_node += next_y_delta;
    }

    num_cells
}
