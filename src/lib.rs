mod node;
use node::Node;

mod list;
pub use list::LinkedList;

#[macro_export]
macro_rules! try_lock {
    ($mutex:expr) => {
        match $mutex.try_lock() {
            Ok(lock) => lock,
            Err(err) => match err {
                std::sync::TryLockError::WouldBlock => continue,
                _ => panic!("{err}"),
            },
        }
    };
}
