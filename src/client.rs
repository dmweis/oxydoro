use oxydoro::oxydoro_client::OxydoroClient;
use oxydoro::{CreateTaskRequest, GetAllTasksRequest, SubscribeToTaskUpdatesRequest};

use clap::Clap;

pub mod oxydoro {
    tonic::include_proto!("oxydoro");
}

#[derive(Clap)]
#[clap(author = "David Weis <dweis7@gmail.com>")]
struct Args {
    #[clap(subcommand)]
    command: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    Add(AddParam),
    Get,
    AsyncGet,
}

#[derive(Clap)]
struct AddParam {
    title: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = Args::parse();

    let mut client = OxydoroClient::connect("http://127.0.0.1:5001").await?;

    match args.command {
        SubCommand::Add(params) => {
            let request = tonic::Request::new(CreateTaskRequest {
                title: params.title,
            });

            let response = client.create_task(request).await?;

            let task = response.into_inner().task.unwrap();
            println!("Created new task with ID: {}", task.id.unwrap().uuid);
        }
        SubCommand::Get => {
            let request = tonic::Request::new(GetAllTasksRequest {});
            let response = client.get_all_tasks(request).await?;
            for task in response.into_inner().tasks {
                println!("{}", task.title);
            }
        }
        SubCommand::AsyncGet => {
            let mut tasks_stream = client
                .subscribe_to_task_updates(tonic::Request::new(SubscribeToTaskUpdatesRequest {}))
                .await?
                .into_inner();
            println!("Connected to stream");
            while let Some(task_update) = tasks_stream.message().await? {
                println!("Tasks");
                for task in task_update.tasks {
                    println!("   {}", task.title);
                }
            }
        }
    }

    Ok(())
}
