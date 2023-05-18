use tokio::runtime::Runtime;
mod gateway;

#[tokio::main]
async fn main() {
        gateway::serve().await;
}
