use lsp_server::Connection;
use ltlsp_server::{run, server_capabilities};
use std::error::Error;

/// Their are four mistakes in this sentence, because it's meaning is not clear to the reader, and we doesn't know who's book it is.
/// Giv a pel mistake.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (connection, io_threads) = Connection::stdio();

    let (id, params) = connection.initialize_start()?;
    let init_params: lsp_types::InitializeParams = serde_json::from_value(params)?;

    connection.initialize_finish(id, serde_json::json!({ "capabilities": server_capabilities() }))?;

    run(connection, init_params).await?;

    io_threads.join()?;
    Ok(())
}
