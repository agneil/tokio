/// Spawn a scoped blocking task, which can share references from the future it
/// is being spawned from.
#[macro_export]
macro_rules! spawn_scoped {
    ($func:expr) => { unsafe { crate::task::spawn_scoped($func) }.await }
}