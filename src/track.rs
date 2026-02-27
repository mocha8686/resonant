use std::{fs::File, io::BufReader, path::PathBuf};

use iced::{
    Element,
    widget::{button, column, text},
};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Message {
    Play,
}

pub struct Track {
    path: PathBuf,
    sink: Option<MixerDeviceSink>,
    player: Option<Player>,
}

impl Track {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            sink: None,
            player: None,
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Play => {
                let sink = DeviceSinkBuilder::open_default_sink()
                    .expect("should be able to open default sink");
                let file = BufReader::new(
                    File::open(&self.path).expect("should be able to open audio file"),
                );
                let player = rodio::play(sink.mixer(), file).expect("should be able to play sound");

                self.sink.replace(sink);
                self.player.replace(player);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let audio_path = self.path.clone().to_string_lossy().to_string();

        column![text(audio_path), button("Play").on_press(Message::Play),].into()
    }
}
