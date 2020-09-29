use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc, watch};
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

use oxydoro::oxydoro_server::{Oxydoro, OxydoroServer};
use oxydoro::{
    CreateTaskReply, CreateTaskRequest, GetAllTasksReply, GetAllTasksRequest,
    SubscribeToTaskUpdatesReply, SubscribeToTaskUpdatesRequest, Task, TaskId,
};

pub mod oxydoro {
    tonic::include_proto!("oxydoro");
}

trait TaskIdWrapper {
    fn new() -> Self;
}

impl TaskIdWrapper for TaskId {
    fn new() -> Self {
        let uuid = Uuid::new_v4();
        TaskId {
            uuid: uuid
                .to_simple()
                .encode_upper(&mut Uuid::encode_buffer())
                .to_owned(),
        }
    }
}

struct OxydoroStore {
    tasks: Arc<RwLock<Vec<Task>>>,
    awaiter: watch::Receiver<()>,
    waker: watch::Sender<()>,
}

impl Default for OxydoroStore {
    fn default() -> Self {
        let (tx, rx) = watch::channel(());
        OxydoroStore {
            tasks: Arc::new(RwLock::new(vec![])),
            awaiter: rx,
            waker: tx,
        }
    }
}

#[tonic::async_trait]
impl Oxydoro for OxydoroStore {
    async fn create_task(
        &self,
        request: Request<CreateTaskRequest>,
    ) -> Result<Response<CreateTaskReply>, Status> {
        let request_inner = request.into_inner();
        let new_task = Task {
            title: request_inner.title,
            id: Some(TaskId::new()),
            done: false,
        };
        let mut tasks = self
            .tasks
            .write()
            .map_err(|_| Status::internal("Failed to unlock store"))?;
        tasks.push(new_task.clone());
        self.waker.broadcast(()).unwrap();
        Ok(Response::new(CreateTaskReply {
            task: Some(new_task),
        }))
    }

    async fn get_all_tasks(
        &self,
        _: Request<GetAllTasksRequest>,
    ) -> Result<Response<GetAllTasksReply>, Status> {
        let tasks = self
            .tasks
            .read()
            .map_err(|_| Status::internal("Failed to unlock store"))?
            .clone();
        let reply = GetAllTasksReply { tasks };
        Ok(Response::new(reply))
    }

    type SubscribeToTaskUpdatesStream = mpsc::Receiver<Result<SubscribeToTaskUpdatesReply, Status>>;

    async fn subscribe_to_task_updates(
        &self,
        req: Request<SubscribeToTaskUpdatesRequest>,
    ) -> Result<Response<Self::SubscribeToTaskUpdatesStream>, Status> {
        let (mut tx, rx) = mpsc::channel(16);

        let tasks = Arc::clone(&self.tasks);
        let mut awaiter = self.awaiter.clone();
        tokio::spawn(async move {
            loop {
                awaiter.recv().await;
                let tasks = tasks.read().unwrap().clone();
                let res = tx.send(Ok(SubscribeToTaskUpdatesReply { tasks })).await;
                if res.is_err() {
                    println!("Client disconnected {:?}", req.remote_addr());
                    return;
                }
            }
        });

        Ok(Response::new(rx))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = "127.0.0.1:5001".parse()?;
    let oxydoro_service = OxydoroStore::default();

    println!("Oxydoro service at {}", address);

    Server::builder()
        .add_service(OxydoroServer::new(oxydoro_service))
        .serve(address)
        .await?;

    Ok(())
}
