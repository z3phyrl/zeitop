pub mod page;
pub mod sysinfo;
pub mod mpd;

use anyhow::Result;

pub trait DefaultService {
    async fn run() -> Result<()>;
}
