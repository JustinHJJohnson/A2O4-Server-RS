use crate::ao3::{
    common::DownloadFormat,
    series::Series,
    work::Work,
};
use crate::clients::{
    crosspoint::Crosspoint,
    sftp::Sftp,
};
use crate::config::{Config, Device};

use anyhow::Result;
use enum_dispatch::enum_dispatch;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[enum_dispatch(Client)]
pub enum Clients {
    Crosspoint(Crosspoint),
    Sftp(Sftp),
}

#[enum_dispatch]
pub trait Client {
    async fn upload_work(
        &self,
        work: &Work,
        device: &Device,
        config: &Config,
        download_format: DownloadFormat,
        series: Option<&Series>,
    ) -> Result<()>;

    async fn upload_series(
        &self,
        series: &Series,
        device: &Device,
        config: &Config,
        download_format: DownloadFormat,
    ) -> Result<()>;
}