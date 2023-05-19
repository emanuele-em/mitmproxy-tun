mod gateway;

#[tokio::main]
async fn main() {
    gateway::serve().await;
}
