use tokio::{join, runtime::Runtime};
mod gateway;

#[tokio::main]
async fn main() {
    let mut rt = Runtime::new().unwrap();
    //let metrics_addr = setting.metrics.clone();

    rt.spawn(async move {
        gateway::serve().await;
    });
    loop {}
}
