use tauri::AppHandle;
use tonic::transport::Server;

use crate::{
    add_tab::file_path_transfer_server::FilePathTransferServer,
    grpc::{add_tab::Transferer, new_window::Opener},
    new_window::new_window_opened_server::NewWindowOpenedServer,
};

pub async fn run_server(app: AppHandle) -> Result<(), anyhow::Error> {
    let addr = "[::1]:50052".parse()?;
    let res = Server::builder()
        .add_service(FilePathTransferServer::new(Transferer::new(app.clone())))
        .add_service(NewWindowOpenedServer::new(Opener::new(app.clone())))
        .serve(addr)
        .await;
    match res {
        Ok(()) => Ok(()),
        Err(_) => {
            Ok(())
        }
    }
}
