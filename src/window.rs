use iced::{Element, Subscription, Task, window};

use crate::scene::{self, Scene};

pub struct Window {
    id: window::Id,
    scene: Scene,
    modified: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Scene(scene::Message),
    CloseRequested,
}

#[derive(Debug)]
pub enum Action {
    Run(Task<Message>),
    AddTrack,
    Save,
    Load,
    Close,
}

impl Window {
    pub fn open(scene: Scene, settings: window::Settings) -> (Self, Task<window::Id>) {
        let (id, task) = window::open(window::Settings {
            exit_on_close_request: false,
            ..settings
        });

        log::info!(
            "Opening new window with ID {} for scene {}.",
            id,
            scene.name()
        );

        let window = Self {
            id,
            scene,
            modified: false,
        };

        (window, task)
    }

    pub fn view(&self) -> Element<'_, Message> {
        self.scene.view().map(Message::Scene)
    }

    pub fn update(&mut self, msg: Message) -> Option<Action> {
        match msg {
            Message::Scene(msg) => {
                if let Some(action) = self.scene.update(msg) {
                    let action = if let scene::Action::Modifying(action) = action {
                        if !self.modified {
                            log::debug!("Setting modified flag for window {}.", self.id);
                            self.modified = true;
                        }
                        *action
                    } else {
                        action
                    };

                    match action {
                        scene::Action::Run(task) => Some(Action::Run(task.map(Message::Scene))),
                        scene::Action::AddTrack => Some(Action::AddTrack),
                        scene::Action::Save => Some(Action::Save),
                        scene::Action::Load => Some(Action::Load),
                        scene::Action::Modifying(_) => unreachable!(),
                    }
                } else {
                    None
                }
            }
            Message::CloseRequested => {
                log::info!("Close requested for window {}.", self.id);
                if !self.modified {
                    return Some(Action::Close);
                }

                let res = rfd::MessageDialog::new()
                    .set_buttons(rfd::MessageButtons::YesNoCancel)
                    .set_level(rfd::MessageLevel::Info)
                    .set_title("Unsaved changes")
                    .set_description("Would you like to save unsaved changes before exiting?")
                    .show();

                match res {
                    rfd::MessageDialogResult::Yes => todo!(),
                    rfd::MessageDialogResult::No => Some(Action::Close),
                    rfd::MessageDialogResult::Cancel => None,
                    _ => unreachable!(),
                }
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        self.scene.subscription().map(Message::Scene)
    }

    pub fn id(&self) -> window::Id {
        self.id
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }
}
