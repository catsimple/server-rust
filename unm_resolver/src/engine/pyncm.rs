//! UNM Resolver [Engine]: PyNCM
//!
//! It can fetch audio from the unofficial
//! Netease Cloud Music API.

use rayon::prelude::*;
use std::str::FromStr;

use http::Method;
use serde::Deserialize;

use crate::request::request;

use super::{Context, Engine, Song};

#[derive(Deserialize)]
struct PyNCMResponse {
    /// The status code of this response.
    pub code: i32,
    pub data: Vec<PyNCMResponseEntry>,
}

#[derive(Deserialize)]
struct PyNCMResponseEntry {
    /// The NCM ID of this song.
    pub id: String,
    /// The URL of this song.
    pub url: Option<String>,
}

/// The `pyncm` engine that can fetch audio from
/// the unofficial Netease Cloud Music API.
pub struct PyNCMEngine;

#[async_trait::async_trait]
impl Engine for PyNCMEngine {
    async fn check<'a>(&self, info: &'a Song, ctx: &'a Context) -> anyhow::Result<Option<String>> {
        track(info, ctx.enable_flac, ctx).await
    }
}

/// Fetch the song info in [`PyNCMResponse`].
async fn fetch_song_info(
    id: &str,
    enable_flac: bool,
    ctx: &Context<'_>,
) -> anyhow::Result<PyNCMResponse> {
    let url_str = format!(
        "http://mos9527.tooo.top/ncm/pyncm/track/GetTrackAudio?song_ids={id}&bitrate={bitrate}",
        id = id,
        bitrate = if enable_flac { 999000 } else { 320000 }
    );
    let url = url::Url::from_str(&url_str)?;

    let response = request(Method::GET, &url, None, None, ctx.proxy.cloned()).await?;
    Ok(response.json::<PyNCMResponse>().await?)
}

/// Find the matched song from an array of [`PyNCMResponseEntry`].
fn find_match(data: &[PyNCMResponseEntry], song_id: &str) -> anyhow::Result<Option<String>> {
    data.par_iter()
        .find_any(|entry| {
            // Test if the ID of this entry matched what we want to fetch,
            // and there is content in its URL.
            entry.id == song_id && entry.url.is_some()
        })
        .map(|v| v.url.clone())
        .ok_or(anyhow::anyhow!("no matched song"))
}

/// Track the matched song.
async fn track(
    song: &Song,
    enable_flac: bool,
    ctx: &Context<'_>,
) -> anyhow::Result<Option<String>> {
    let response = fetch_song_info(&song.id, enable_flac, ctx).await?;

    if response.code == 200 {
        Ok(find_match(&response.data, &song.id)?)
    } else {
        Err(anyhow::anyhow!(
            "failed to request. code: {}",
            response.code
        ))
    }
}
