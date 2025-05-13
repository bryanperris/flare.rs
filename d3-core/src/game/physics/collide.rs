use std::collections::HashMap;

use matrix::Matrix;
use vector::Vector;
use vector2d::Vector2D;

use crate::{game::{
    context::{self, GameContext}, object, object_dynamic_behavior::MovementType, object_static_behavior::PhysicsFlags, room::{get_ij, Room}, GameMode
}, graphics::texture::{Texture16, TextureFlags}, rand::ps_rand};

use super::{super::prelude::*, intersection::IntersectionFinder, physics_apply_force, physics_apply_rot};

const PLAYER_ROTATION_BY_FORCE_SCALAR: f32 = 0.12;
const NONPLAYER_ROTATION_BY_FORCE_SCALAR: f32 = 1.0;
const FORCEFIELD_DAMAGE: f32 = 5.0;
const MIN_WALL_HIT_SOUND_VEL: f32 = 40.0;
const MAX_WALL_HIT_SOUND_VEL: f32 = 120.0;
const MIN_PLAYER_WALL_SOUND_TIME: f32 = 0.1;
const WALL_DAMAGE: f32 = 0.5;
const MIN_WALL_HIT_DAMAGE_SHIELDS: i32 = 5;
const MIN_WALL_DAMAGE_SPEED: f32 = 65.0;
const VOLATILE_DAMAGE: f32 = 7.0;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CollisionResultType {
    Nothing,
    CheckSphereSphere,
    CheckSpherePoly,
    CheckPolySphere,
    CheckBBoxPoly,
    CheckPolyBBox,
    CheckBBoxBBox,
    CheckBBoxSphere,
    CheckSphereBBox,
    CheckSphereRoom,
    CheckBBoxRoom,
}

pub struct CollisionMap {
    result_map: [[CollisionResultType; ObjectClass::Waypoint as usize + 1]; ObjectClass::Waypoint as usize + 1],
    ray_result: [CollisionResultType; ObjectClass::Waypoint as usize + 1]
}

impl CollisionMap {
    fn set_ray_result(&mut self, class: ObjectClass, result: CollisionResultType) {
        self.ray_result[class as usize] = result;
    }

    /// Enables collision detection of `CheckSphereSphere` between `type1` and `type2`.
    fn enable_collision_sphere_sphere(&mut self, type1: ObjectClass, type2: ObjectClass) {
        self.result_map[type1 as usize][type2 as usize] = CollisionResultType::CheckSphereSphere;
        self.result_map[type2 as usize][type1 as usize] = CollisionResultType::CheckSphereSphere;
    }

    /// Enables collision detection of `CheckSpherePoly` between `type1` and `type2`.
    fn enable_collision_sphere_poly(&mut self, type1: ObjectClass, type2: ObjectClass) {
        self.result_map[type1 as usize][type2 as usize] = CollisionResultType::CheckSpherePoly;
        self.result_map[type2 as usize][type1 as usize] = CollisionResultType::CheckPolySphere;
    }

    /// Enables collision detection of `CheckPolySphere` between `type1` and `type2`.
    fn enable_collision_poly_sphere(&mut self, type1: ObjectClass, type2: ObjectClass) {
        self.result_map[type1 as usize][type2 as usize] = CollisionResultType::CheckPolySphere;
        self.result_map[type2 as usize][type1 as usize] = CollisionResultType::CheckSpherePoly;
    }

    /// Enables collision detection of `CheckBBoxPoly` between `type1` and `type2`.
    fn enable_collision_bbox_poly(&mut self, type1: ObjectClass, type2: ObjectClass) {
        self.result_map[type1 as usize][type2 as usize] = CollisionResultType::CheckBBoxPoly;
        self.result_map[type2 as usize][type1 as usize] = CollisionResultType::CheckPolyBBox;
    }

    /// Enables collision detection of `CheckPolyBBox` between `type1` and `type2`.
    fn enable_collision_poly_bbox(&mut self, type1: ObjectClass, type2: ObjectClass) {
        self.result_map[type1 as usize][type2 as usize] = CollisionResultType::CheckPolyBBox;
        self.result_map[type2 as usize][type1 as usize] = CollisionResultType::CheckBBoxPoly;
    }

