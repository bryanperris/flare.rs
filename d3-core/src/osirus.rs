// use std::{rc::Rc, sync::{Arc, Mutex}};
// use crate::game::object::ObjectType;
// use log::{info};

// use bitflags::bitflags;

// const OSIRUS_MAX_MODULES: usize = 64;

// bitflags! {
//     /// Represents a set of flags.
//     #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
//     struct OsirusModuleFlags: u8 {
//         const None  =       0b00000000;
//         /// Slot in use
//         const InUse =       0b00000001;
//         /// Level module
//         const LevelType =   0b00000010;
//         /// DLL Elsewhere (mission module)
//         const MissionType = 0b00000100;
//         /// DLL extract from a hog and it is in a temp directory
//         const TempDirType = 0b00001000;
//         /// No Unloading - // the dll should not be unloaded if the reference count is 0, only when the level ends
//         const NoUnload =    0b00010000;
//     }
// }

// bitflags! {
//     #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
//     struct OsirusExtractedScriptFlags: u8 {
//         const None  =       0b00000000;
//         /// Slot in use
//         const Used =        0b00000001;
//         /// Level module
//         const Mission =     0b00000010;
//     }
// }

// bitflags! {
//     #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
//     struct OsirusEventMasks: u8 {
//         const None  =      0b00000000;
//         const Objects  =   0b00000001;
//         /// Slot in use
//         const Triggers =   0b00000010;
//         /// Level module
//         const Levels =     0b00000100;
//     }
// }

// type SafeOsirusModule = Arc<OsirusModule>;

// lazy_static! {
//     static ref LOADED_MODULES: Mutex<Vec<SafeOsirusModule>> = 
//         Mutex::new(vec![SafeOsirusModule::new(OsirusModule::default()); OSIRUS_MAX_MODULES]);
// }

// // struct ObjectRef {
// //     object_number: i32,
// //     object_type: ObjectType, // TODO: is it ok to use objectType with the enum?
// //     is_dummy: bool,
// //     next: Option<Box<ObjectRef>>
// // }

// pub struct tOSIRISModuleInit;
// pub struct tOSIRISEventInfo;

// // TODO: we must figured out the Init struct better here, one to pass to a DL

// pub trait ModuleBinding: std::fmt::Debug {
//     fn describe(&self) -> String;
//     fn initialize_dll(&self, function_list: *mut tOSIRISModuleInit) -> char;
//     fn shutdown_dll(&self);
//     fn get_go_script_id(&self, name: *const char, isdoor: u8) -> i32;
//     fn create_instance(&self, id: i32) -> *mut std::ffi::c_void;
//     fn destroy_instance(&self, id: i32, ptr: *mut std::ffi::c_void);
//     fn call_instance_event(&self, id: i32, ptr: *mut std::ffi::c_void, event: i32, data: *mut tOSIRISEventInfo) -> i16;
//     fn get_trigger_script_id(&self, trigger_room: i32, trigger_face: i32) -> i32;
//     fn get_co_script_list(&self, list: *mut *mut i32, id_list: *mut *mut i32) -> i32;
//     fn save_restore_state(&self, file_ptr: *mut std::ffi::c_void, saving_state: u8) -> i32;
// }

// // Use Rc<OsirusModule> when managing these modules
// #[derive(Debug)]
// struct OsirusModule {
//     flags: OsirusModuleFlags,
//     extracted_id: u8,
//     binded_script: Option<Box<dyn ModuleBinding>>,

//     // TODO: figure out 'module mod' field
//     name: &'static str,
//     string_table: Vec<&'static str>
// }

// impl Drop for OsirusModule {
//     fn drop(&mut self) {
//         todo!("execute DLL shutdown!")
//         // See OsirusLoadandBind.cpp line 660
//     }
// }

// unsafe impl Send for OsirusModule {

// }

// unsafe impl Sync for OsirusModule {

// }

// impl Default for OsirusModule {
//     fn default() -> Self {
//         Self { 
//             flags: OsirusModuleFlags::None, 
//             extracted_id: Default::default(), 
//             name: Default::default(), 
//             string_table: Default::default(),
//             binded_script: None
//         }
//     }
// }

// #[derive(Debug, Clone)]
// struct OsirusCurrentLevel {
//     is_loaded: bool,
//     custom_count: u16,
//     dll_id: u16,
//     custom_ids: Vec<i32>,
//     custom_handles: Vec<i32>,
//     // TODO: Figure out 'void * instance'
// }


// #[derive(Debug, Clone)]
// struct OsirusMission {
//     is_loaded: bool,
//     dll_id: u16
// }

// struct ModuleFunctions {
//     // TODO:
// }

