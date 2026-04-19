use std::ffi::OsString;

use kira::{AudioManager, AudioManagerSettings};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::{
    FileStreamingSoundData, Handle, Track, looping::Loop, play_pause::PlayPause, progress::Progress,
};
use crate::{PROJECT_DIRS, Vector2};

#[derive(Serialize, Deserialize)]
pub struct TrackData {
    id: Ulid,
    name: String,
    extension: OsString,
    audio_data: Vec<u8>,
    position: Vector2,
    radius: f32,
    duration: f32,
}

impl TryFrom<&Track> for TrackData {
    type Error = anyhow::Error;

    fn try_from(track: &Track) -> Result<Self, Self::Error> {
        let audio_data = std::fs::read(&track.path)?;

        let extension = track.path.extension().unwrap().to_os_string();

        Ok(Self {
            id: track.id,
            name: track.name.clone(),
            extension,
            audio_data,
            position: track.position,
            radius: track.radius,
            duration: track.progress.duration(),
        })
    }
}

impl TryFrom<TrackData> for Track {
    type Error = anyhow::Error;

    fn try_from(track_data: TrackData) -> Result<Self, Self::Error> {
        let manager = AudioManager::new(AudioManagerSettings::default())?;

        let cache_dir = PROJECT_DIRS.cache_dir();

        std::fs::create_dir_all(cache_dir)?;
        let cache_dest = cache_dir
            .join(track_data.id.to_string())
            .with_extension(track_data.extension);
        std::fs::write(&cache_dest, &track_data.audio_data)?;

        let data = FileStreamingSoundData::from_file(&cache_dest)?;
        let duration = data.unsliced_duration().as_secs_f32();

        Ok(Track {
            id: track_data.id,
            name: track_data.name,
            path: cache_dest,
            position: track_data.position,
            radius: track_data.radius,
            manager,
            handle: Handle::Uninitialized(Some(data)),
            play_pause: PlayPause::new(),
            progress: Progress::new(duration),
            looping: Loop::new(),
        })
    }
}