    /// Enables collision detection of `CheckBBoxBBox` between `type1` and `type2`.
    fn enable_collision_bbox_bbox(&mut self, type1: ObjectClass, type2: ObjectClass) {
        self.result_map[type1 as usize][type2 as usize] = CollisionResultType::CheckBBoxBBox;
        self.result_map[type2 as usize][type1 as usize] = CollisionResultType::CheckBBoxBBox;
    }

    /// Enables collision detection of `CheckBBoxSphere` between `type1` and `type2`.
    fn enable_collision_bbox_sphere(&mut self, type1: ObjectClass, type2: ObjectClass) {
        self.result_map[type1 as usize][type2 as usize] = CollisionResultType::CheckBBoxSphere;
        self.result_map[type2 as usize][type1 as usize] = CollisionResultType::CheckSphereBBox;
    }

    /// Enables collision detection of `CheckSphereBBox` between `type1` and `type2`.
    fn enable_collision_sphere_bbox(&mut self, type1: ObjectClass, type2: ObjectClass) {
        self.result_map[type1 as usize][type2 as usize] = CollisionResultType::CheckSphereBBox;
        self.result_map[type2 as usize][type1 as usize] = CollisionResultType::CheckBBoxSphere;
    }

    /// Enables collision detection of `CheckSphereRoom` between `type1` and `type2`.
    fn enable_collision_sphere_room(&mut self, type1: ObjectClass, type2: ObjectClass) {
        self.result_map[type1 as usize][type2 as usize] = CollisionResultType::CheckSphereRoom;
        self.result_map[type2 as usize][type1 as usize] = CollisionResultType::CheckSphereRoom;
    }

    /// Enables collision detection of `CheckBBoxRoom` between `type1` and `type2`.
    fn enable_collision_bbox_room(&mut self, type1: ObjectClass, type2: ObjectClass) {
        self.result_map[type1 as usize][type2 as usize] = CollisionResultType::CheckBBoxRoom;
        self.result_map[type2 as usize][type1 as usize] = CollisionResultType::CheckBBoxRoom;
    }

    /// Disables collision detection between `type1` and `type2`.
    fn disable_collision(&mut self, type1: ObjectClass, type2: ObjectClass) {
        self.result_map[type1 as usize][type2 as usize] = CollisionResultType::Nothing;
        self.result_map[type2 as usize][type1 as usize] = CollisionResultType::Nothing;
    }
}

