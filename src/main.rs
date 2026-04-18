use iced::{Element, Subscription, Task};
use resonant::scene::{self, Scene};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Message {
    Scene(scene::Message),
}

#[derive(Default)]
struct State {
    scene: Scene,
}

impl State {
    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Scene(msg) => self.scene.update(msg).map(Message::Scene),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        self.scene.view().map(Message::Scene)
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
