pub mod add_tab {
    tonic::include_proto!("add_tab");
}

pub mod new_window {
    tonic::include_proto!("new_window");
}

pub mod app;
pub mod grpc;
pub mod utils;
pub mod service;
