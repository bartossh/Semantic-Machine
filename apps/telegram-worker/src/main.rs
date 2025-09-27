use rust_tdlib::{
    client::Worker,
    types::{TdlibParameters, Update},
};

mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let config = config::TelegramConfig::try_from_env().map_err(|e| format!("{e}"))?;

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<Box<Update>>(10);
    let tdlib_params = TdlibParameters::builder()
        .api_id(config.telegram_api_id)
        .api_hash(config.telegram_api_hash)
        .build();
    let client = rust_tdlib::client::Client::builder()
        .with_tdlib_parameters(tdlib_params)
        .with_updates_sender(sender)
        .build()?;
    let mut worker = Worker::builder().build().unwrap();
    let _waiter = worker.start();
    let (_client_state, _client) = worker.get_client_state(&client).await.unwrap();
    if let Some(message) = receiver.recv().await {
        eprintln!("updates handler received {:?}", message);
    }

    Ok(())
}