impl Default for CollisionMap {
    fn default() -> Self {
        let mut s = Self { 
            result_map: [[CollisionResultType::Nothing; ObjectClass::Waypoint as usize + 1]; ObjectClass::Waypoint as usize + 1],
            ray_result: [CollisionResultType::Nothing; ObjectClass::Waypoint as usize + 1]
        };

        s.set_ray_result(ObjectClass::Robot, CollisionResultType::CheckSpherePoly);
        s.set_ray_result(ObjectClass::Player, CollisionResultType::CheckSpherePoly);
        s.set_ray_result(ObjectClass::Weapon, CollisionResultType::CheckSpherePoly);
        s.set_ray_result(ObjectClass::Powerup, CollisionResultType::CheckSpherePoly);
        s.set_ray_result(ObjectClass::Clutter, CollisionResultType::CheckSpherePoly);
        s.set_ray_result(ObjectClass::Building, CollisionResultType::CheckSpherePoly);
        s.set_ray_result(ObjectClass::Door, CollisionResultType::CheckSpherePoly);
        s.set_ray_result(ObjectClass::Room, CollisionResultType::CheckSpherePoly);

        // Enable sphere-room collisions for all object types.
        for i in 0..=ObjectClass::Waypoint as usize {
            s.enable_collision_sphere_room(ObjectClass::from(i), ObjectClass::Room);
        }

        // Enable specific collision rules.
        s.enable_collision_poly_sphere(ObjectClass::Wall, ObjectClass::Robot);
        s.enable_collision_poly_sphere(ObjectClass::Wall, ObjectClass::Weapon);
        s.enable_collision_poly_sphere(ObjectClass::Wall, ObjectClass::Player);
        s.enable_collision_sphere_sphere(ObjectClass::Robot, ObjectClass::Robot);
        // s.enable_collision_sphere_sphere(ObjectClass::Building, ObjectClass::Building);
        s.enable_collision_poly_sphere(ObjectClass::Player, ObjectClass::Fireball);
        s.enable_collision_sphere_sphere(ObjectClass::Player, ObjectClass::Player);
        s.enable_collision_sphere_sphere(ObjectClass::Player, ObjectClass::Marker);
        s.enable_collision_sphere_sphere(ObjectClass::Marker, ObjectClass::Player);
        s.enable_collision_sphere_sphere(ObjectClass::Weapon, ObjectClass::Weapon);
        s.enable_collision_poly_sphere(ObjectClass::Robot, ObjectClass::Player);
        // s.enable_collision_sphere_sphere(ObjectClass::Robot, ObjectClass::Player);
        s.enable_collision_poly_sphere(ObjectClass::Robot, ObjectClass::Weapon);

        s.enable_collision_poly_sphere(ObjectClass::Player, ObjectClass::Weapon);
        s.enable_collision_sphere_sphere(ObjectClass::Player, ObjectClass::Powerup);
        s.enable_collision_sphere_sphere(ObjectClass::Powerup, ObjectClass::Wall);
        s.enable_collision_sphere_poly(ObjectClass::Weapon, ObjectClass::Clutter);
        s.enable_collision_sphere_poly(ObjectClass::Player, ObjectClass::Clutter);
        s.enable_collision_sphere_sphere(ObjectClass::Clutter, ObjectClass::Clutter);
        s.enable_collision_sphere_poly(ObjectClass::Robot, ObjectClass::Clutter);
        s.enable_collision_sphere_poly(ObjectClass::Player, ObjectClass::Building);
        s.enable_collision_sphere_poly(ObjectClass::Robot, ObjectClass::Building);
        s.enable_collision_sphere_poly(ObjectClass::Weapon, ObjectClass::Building);
        s.enable_collision_sphere_poly(ObjectClass::Clutter, ObjectClass::Building);
        s.enable_collision_sphere_poly(ObjectClass::Clutter, ObjectClass::Door);
        s.enable_collision_sphere_poly(ObjectClass::Building, ObjectClass::Door);

        s.enable_collision_sphere_room(ObjectClass::Player, ObjectClass::Room);
        s.enable_collision_sphere_room(ObjectClass::Robot, ObjectClass::Room);
        s.enable_collision_sphere_room(ObjectClass::Weapon, ObjectClass::Room);
        s.enable_collision_sphere_room(ObjectClass::Viewer, ObjectClass::Room);

        s.enable_collision_sphere_poly(ObjectClass::Player, ObjectClass::Door);
        s.enable_collision_sphere_poly(ObjectClass::Robot, ObjectClass::Door);
        s.enable_collision_sphere_poly(ObjectClass::Weapon, ObjectClass::Door);

        s.disable_collision(ObjectClass::Powerup, ObjectClass::Powerup);

        s
    }
}

/*
TODO:

#define COLLISION_OF(a, b) (((a) << 8) + (b))

#define DO_COLLISION(type1, type2, collision_function)                                                                 \
  case COLLISION_OF((type1), (type2)):                                                                                 \
    (collision_function)((A), (B), collision_point, collision_normal, false, hit_info);                                \
    break;                                                                                                             \
  case COLLISION_OF((type2), (type1)):                                                                                 \
    (collision_function)((B), (A), collision_point, collision_normal, true, hit_info);                                 \
    break;

#define DO_SAME_COLLISION(type1, type2, collision_function)                                                            \
  case COLLISION_OF((type1), (type1)):                                                                                 \
    (collision_function)((A), (B), collision_point, collision_normal, false, hit_info);                                \
    break;

#define NO_COLLISION(type1, type2)                                                                                     \
  case COLLISION_OF((type1), (type2)):                                                                                 \
  case COLLISION_OF((type2), (type1)):                                                                                 \
    break;


*/

pub fn can_apply_force(context: &GameContext, object_ref: &SharedMutRef<Object>) -> bool {
    let object = object_ref.borrow();

    let mut is_server = false;

    #[cfg(feature = "dedicated_server")]
    {
        is_server = true;
    }

    if context.mode.contains(GameMode::MULTI) {
        if object.typedef().class == ObjectClass::Player {
            if !Rc::ptr_eq(object_ref, &context.player_object_ref) {
                return false;
            }
        } else {
            if object.typedef().class != ObjectClass::Weapon
                && object.typedef().class != ObjectClass::Powerup
                && !is_server
            {
                return false;
            }
        }
    }

    match object.typedef().behavior.physical {
        Some(p) => {
            if p.mass == 0.0 {
                return false;
            }

            if p.flags.contains(PhysicsFlags::PERSISTENT) {
                return false;
            }

            if p.flags.contains(PhysicsFlags::LOCK_MASK) {
                return false;
            }
        }
        _ => {
            return false;
        }
    }

    match object.dyn_behavior.movement {
        Some(ref m) => match m {
            MovementType::Physical(_) | MovementType::Walking => {}
            _ => {
                return false;
            }
        },
        _ => {
            return false;
        }
    }

    true
}

