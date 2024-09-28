use std::ops::ControlFlow;

use async_lsp::{
    client_monitor::ClientProcessMonitorLayer,
    concurrency::ConcurrencyLayer,
    lsp_types::{InitializeParams, InitializeResult, ServerCapabilities},
    panic::CatchUnwindLayer,
    router::Router,
    server::LifecycleLayer,
    tracing::TracingLayer,
    ClientSocket, LanguageServer, ResponseError,
};
use async_std::net::TcpStream;
use futures::future::BoxFuture;
use tower::ServiceBuilder;

pub async fn spawn(stream: &TcpStream) {
    let (server, _) = async_lsp::MainLoop::new_server(|client| {
        let router = Router::from_language_server(ServerState {
            client: client.clone(),
        });

        ServiceBuilder::new()
            .layer(TracingLayer::default())
            .layer(LifecycleLayer::default())
            .layer(CatchUnwindLayer::default())
            .layer(ConcurrencyLayer::default())
            .layer(ClientProcessMonitorLayer::new(client))
            .service(router)
    });

    server.run_buffered(stream, stream).await.unwrap();
}

pub struct ServerState {
    #[expect(unused)]
    client: ClientSocket,
}

impl LanguageServer for ServerState {
    type Error = ResponseError;
    type NotifyResult = ControlFlow<async_lsp::Result<()>>;

    fn initialize(
        &mut self,
        _: InitializeParams,
    ) -> BoxFuture<'static, Result<InitializeResult, Self::Error>> {
        Box::pin(async move {
            Ok(InitializeResult {
                capabilities: ServerCapabilities {
                    ..ServerCapabilities::default()
                },
                server_info: None,
            })
        })
    }
}
