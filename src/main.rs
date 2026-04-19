#![feature(file_buffered)]
use std::fs::File;

use iced::{Element, Subscription, Task, widget::{button, column, row}};
use resonant::scene::{self, Scene, SceneData};
use rfd::FileDialog;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Message {
    Scene(scene::Message),
    Save,
    Load,
}

#[derive(Default)]
struct State {
    scene: Scene,
}

impl State {
    const FILE_EXTENSION: &'static str = "rst";

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Scene(msg) => Some(self.scene.update(msg).map(Message::Scene)),
            Message::Save => {
                if let Some(mut path) = FileDialog::new()
                    .add_filter("resonant scene", &[Self::FILE_EXTENSION])
                    .set_file_name(format!("scene.{}", Self::FILE_EXTENSION))
                    .save_file()
                {
                    if path.extension().and_then(|s| s.to_str()) != Some(Self::FILE_EXTENSION) {
                        path.add_extension(Self::FILE_EXTENSION);
                    }

                    let mut file =
                        File::create_buffered(path).expect("should be able to open save file");

                    let data = SceneData::try_from(&self.scene)
                        .expect("should be able to convert scene to data");
                    rmp_serde::encode::write(&mut file, &data)
                        .expect("should be able to write to save file");
                }
                None
            }
            Message::Load => {
                if let Some(path) = FileDialog::new()
                    .add_filter("resonant scene", &[Self::FILE_EXTENSION])
                    .pick_file()
                {
                    let file = File::open(path).expect("should be able to open save file");
                    let data: SceneData = rmp_serde::decode::from_read(file)
                        .expect("should be able to read scene data");
                    self.scene = data
                        .try_into()
                        .expect("should be able to load scene from data");
                }
                None
            }
        }
        .unwrap_or_else(Task::none)
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            row![
                button("Save").on_press(Message::Save),
                button("Load").on_press(Message::Load),
            ],
            self.scene.view().map(Message::Scene),
        ].into()
    }

    fn subscription(&self) -> Subscription<Message> {
        self.scene.subscription().map(Message::Scene)
    }
}

fn main() -> iced::Result {
    iced::application(State::default, State::update, State::view)
        .subscription(State::subscription)
        .run()
}
