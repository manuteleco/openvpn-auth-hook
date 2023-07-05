//! Global state managed by the hook.
//!
//! The hook needs to keep track of what `auth-user-pass` files (streams) are
//! open and how many lines have been read from them. In practice we don't
//! expect that there will be more than one stream open at a time, but we still
//! support it.

use libc::FILE;
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::Mutex};

static OPEN_FILES: Lazy<Mutex<HashMap<usize, StreamState>>> = Lazy::new(Default::default);

/// Namespace for functions that manipulate the global state.
pub struct State;

impl State {
    /// Add a new stream to the global state. Tracks that a new stream has been
    /// opened with `fopen`.
    pub fn add(stream: *mut FILE) {
        let previous_value = OPEN_FILES
            .lock()
            .unwrap()
            .insert(stream as usize, StreamState::new());
        if previous_value.is_some() {
            eprintln!("[Hook] WARNING: Stream {:p} was already in the map", stream);
        }
    }

    /// Increment the number of lines read from a stream. Tracks that a new line
    /// has been read from the stream via `fgets`. Returns the number of lines
    /// read so far from the stream.
    pub fn inc_lines(stream: *mut FILE) -> Option<usize> {
        OPEN_FILES
            .lock()
            .unwrap()
            .get_mut(&(stream as usize))
            .map(|state| state.inc())
    }

    /// Remove a stream from the global state. Tracks that a stream has been
    /// closed with `fclose`.
    pub fn remove(stream: *mut FILE) {
        OPEN_FILES.lock().unwrap().remove(&(stream as usize));
    }
}

struct StreamState {
    lines: usize,
}

impl StreamState {
    fn new() -> Self {
        StreamState { lines: 0 }
    }

    fn inc(&mut self) -> usize {
        self.lines = self.lines.saturating_add(1);
        self.lines
    }
}
