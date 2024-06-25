use tauri::{AppHandle, Manager};
use tonic::{Request, Response, Status};

use crate::{
    add_tab::{
        file_path_transfer_client::FilePathTransferClient,
        file_path_transfer_server::FilePathTransfer, FilePathTransferRequest,
        FilePathTransferResponse,
    },
    service::app_state::{add_viewer_tab_state, AppState},
};

pub struct Transferer {
    app: AppHandle,
}

impl Transferer {
    pub fn new(app: AppHandle) -> Self {
        Transferer { app }
    }
}

#[tonic::async_trait]
impl FilePathTransfer for Transferer {
    async fn transfer_file_path(
        &self,
        request: Request<FilePathTransferRequest>,
    ) -> Result<Response<FilePathTransferResponse>, Status> {
        let state = self.app.state::<AppState>();
        let label = state.active.lock().await.label.clone();
        add_viewer_tab_state(&request.get_ref().path.clone(), &label, &state)
            .await
            .map_err(|_| Status::failed_precondition("system unavailable"))?;
        let windows = state.viewers.lock().await;
        let window_state = windows.iter().find(|v| v.label == label).unwrap();
        self.app
            .emit_to(label.as_str(), "viewer-state-changed", window_state)
            .unwrap_or_else(|_| {
                self.app
                    .emit_all("viewer-state-changed", window_state)
                    .unwrap_or(())
            });
        let response = FilePathTransferResponse { result: 5 };
        Ok(Response::new(response))
    }
}

pub async fn transfer(filepath: String, handler: AppHandle) -> Result<(), anyhow::Error> {
    let mut client = FilePathTransferClient::connect("http://[::1]:50052").await?;
    let request = tonic::Request::new(FilePathTransferRequest { path: filepath });
    client.transfer_file_path(request).await?;
    handler.exit(0);
    Ok(())
}