pub fn bump_this_object(
    context: &GameContext,
    a_ref: &SharedMutRef<Object>,
    b_ref: &SharedMutRef<Object>,
    force: &Vector,
    collision_point: &Vector,
    damange_flag: i32,
) {
    let a = a_ref.borrow();
    let b = b_ref.borrow();

    let mut is_server = false;

    #[cfg(feature = "dedicated_server")]
    {
        is_server = true;
    }

    if a.typedef().class == ObjectClass::Player {
        if context.mode.contains(GameMode::MULTI) && !Rc::ptr_eq(a_ref, b_ref) {
            return;
        }
    } else {
        if context.mode.contains(GameMode::MULTI)
            && a.typedef().class != ObjectClass::Player
            && a.typedef().class != ObjectClass::Powerup
            && is_server
        {
            return;
        }

        physics_apply_force(&a, force, None);
        physics_apply_rot(&a, force);
    }
}

// finds the uv coords of the given point on the given seg & side
// fills in u & v. if l is non-NULL fills it in also
pub fn find_hitpoint_uv(u: &mut f32, v: &mut f32, point: &Vector, room: &Room, face_num: usize) {
    let mut ii = 0;
    let mut jj = 0;
    let face = &room.faces[face_num];

    // 1. find what plane to project this wall onto to make it a 2d case
    get_ij(&face.normal, &mut ii, &mut jj);

    // 2. compute u,v of intersection point

    // Copy face points into 2d verts array
    let mut point2d = [Vector2D::default(); 3];
    for i in 0..point2d.len() {
        let t = room.vertices[face.face_verts[i]].as_slice();
        point2d[i] = Vector2D {
            x: t[ii],
            y: t[jj]
        }
    }

    let t = point.as_slice();
    
    let checkpoint = Vector2D {
        x: t[ii],
        y: t[jj]
    };

     // vec from 1 -> 0
     let vec0 = Vector2D {
        x: point2d[0].x - point2d[1].x,
        y: point2d[0].y - point2d[1].y
     };

     // vec from 1 -> 2
     let vec1 = Vector2D {
        x: point2d[2].x - point2d[1].x,
        y: point2d[2].y - point2d[1].y
     };

     let k1 = -((checkpoint.cross(&vec0) + vec0.cross(&point2d[1])) / vec0.cross(&vec1));
     let k0;

     if vec0.x.abs() > vec0.y.abs() {
        k0 = ((-k1 * vec1.x) + checkpoint.x - point2d[1].x) / vec0.x;
     }
     else {
        k0 = ((-k1 * vec1.y) + checkpoint.y - point2d[1].y) / vec0.x;
     }


    *u = face.face_uvls[1].u + (k0 * (face.face_uvls[0].u - face.face_uvls[1].u)) + (k1 * (face.face_uvls[2].u - face.face_uvls[1].u));
    *v = face.face_uvls[1].v + (k0 * (face.face_uvls[0].v - face.face_uvls[1].v)) + (k1 * (face.face_uvls[2].v - face.face_uvls[1].v));

}

/// Creates some effects where a weapon has collided with a wall
pub fn do_wall_effects(weapon: &Object, surface_texture: &Texture16) {
    let is_water = surface_texture.flags.contains(TextureFlags::WATER);

    if surface_texture.flags.contains(TextureFlags::VOLATILE) ||
       surface_texture.flags.contains(TextureFlags::LAVA) ||
       is_water {
            // Create some lava steam
            let mut rand = crate::create_rng();

            if is_water || (ps_rand(&mut rand) % 4) == 0 {
                
            }
       }
}

/// Check for lava, volatile, or water surface.  If contact, make special sound & kill the weapon
pub fn check_for_special_surface(weapon: &Object, surface_tmap: usize, surface_norma: &Vector, hit_dot: f32) {
    todo!()
}


