mod buffer;
pub(crate) mod choice;
mod condition;
pub(crate) mod line;

pub use buffer::process_buffer;
pub use choice::{get_fallback_choices, prepare_choices_for_user};
pub(self) use condition::check_condition;
pub use line::Process;
