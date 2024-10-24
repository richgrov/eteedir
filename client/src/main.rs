use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() {
    let (mut socket, _) = connect_async("wss://ws.postman-echo.com/raw")
        .await
        .expect("can't connect");
    socket.send("hi".into()).await.unwrap();
    let response = socket.next().await.unwrap().unwrap();
    println!("{}", response);
}
