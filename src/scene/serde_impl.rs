use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Cursor, Read},
    sync::Arc,
};

use anyhow::{Result, anyhow};
use ordermap::OrderMap;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::Scene;
use crate::{
    audio_cache::{AudioCache, AudioData, FileHash},
    soundscape::Soundscape,
    track::{Track, TrackData},
};

#[derive(Serialize, Deserialize)]
pub struct SceneData {
    #[serde(skip)]
    name: String,
    tracks: OrderMap<Ulid, TrackData>,
    audio_saves: Vec<Vec<u8>>,
    soundscape: Soundscape,
}

impl SceneData {
    #[must_use]
    pub fn with_name(self, name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..self
        }
    }
}

impl SceneData {
    pub fn new(scene: &Scene, audio_cache: &AudioCache) -> Result<Self> {
        let hashes: Vec<_> = scene.tracks.iter().map(|(_, t)| t.hash()).collect();
        let files = audio_cache
            .subset(&hashes)
            .into_values()
            .map(|data| data.load_file())
            .try_collect::<Vec<BufReader<File>>>()?;

        let audio_data = files
            .into_iter()
            .map(|mut f| {
                let mut data = Vec::new();
                f.read_to_end(&mut data)?;
                Ok::<Vec<u8>, anyhow::Error>(data)
            })
            .try_collect::<Vec<Vec<u8>>>()?;

        let tracks = scene
            .tracks
            .iter()
            .map(|(id, track)| (*id, TrackData::new(track)))
            .collect();

        Ok(Self {
            name: scene.name.clone(),
            tracks,
            audio_saves: audio_data,
            soundscape: scene.soundscape.clone(),
        })
    }
}

impl Scene {
    pub fn from_data(scene_data: SceneData, audio_cache: &mut AudioCache) -> Result<Self> {
        let audio_datas = scene_data
            .audio_saves
            .into_iter()
            .map(|audio_data| {
                let mut cursor = Cursor::new(audio_data);
                let audio_data = audio_cache.get_or_register(&mut cursor)?;
                let hash = audio_data.hash();
                Ok::<(FileHash, Arc<AudioData>), anyhow::Error>((hash, audio_data))
            })
            .try_collect::<HashMap<FileHash, Arc<AudioData>>>()?;

        // Track::from_data(track_data, audio_data);
        let tracks = scene_data
            .tracks
            .into_iter()
            .map(|(id, track_data)| {
                let hash = track_data.hash();
                let audio_data = audio_datas
                    .get(&hash)
                    .ok_or_else(|| {
                        anyhow!("Failed to load audio for track while loading save file.")
                    })?
                    .clone();
                let track = Track::from_data(track_data, audio_data)?;
                Ok::<(Ulid, Track), anyhow::Error>((id, track))
            })
            .try_collect()?;

        Ok(Self {
            name: scene_data.name.clone(),
            tracks,
            soundscape: scene_data.soundscape,
        })
    }
}
