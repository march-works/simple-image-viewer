use std::ops::AddAssign;

use tauri::{async_runtime::Mutex, AppHandle};
use tonic::{Request, Response, Status};

use crate::new_window::{
    new_window_opened_client::NewWindowOpenedClient, new_window_opened_server::NewWindowOpened,
    OpenNewWindowRequest, OpenNewWindowResponse,
};

pub struct Opener {
    app: AppHandle,
    count: Mutex<i32>,
}

impl Opener {
    pub fn new(app: AppHandle) -> Self {
        Opener {
            app,
            count: Mutex::new(0),
        }
    }
}

#[tonic::async_trait]
impl NewWindowOpened for Opener {
    async fn open_new_window(
        &self,
        _: Request<OpenNewWindowRequest>,
    ) -> Result<Response<OpenNewWindowResponse>, Status> {
        tauri::WindowBuilder::new(
            &self.app,
            format!("label-{}", self.count.lock().await),
            tauri::WindowUrl::App("index.html".into()),
        )
        .focus()
        .title("Simple Image Viewer")
        .maximized(true)
        .build()
        .map_err(|_| Status::failed_precondition("system unavailable"))?;
        self.count.lock().await.add_assign(1);
        let response = OpenNewWindowResponse { result: 3 };
        Ok(Response::new(response))
    }
}

pub async fn open(handler: AppHandle) -> Result<(), anyhow::Error> {
    let mut client = NewWindowOpenedClient::connect("http://[::1]:50052").await?;
    let request = tonic::Request::new(OpenNewWindowRequest {});
    client.open_new_window(request).await?;
    handler.exit(0);
    Ok(())
}
