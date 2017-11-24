use std::cell::UnsafeCell;
use std::ops::Deref;

pub struct Lazy<T> {
    val: UnsafeCell<Option<T>>,
}

impl<T> Lazy<T> {
    pub fn new() -> Lazy<T> {
        Lazy { val: UnsafeCell::new(None) }
    }

    pub fn init(&self, new_val: T) {
        let ptr = self.val.get();

        unsafe {
            if (*ptr).is_some() {
                panic!("already initialized");
            }
            *ptr = Some(new_val);
        }
    }

    pub fn get(&self) -> &T {
        unsafe { (*self.val.get()).as_ref().expect("yet initialized") }
    }
}

impl<T> Deref for Lazy<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.get()
    }
}
