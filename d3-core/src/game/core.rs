/* Implement the game core logic here */
use crate::{game::door::{DoorwayFlags, KeyFlags}, gr_rgb};
use crate::graphics::ddgr_color;

use super::{context::GameContext, door::{self, Doorway, DoorwayState}, node::Node, prelude::*, room::Room, terrain::{self, Terrain}, weather::Weather, RegionRef};

pub fn remove_active_doorway(context: &mut GameContext, doorway: &SharedMutRef<Doorway>) {
    context.doorways.remove_by_ref(doorway);

    /* We need to remove the door info from the room that had its door destroyed */
    for bounded_room in context.rooms.bindings() {
        let mut room = bounded_room.inner().borrow_mut();
        
        if room.assigned_door_data.is_some() {
            let door_data = room.assigned_door_data.as_ref().unwrap();

            if Rc::ptr_eq(door_data.doorway(), doorway) {
                room.assigned_door_data = None;
            }
        }
    }
}

pub fn update_doorway_animation(room: SharedMutRef<Room>) {
    todo!()
    // DoorwayUpdateAnimation
}

///
/// Update all the active doorways in the mine
pub fn do_frame_doorways(context: &mut GameContext) {
    let mut doorways_to_remove: Vec<SharedMutRef<Doorway>> = Vec::new();

    for bounded_doorway in context.doorways.bindings() {
        let doorway_ref = bounded_doorway.inner();
        let mut doorway = doorway_ref.borrow_mut();
        
        if !doorway.is_active {
            continue;
        }

        let door_ref = doorway.assigned_door.as_ref().unwrap().clone();
        let mut door = door_ref.borrow_mut();
        
        assert_eq!(doorway.assigned_room.is_none(), false);

        let door_room_ref = doorway.assigned_room.as_ref().unwrap().clone();
        let door_room = door_room_ref.borrow();

        match doorway.state {
            DoorwayState::Opening | DoorwayState::OpeningAuto => {
                let delta = context.frametime() / door.total_time_open;

                if doorway.position >= doorway.dest_pos {
                    doorway.position = doorway.dest_pos;

                    match doorway.state {
                        DoorwayState::OpeningAuto => {
                            doorway.state = DoorwayState::Waiting;
                            doorway.dest_pos = door.total_time_open;
                        },
                        _ => {
                            doorway.state = DoorwayState::Stopped;
                            doorway.is_active = false;

                            doorways_to_remove.push(doorway_ref.clone());
                        }
                    }
                }
            },
            DoorwayState::Closing => {
                let delta = context.frametime() / door.total_close_time;

                doorway.position -= delta;

                if doorway.position <= doorway.dest_pos {
                    doorway.position = doorway.dest_pos;

                    doorway.state = DoorwayState::Stopped;

                    doorways_to_remove.push(doorway_ref.clone());

                    context.script_runtime.signal_event(
                        crate::game::scripting::EventType::DoorClose, 
                        None,
                        door_room.assigned_door_data.as_ref().unwrap().door_obj().clone()
                    );
                }
            },
            DoorwayState::Waiting => {
                doorway.dest_pos -= context.frametime();

                if doorway.dest_pos <= 0.0 {
                    let door_obj_ref = door_room.assigned_door_data.as_ref().unwrap().door_obj();
                    let door_obj = door_obj_ref.borrow();

                    /* Start closing the door, check if anything is in the way */
                    if door_obj.link_prev_obj.is_none() && door_obj.link_next_obj.is_none() {
                        doorway.dest_pos = 0.0;
                        doorway.state = DoorwayState::Closing;

                        bounded_doorway.play_sound(&door_obj);
                    }
                }
            },
            DoorwayState::Stopped => {
                panic!("stopped doorstate is not allowed");
            }
        }

        update_doorway_animation(door_room_ref.clone());
    }

    for doorway_ref in &doorways_to_remove {
        remove_active_doorway(context, doorway_ref);
    }
}

pub fn check_doorway_openable(context: &mut GameContext, door_obj_ref: &SharedMutRef<Object>, opener_ref: &SharedMutRef<Object>) -> bool {
    let door_obj = door_obj_ref.borrow();
    let room_ref = door_obj.parent_room.upgrade().unwrap();
    let room = room_ref.borrow();
    
    assert_eq!(room.assigned_door_data.is_some(), true);

    let assigned_door_data = room.assigned_door_data.as_ref().unwrap();
    let doorway_ref = assigned_door_data.doorway();
    let doorway = doorway_ref.borrow();
    
    if doorway.flags.contains(DoorwayFlags::LOCKED) {
        return false;
    }

    if doorway.keys_needed.is_empty() {
        return true;
    }

    let opener = opener_ref.borrow();

    let mut keys = KeyFlags::empty();

    match opener.typedef().class {
        ObjectClass::Weapon => {
            todo!()
        }
        ObjectClass::Player => {
            match opener.typedef().class {
                ObjectClass::Player => {
                    keys = todo!("get player keys")
                },
                ObjectClass::Building | ObjectClass::Robot => {
                    let automnomous_info = opener.typedef().behavior.autonomous.as_ref();

                    match automnomous_info {
                        Some(_) => {
                            keys = context.world_keys;
                        },
                        _ => {}
                    }
                },
                _ => {}
            }
        }
        _ => {}
    }

    // Check if player has proper keys
    if doorway.flags.contains(DoorwayFlags::KEY_ONLY_ONE) {
        return keys.contains(doorway.keys_needed)
    }
    else {
        return (keys & doorway.keys_needed) == doorway.keys_needed
    }
}

pub fn make_new_terrain(context: &mut GameContext) {
    let mut bounded_terrain_ref = context.terrain.only_one_mut();
    bounded_terrain_ref.swap_and_drop(new_shared_mut_ref(Terrain::default()));

    let mut bounded_weather_ref = context.weather.only_one_mut();
    bounded_weather_ref.swap_and_drop(new_shared_mut_ref(Weather::default()));


    // We also setup the sky texturing here
    let first_top_texture = todo!("Find a texture by name");
    let terrain_ref = bounded_terrain_ref.inner();
    let terrain = terrain_ref.borrow_mut();
    // terrain.sky.dome_texture = todo!();

    terrain.sky.sky_color = gr_rgb!(0, 0, 255);

    if !terrain.sky.is_textured {
        terrain.sky.sky_color = gr_rgb!(8, 0, 32);
        terrain.sky.fog_color = gr_rgb!(4, 0, 16);
        terrain.sky.horizon.color = gr_rgb!(128, 32, 32);
        terrain.sky.is_textured = true;
        terrain.sky.rotate_rate = 0.0;
    }
}

pub fn get_node_list(context: &mut GameContext, region: Option<RegionRef>) -> Option<SharedMutRef<Vec<Node>>> {
    if region.is_none() {
        let region = region.unwrap();

        match region {
            RegionRef::Room(r) => {
                let room = r.borrow();
                return Some(room.nodes.clone());
            },
            RegionRef::Terrain((t,i)) => {
                debug!("terrain node list requested");

                if !super::room::is_room_outside(i) {
                    if i <= (context.room_highest_index + 8) {
                        panic!("We hit this spot...");
                    }
                }

                let terrain = t.borrow();
                let region = terrain.lookup_region(i);
                let list = terrain.node_lists[region].clone();

                return Some(list);
            }
        }
    }

    None
}
