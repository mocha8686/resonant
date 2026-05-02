use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::PathBuf,
    sync::Arc,
};

use anyhow::Result;
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use log::debug;
use ulid::Ulid;

use crate::PROJECT_DIRS;

pub type FileHash = blake3::Hash;

pub type FileStreamingSoundData = StreamingSoundData<kira::sound::FromFileError>;
pub type FileStreamingSoundHandle = StreamingSoundHandle<kira::sound::FromFileError>;

pub type AudioHandles = HashMap<blake3::Hash, Arc<AudioData>>;

#[derive(Debug, Clone, Default)]
pub struct AudioCache {
    handles: AudioHandles,
}

impl AudioCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_register<T: Read + Seek>(&mut self, data: &mut T) -> Result<Arc<AudioData>> {
        let hash = file_hash(data)?;
        debug!("Requested track with hash {hash}.");

        if let Some(data) = self.handles.get(&hash) {
            debug!("Cache hit for hash {hash} with ID {}.", data.id);
            Ok(data.clone())
        } else {
            debug!("Cache miss for hash {hash}, creating new entry.");
            data.seek(SeekFrom::Start(0))?;

            let id = Ulid::new();
            debug!("New entry has ID {id}.");

            let cache_path = cache_path(id);

            let mut file = File::create_buffered(cache_path)?;
            std::io::copy(data, &mut file)?;

            let data = Arc::new(AudioData { id, hash });

            let data = self.handles.entry(hash).insert_entry(data);
            Ok(data.get().clone())
        }
    }

    pub fn subset(&self, hashes: &[FileHash]) -> AudioHandles {
        self.handles
            .iter()
            .filter_map(|(hash, track)| {
                if hashes.contains(hash) {
                    Some((*hash, track.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AudioData {
    id: Ulid,
    hash: FileHash,
}

impl AudioData {
    pub fn cache_path(&self) -> PathBuf {
        cache_path(self.id)
    }

    pub fn load(&self) -> Result<FileStreamingSoundData> {
        debug!("Loading sound data for audio ID {}.", self.id.to_string());
        let data = FileStreamingSoundData::from_file(self.cache_path())?;
        Ok(data)
    }

    pub fn load_file(&self) -> Result<BufReader<File>> {
        Ok(File::open_buffered(self.cache_path())?)
    }

    pub fn hash(&self) -> FileHash {
        self.hash
    }
}

pub fn cache_path(id: Ulid) -> PathBuf {
    PROJECT_DIRS.cache_dir().join(id.to_string())
}

impl Drop for AudioData {
    fn drop(&mut self) {
        std::fs::remove_file(self.cache_path()).ok();
    }
}

fn file_hash(f: &mut impl Read) -> Result<FileHash> {
    let mut hasher = blake3::Hasher::new();
    std::io::copy(f, &mut hasher)?;
    Ok(hasher.finalize())
}
