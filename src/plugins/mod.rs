use std::io;

use missdemeanor::CRequest;

use crate::config::Trigger;

mod err;
pub use self::err::*;

mod cabi;
pub use self::cabi::*;

mod interpreted;
pub use self::interpreted::*;

pub trait NewPlugin: Sized {
    fn new(trigger: Trigger) -> Result<Self, io::Error>;
}

pub trait Plugin {
    fn run_trigger(&self, request: CRequest) -> Result<(), PluginError>;
}
