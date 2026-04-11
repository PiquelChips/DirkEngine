pub(crate) mod format;

pub mod console;
pub mod file;

// The storage layer is only compiled (and exposed) for editor builds.
// In game builds, the entire module and its Arc<LogStore> overhead are absent.
#[cfg(editor)]
pub mod storage;
