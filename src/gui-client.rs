mod style;

use iced::{
    executor, scrollable, text_input, Align, Application, Color, Column, Command, Container,
    Element, Length, Row, Scrollable, Settings, Text, TextInput,
};

use style::Theme;

use oxydoro::oxydoro_client::OxydoroClient;
use oxydoro::{CreateTaskRequest, GetAllTasksRequest, Task};
use tonic::transport::Channel;

pub mod oxydoro {
    tonic::include_proto!("oxydoro");
}

struct OxydoroUI {
    state: OxydoroState,
    theme: Theme,
}

struct LoadedViewState {
    rpc_connector: OxydoroClient<Channel>,
    tasks: Vec<Task>,
    scroll_state: scrollable::State,
    text_input_state: text_input::State,
    new_task_name: String,
}

impl LoadedViewState {
    fn new(rpc_connector: OxydoroClient<Channel>, task_list: Vec<Task>) -> LoadedViewState {
        LoadedViewState {
            rpc_connector,
            tasks: task_list,
            scroll_state: scrollable::State::new(),
            text_input_state: text_input::State::focused(),
            new_task_name: String::new(),
        }
    }
}

enum OxydoroState {
    Connecting,
    Connected {
        rpc_connector: OxydoroClient<Channel>,
    },
    LoadedView(LoadedViewState),
    Error,
}

impl OxydoroUI {
    fn new(theme: Theme) -> OxydoroUI {
        OxydoroUI {
            state: OxydoroState::Connecting,
            theme,
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

#[derive(Debug, Clone)]
enum Message {
    Connected(Result<OxydoroClient<Channel>, OxydoroError>),
    Received(Result<Vec<Task>, OxydoroError>),
    InputChanged(String),
    SubmitNewTask,
    TaskCreated,
}

impl Application for OxydoroUI {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = Theme;

    fn new(flags: Theme) -> (OxydoroUI, Command<Message>) {
        (
            OxydoroUI::new(flags),
            Command::perform(
                create_rpc_connection(String::from("http://127.0.0.1:5001")),
                Message::Connected,
            ),
        )
    }

    fn title(&self) -> String {
        match &self.state {
            OxydoroState::Connecting => String::from("Oxydoro - Connecting..."),
            OxydoroState::Connected { rpc_connector: _ } => String::from("Oxydoro"),
            OxydoroState::Error => String::from("Oxydoro - Error connecting"),
            OxydoroState::LoadedView(_) => String::from("Oxydoro"),
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Connected(Ok(mut rpc_client)) => {
                self.state = OxydoroState::Connected {
                    rpc_connector: rpc_client.clone(),
                };
                let request = tonic::Request::new(GetAllTasksRequest {});
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
                if let OxydoroState::Connected { rpc_connector } = &self.state {
                    self.state = OxydoroState::LoadedView(LoadedViewState::new(
                        rpc_connector.clone(),
                        task_list,
                    ));
                }
                Command::none()
            }
            Message::Received(Err(_)) => {
                self.state = OxydoroState::Error;
                Command::none()
            }
            Message::InputChanged(new_input_value) => {
                if let OxydoroState::LoadedView(loaded_view_state) = &mut self.state {
                    loaded_view_state.new_task_name = new_input_value;
                }
                Command::none()
            }
            Message::SubmitNewTask => {
                if let OxydoroState::LoadedView(loaded_view_state) = &mut self.state {
                    let request = tonic::Request::new(CreateTaskRequest {
                        title: loaded_view_state.new_task_name.clone(),
                    });
                    loaded_view_state.new_task_name = String::new();
                    let mut rpc_connector = loaded_view_state.rpc_connector.clone();
                    let future = async move { rpc_connector.create_task(request).await };
                    Command::perform(future, |_| Message::TaskCreated)
                } else {
                    Command::none()
                }
            }
            Message::TaskCreated => Command::none(),
        }
    }

    fn view(&mut self) -> Element<Message> {
        match &mut self.state {
            OxydoroState::Connecting => {
                centered_element(Text::new("Loading tasks").size(40).into(), self.theme)
            }
            OxydoroState::Connected { rpc_connector: _ } => {
                centered_element(Text::new("Connected").size(40).into(), self.theme)
            }
            OxydoroState::Error => centered_element(
                Text::new("Error")
                    .size(40)
                    .color(Color::from_rgb(1., 0., 0.))
                    .into(),
                self.theme,
            ),
            OxydoroState::LoadedView(loaded_view_state) => {
                let input = TextInput::new(
                    &mut loaded_view_state.text_input_state,
                    "Enter search here...",
                    &loaded_view_state.new_task_name,
                    Message::InputChanged,
                )
                .width(Length::Fill)
                .style(self.theme)
                .size(30)
                .padding(10)
                .on_submit(Message::SubmitNewTask);

                let entries = loaded_view_state.tasks.iter().fold(
                    Column::new().padding(20),
                    |column: Column<Message>, task| column.push(task.view()),
                );

                let scrollable_entries = Scrollable::new(&mut loaded_view_state.scroll_state)
                    .push(input)
                    .push(entries)
                    .style(self.theme);

                let content = Column::new()
                    .push(scrollable_entries)
                    .spacing(10)
                    .padding(5);

                Container::new(content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .align_y(iced::Align::Start)
                    .style(self.theme)
                    .into()
            }
        }
    }
}

fn centered_element(content: Element<Message>, theme: Theme) -> Element<Message> {
    Container::new(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .style(theme)
        .into()
}

trait ViewModel {
    fn view(&self) -> Element<Message>;
}

impl ViewModel for Task {
    fn view(&self) -> Element<Message> {
        Row::new()
            .width(Length::Fill)
            .align_items(Align::Center)
            .padding(10)
            .push(Text::new(self.title.clone()).size(20))
            .into()
    }
}

pub fn main() {
    let light = false;
    let theme = if light { Theme::Light } else { Theme::Dark };
    OxydoroUI::run(Settings::with_flags(theme))
}
