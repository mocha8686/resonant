use std::{fs::File, io::BufReader, path::PathBuf};

use iced::{Element, widget::{button, column, text}};
use rfd::FileDialog;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Message {
    Select,
    Play,
}

#[derive(Default)]
pub struct Track {
    path: Option<PathBuf>,
    sink: Option<MixerDeviceSink>,
    player: Option<Player>,
}

impl Track {
    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Select => {
                let file = FileDialog::new()
                    .add_filter("audio", &["mp3", "ogg", "wav", "m4a"])
                    .pick_file();
                self.path = file;
            }
            Message::Play => {
                let Some(audio_path) = &self.path else {
                    return;
                };

                let sink = DeviceSinkBuilder::open_default_sink()
                    .expect("should be able to open default sink");
                let file = BufReader::new(
                    File::open(audio_path).expect("should be able to open audio file"),
                );
                let player =
                    rodio::play(sink.mixer(), file).expect("should be able to play sound");

                self.sink.replace(sink);
                self.player.replace(player);
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let audio_path = self
            .path
            .clone()
            .map_or("No file selected.".into(), |p| p.to_string_lossy().to_string());

        column![
            text(audio_path),
            button("Select file").on_press(Message::Select),
            button("Play").on_press(Message::Play),
        ].into()
    }
}