/// Process a collision between a weapon and a wall
//// Returns true if the weapon hits the wall, and false if should keep going though the wall (for breakable glass)
pub fn collide_weapon_and_wall(weapon: &Object, hitspeed: i64, hitseg: i32, hitwall: i32, hitpoint: &Vector, wall_normal: &Vector, hit_dot: f32) {
    todo!()
}

/// Prints out a marker hud message if needed
pub fn collide_player_and_marker(player_object: &Object, marker_obj: &Object, collision_point: &Vector, collision_normal: &Vector, reverse_normal: bool, fvi: &IntersectionFinder) {
    todo!()
}

// Function signatures converted to Rust
fn collide_player_and_wall(
    player_obj: &mut Object,
    hit_speed: f32,
    hit_seg: i32,
    hit_wall: i32,
    hit_pt: &Vector,
    wall_normal: &Vector,
    hit_dot: f32,
) {
    // Function body to be implemented
}

fn collide_generic_and_wall(
    generic_obj: &mut Object,
    hit_speed: f32,
    hit_seg: i32,
    hit_wall: i32,
    hit_pt: &Vector,
    wall_normal: &Vector,
    hit_dot: f32,
) {
    // Function body to be implemented
}

// This gets called when an object is scraping along the wall
fn scrape_object_on_wall(
    obj: &mut Object,
    hit_seg: i32,
    hit_wall: i32,
    hit_pt: &Vector,
    wall_normal: &Vector,
) {
    // Function body to be implemented
}

fn collide_angles_to_matrix(m: &mut Matrix, p: f32, h: f32, b: f32) {
    // Function body to be implemented
    todo!();
}

fn collide_extract_angles_from_matrix(a: &mut Vector, m: &Matrix) -> Vector {
    // Function body to be implemented
    todo!()
}

fn convert_euler_to_axis_amount(e: &Vector, n: &Vector, w: &mut f32) {
    // Function body to be implemented
}

fn convert_axis_amount_to_euler(n: &Vector, w: &f32, e: &mut Vector) {
    // Function body to be implemented
}

fn bump_obj_against_fixed(obj: &mut Object, collision_point: &Vector, collision_normal: &Vector) {
    // Function body to be implemented
}

fn bump_two_objects(
    object0: &mut Object,
    object1: &mut Object,
    collision_point: &Vector,
    collision_normal: &Vector,
    damage_flag: bool,
) {
    todo!()
}

fn collide_player_and_player(
    p1: &mut Object,
    p2: &mut Object,
    collision_point: &Vector,
    collision_normal: &Vector,
    f_reverse_normal: bool,
    hit_info: &mut IntersectionFinder,
) {
    todo!()
}

fn collide_generic_and_player(
    robot_obj: &mut Object,
    player_obj: &mut Object,
    collision_point: &Vector,
    collision_normal: &Vector,
    f_reverse_normal: bool,
    hit_info: &mut IntersectionFinder,
) {
    todo!()
}

fn make_weapon_stick(weapon: &mut Object, parent: &mut Object, hit_info: &mut IntersectionFinder) {
    todo!()
}

fn collide_generic_and_weapon(
    robot_obj: &mut Object,
    weapon: &mut Object,
    collision_point: &Vector,
    collision_normal: &Vector,
) {
    todo!()
}

fn collide_player_and_weapon(
    player_obj: &mut Object,
    weapon: &mut Object,
    collision_point: &Vector,
    collision_normal: &Vector,
    f_reverse_normal: bool,
    hit_info: &mut IntersectionFinder,
) {
    todo!()
}

fn check_lg_inform(a: &mut Object, b: &mut Object) {
    todo!()
}

fn collide_two_objects(
    a: &mut Object,
    b: &mut Object,
    collision_point: &Vector,
    collision_normal: &Vector,
    hit_info: &mut IntersectionFinder,
) {
    todo!()
}

// Process a collision between an object and a wall
// Returns true if the object hits the wall, and false if it should keep going through the wall (for breakable glass)
fn collide_object_with_wall(
    a: &mut Object,
    hit_speed: f32,
    hit_seg: i32,
    hit_wall: i32,
    hit_pt: &Vector,
    wall_normal: &Vector,
    hit_dot: f32,
) -> bool {
    todo!()
}