// struct OsirusModuleInitializeInfo {
//     // functions: ModuleFunctions,
//     // TODO: string table (vector)
//     module_id: i32,

//     /// if this is set to true after initialization
//     /// then the module will not unload if it's reference
//     /// count is 0....only when the level ends.
//     /// this is for Game modules ONLY.
//     is_static: bool,

//     // TODO: script identifer

//     game_checksum: u32,
// }

// impl OsirusModuleInitializeInfo {
//     // TODO: see how this is generally setup
//     fn create_new(id: i32) -> Self {
//         Self {
//             // TODO: functions pairing
//             module_id: id,
//             game_checksum: 0,
//             is_static: false
//         }
//     }
// }

// macro_rules! have_custom_only {
//     ($type:expr) => {
//         $type == ObjectType::ObjCamera
//     };
// }

// macro_rules! can_be_assigned_script {
//     ($obj:expr) => {
//         match $obj {
//             Some(ref o) => matches!(o.obj_type,
//                 ObjectType::ObjRobot | ObjectType::ObjBuilding | ObjectType::ObjPowerup |
//                 ObjectType::ObjClutter | ObjectType::ObjDoor | ObjectType::ObjCamera | ObjectType::ObjDummy
//             ),
//             None => false
//         }
//     };
// }

// macro_rules! can_have_any_script {
//     ($obj:expr) => {
//         can_be_assigned_script!($obj) || matches!($obj,
//             Some(o) if o.obj_type == ObjectType::ObjDebris ||
//             o.obj_type == ObjectType::ObjGhost
//         )
//     };
// }




// // TODO:
// /*
// void Osiris_RestoreOMMS(CFILE *file);
// void Osiris_SaveOMMS(CFILE *file);
// uint Osiris_CreateGameChecksum(void);
// bool Osiris_IsEventEnabled(int event);
// void Cinematic_StartCannedScript(tCannedCinematicInfo *info);
// void Osiris_DumpLoadedObjects(char *file);
// void Osiris_ForceUnloadModules(void);
// void Osiris_DumpLoadedObjects(char *file)
// void Osiris_UnloadModule(int module_id)
// int Osiris_FindLoadedModule(char *module_name)
// void Osiris_UnloadLevelModule(void)
// int _get_full_path_to_module(char *module_name, char *fullpath, char *basename) {
// int Osiris_LoadLevelModule(char *module_name) {
// int Osiris_LoadGameModule(char *module_name) {
// bool Osiris_BindScriptsToObject(object *obj) {
// void Osiris_DetachScriptsFromObject(object *obj) {
// bool Osiris_CallLevelEvent(int event, tOSIRISEventInfo *data) {
// bool Osiris_CallTriggerEvent(int trignum, int event, tOSIRISEventInfo *ei) {
// void Osiris_EnableEvents(ubyte mask)
// void Osiris_DisableEvents(ubyte mask)
// void Osiris_DisableCreateEvents(void)
// void Osiris_EnableCreateEvents(void)
// bool Osiris_IsEventEnabled(int event)
// bool Osiris_CallEvent(object *obj, int event, tOSIRISEventInfo *data)
// void Osiris_ProcessTimers(void) 
// int Osiris_CreateTimer(tOSIRISTIMER *ot) 
// void Osiris_CancelTimer(int handle)
// int Osiris_GetTimerHandle(int id)
// void Osiris_CancelTimerID(int id)
// ubyte Osiris_TimerExists(int handle)
// float Osiris_TimerTimeRemaining(int handle)
// void Osiris_SaveSystemState(CFILE *file)
// bool Osiris_RestoreSystemState(CFILE *file) 
// void Osiris_InitMemoryManager(void)
// void Osiris_CloseMemoryManager(void)
// void *Osiris_AllocateMemory(tOSIRISMEMCHUNK *mc)
// void Osiris_FreeMemory(void *mem_ptr)
// bool compareid(tOSIRISSCRIPTID *sid, tOSIRISSCRIPTID *oid)
// void Osiris_FreeMemoryForScript(tOSIRISSCRIPTID *sid)
// void Osiris_SaveMemoryChunks(CFILE *file)
// void Osiris_RestoreMemoryChunks(CFILE *file)
// void _extractscript(char *script, char *tempfilename)
// void _clearextractedall(void)
// int Osiris_ExtractScriptsFromHog(int library_handle, bool is_mission_hog)
// void Osiris_ClearExtractedScripts(bool mission_only)
// tOMMSHashNode *Osiris_OMMS_FindHashNode(char *script_name, bool autocreate);
// tOMMSHashNode *Osiris_OMMS_DeleteHashNode(tOMMSHashNode *node);
// */