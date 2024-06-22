use tauri::{AppHandle, Manager};
use tonic::{Request, Response, Status};

use crate::{
    new_window::{
        new_window_opened_client::NewWindowOpenedClient, new_window_opened_server::NewWindowOpened,
        OpenNewWindowRequest, OpenNewWindowResponse,
    },
    service::app_state::{add_viewer_state, AppState},
};

pub struct Opener {
    app: AppHandle,
}

impl Opener {
    pub fn new(app: AppHandle) -> Self {
        Opener { app }
    }
}

#[tonic::async_trait]
impl NewWindowOpened for Opener {
    async fn open_new_window(
        &self,
        _: Request<OpenNewWindowRequest>,
    ) -> Result<Response<OpenNewWindowResponse>, Status> {
        let state = self.app.state::<AppState>();
        let label = add_viewer_state(&state)
            .await
            .map_err(|_| Status::failed_precondition("system unavailable"))?;
        tauri::WindowBuilder::new(&self.app, label, tauri::WindowUrl::App("index.html".into()))
            .title("Simple Image Viewer")
            .maximized(true)
            .build()
            .map_err(|_| Status::failed_precondition("system unavailable"))?;
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
