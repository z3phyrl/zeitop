pub mod lib;
pub mod mpd;
pub mod page;
pub mod sysinfo;
pub mod obs;
pub mod pulse;

use anyhow::Result;

pub trait DefaultService {
    async fn run() -> Result<()>;
}
