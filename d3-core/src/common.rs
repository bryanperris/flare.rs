use core::ops::{Sub, SubAssign};
use std::{cell::RefCell, rc::{Rc, Weak}};

// TODO: Should we create our own type
// for sharing mutable referecens to game objects?

pub type SharedMutRef<T> = Rc<RefCell<T>>;
pub type WeakSharedMutRef<T> = Weak<RefCell<T>>;
pub type SharedRef<T> = Rc<T>;

pub fn new_shared_mut_ref<T>(value: T) -> SharedMutRef<T> {
    Rc::new(RefCell::new(value))
}

pub fn unsigned_safe_sub<T>(a: T, b: T) -> T
where
    T: PartialOrd + Sub<Output = T> + From<u8> + SubAssign,
{
    if a >= b {
        a - b
    } else {
        T::from(0u8)
    }
}

pub trait SystemClock: std::fmt::Debug + Send + Sync {
    fn get_ticks(&self) -> u128;
}

#[derive(Debug)]
pub struct StdSystemClock;

impl SystemClock for StdSystemClock {
    fn get_ticks(&self) -> u128 {
        let duration_since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");

        duration_since_epoch.as_micros()
    }
}