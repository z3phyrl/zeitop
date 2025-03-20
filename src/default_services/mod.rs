pub mod page_assets;
pub mod mpd;
pub mod page;
pub mod sysinfo;

use anyhow::Result;

pub trait DefaultService {
    async fn run() -> Result<()>;
}
