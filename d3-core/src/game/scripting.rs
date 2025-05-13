use super::prelude::*;

#[derive(Debug, Copy, Clone)]
pub enum EventType {
    /// Called every frame.
    Interval,
    /// Called every frame for AI information.
    AiFrame,
    /// Called when an object is damaged.
    Damaged,
    /// Called when an object collides with something.
    Collide,
    /// Called when an object is created.
    Created,
    /// Called when an object is destroyed.
    Destroy,
    /// Called when a timer event is signaled.
    Timer,
    /// Called when an item is selected for use from the inventory.
    Use,
    /// Called when an AI gets notified.
    AiNotify,
    /// Called to initialize SCRIPT AI stuff.
    AiInit,
    /// Called when an object changes room.
    ChangeSeg,
    /// Called when the script should save its state.
    SaveState,
    /// Called when the script should restore its state.
    RestoreState,
    /// Called when the script should restore a pointer to the special auto-save memory it allocated.
    MemRestore,
    /// Called when a timer is canceled (either by function call or from its object detonator).
    TimerCancel,
    /// Child event of AiNotify for when an object is killed.
    AinObjKilled,
    /// Child event of AiNotify for when an AI sees a player.
    AinSeePlayer,
    /// Child event of AiNotify for when an AI hits an object.
    AinWhitObject,
    /// Child event of AiNotify for when a goal is completed.
    AinGoalComplete,
    /// Child event of AiNotify for when a goal fails.
    AinGoalFail,
    /// Child event of AiNotify for when a melee hit occurs.
    AinMeleeHit,
    /// Child event of AiNotify for when a melee attack frame occurs.
    AinMeleeAttackFrame,
    /// Child event of AiNotify for when a movie starts.
    AinMovieStart,
    /// Child event of AiNotify for when a movie ends.
    AinMovieEnd,
    /// Level event that a matcen created an object.
    MatcenCreate,
    /// Event for when a door is opening.
    DoorActivate,
    /// Event for when a door is closing.
    DoorClose,
    /// Event for when a child object dies.
    ChildDied,
    /// Event for when a level goal is completed.
    LevelGoalComplete,
    /// Event for when all level goals are completed.
    AllLevelGoalsComplete,
    /// Event for when a level goal item is completed.
    LevelGoalItemComplete,
    /// Event for when an IGC focusing on the player starts.
    PlayerMovieStart,
    /// Event for when an IGC focusing on the player ends.
    PlayerMovieEnd,
    /// Event for when a player respawns.
    PlayerRespawn,
    /// Event for when a player dies.
    PlayerDies,
}

#[derive(Debug, Copy, Clone)]
pub struct EventInfo {

}


pub trait NewOsirusScriptSystem {
    fn signal_event(&mut self, event_type: EventType, info: Option<EventInfo>, object: SharedMutRef<Object>) {

    }
}