use iced::{
    executor, scrollable, text_input, Align, Application, Color, Column, Command, Container,
    Element, Font, Length, Row, Scrollable, Settings, Space, Subscription, Text, TextInput,
};

use oxydoro::oxydoro_client::OxydoroClient;
use oxydoro::{CreateTaskRequest, GetAllTasksRequest, Task};
use tonic::transport::Channel;

pub mod oxydoro {
    tonic::include_proto!("oxydoro");
}

struct OxydoroUI {
    state: OxydoroState,
    rpc_connector: Option<OxydoroClient<Channel>>,
}

impl OxydoroUI {
    fn new() -> OxydoroUI {
        OxydoroUI {
            state: OxydoroState::Connecting,
            rpc_connector: None,
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
enum OxydoroError {
    ConnectionError,
}

async fn create_rpc_connection(address: String) -> Result<OxydoroClient<Channel>, OxydoroError> {
    OxydoroClient::connect(address)
        .await
        .map_err(|_| OxydoroError::ConnectionError)
}

enum OxydoroState {
    Connecting,
    Connected,
    Data(Vec<Task>),
    Error,
}

#[derive(Debug, Clone)]
enum Message {
    Connected(Result<OxydoroClient<Channel>, OxydoroError>),
    Received(Result<Vec<Task>, OxydoroError>),
}

impl Application for OxydoroUI {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (OxydoroUI, Command<Message>) {
        (
            OxydoroUI::new(),
            Command::perform(
                create_rpc_connection(String::from("http://127.0.0.1:5001")),
                Message::Connected,
            ),
        )
    }

    fn title(&self) -> String {
        match &self.state {
            OxydoroState::Connecting => String::from("Oxydoro - Connecting..."),
            OxydoroState::Connected => String::from("Oxydoro"),
            OxydoroState::Error => String::from("Oxydoro - Error connecting"),
            OxydoroState::Data(_) => String::from("Oxydoro"),
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Connected(Ok(mut rpc_client)) => {
                self.state = OxydoroState::Connected;
                let request = tonic::Request::new(GetAllTasksRequest {});
                self.rpc_connector = Some(rpc_client.clone());
                let future = async move { rpc_client.get_all_tasks(request).await };
                Command::perform(future, |response| {
                    Message::Received(
                        response
                            .map(|res| res.into_inner().tasks)
                            .map_err(|_| OxydoroError::ConnectionError),
                    )
                })
            }
            Message::Connected(Err(_)) => {
                self.state = OxydoroState::Error;
                Command::none()
            }
            Message::Received(Ok(task_list)) => {
                self.state = OxydoroState::Data(task_list);
                Command::none()
            }
            Message::Received(Err(_)) => {
                self.state = OxydoroState::Error;
                Command::none()
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        match &self.state {
            OxydoroState::Connecting => Container::new(Text::new("Loading tasks").size(40))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into(),
            OxydoroState::Connected => Container::new(Text::new("Connected").size(40))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into(),
            OxydoroState::Error => Container::new(
                Text::new("Error")
                    .size(40)
                    .color(Color::from_rgb(1., 0., 0.)),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into(),
            OxydoroState::Data(task_list) => Container::new(Text::new("Got data").size(40))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into(),
        }
    }
}

pub fn main() {
    OxydoroUI::run(Settings::default())
}
