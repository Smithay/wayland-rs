use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use wayland_commons::wire::Message;

pub(crate) type QueueBuffer = Arc<Mutex<VecDeque<Message>>>;
