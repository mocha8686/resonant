use std::{fs::File, io::BufReader, path::PathBuf};

use iced::{
    Element,
    Length::Fill,
    Theme,
    widget::{
        button, column, container, svg,
        svg::{Handle, Style},
        text,
    },
};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Message {
    Play,
    Pause,
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
                if let Some(player) = &self.player {
                    player.play();
                } else {
                    let sink = DeviceSinkBuilder::open_default_sink()
                        .expect("should be able to open default sink");
                    let file = BufReader::new(
                        File::open(&self.path).expect("should be able to open audio file"),
                    );
                    let player =
                        rodio::play(sink.mixer(), file).expect("should be able to play sound");

                    self.sink.replace(sink);
                    self.player.replace(player);
                }
            }
            Message::Pause => {
                let Some(player) = &self.player else {
                    return;
                };
                player.pause();
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let filename = self
            .path
            .with_extension("")
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown filename")
            .to_string();

        column![
            text(filename),
            self.play_pause(),
        ].into()
    }

    fn play_pause(&self) -> Element<'_, Message> {
        let icon = if self.is_paused() {
            include_bytes!("icons/play.svg").as_slice()
        } else {
            include_bytes!("icons/pause.svg").as_slice()
        };
        let handle = Handle::from_memory(icon);
        let svg = svg(handle)
            .style(|theme: &Theme, _| Style {
                color: Some(theme.palette().text),
            })
            .width(16)
            .height(16);

        button(svg)
            .on_press(if self.is_paused() {
                Message::Play
            } else {
                Message::Pause
            })
            .into()
    }

    fn is_paused(&self) -> bool {
        self.player.as_ref().is_none_or(rodio::Player::is_paused)
    }
}
