use std::io;

use missdemeanor::CRequest;

use config::Trigger;

mod err;
pub use self::err::*;

mod cabi;
pub use self::cabi::*;

mod interpreted;
pub use self::interpreted::*;

pub trait NewPlugin: Sized {
    fn new(Trigger) -> Result<Self, io::Error>;
}

pub trait Plugin {
    fn run_trigger(&self, CRequest) -> Result<(), PluginError>;
}
