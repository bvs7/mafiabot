#[macro_use]
extern crate enum_kinds;

mod engine;
mod server;

// Game loop.

#[tokio::main]
async fn main() -> Result<(), ()> {
    // let subscriber = tracing_subscriber::FmtSubscriber::new();
    // // use that subscriber to process traces emitted after this point
    // tracing::subscriber::set_global_default(subscriber).unwrap();

    // info!("Starting");

    // let (tx, rx) = mpsc::channel(100);

    // let registry = HashMap::from([
    //     (1, Role::TOWN),
    //     (2, Role::COP),
    //     (3, Role::DOCTOR),
    //     (4, Role::MAFIA),
    // ]);
    // let core = Arc::new(RwLock::new(Core::new(0, registry, Rules {})));
    // let core2 = core.clone();

    // let game_task = tokio::spawn(async move { run_game(rx, core).await });

    // run_api(tx, core2).await.unwrap();

    Ok(())
}
