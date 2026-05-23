use lsp_server::Connection;
use ltlsp_server::run;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (connection, io_threads) = Connection::stdio();
    
    let (id, params) = connection.initialize_start()?;
    let init_params: lsp_types::InitializeParams = serde_json::from_value(params)?;
    
    connection.initialize_finish(id, serde_json::json!({
        "capabilities": {
            "textDocumentSync": 1,
            "codeActionProvider": true
        }
    }))?;

    run(connection, init_params).await?;
    
    io_threads.join()?;
    Ok(())
}
