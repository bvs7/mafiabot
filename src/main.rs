pub mod controller;
pub mod core;
pub mod discord;

#[tokio::main]
async fn main() {
    let mut client = discord::parser::get_client().await;
    //client.await.expect("I found a secret").start_autosharded();
    if let Err(why) = client.start().await {
        println!("Err with client: {:?}", why);
    }
}
