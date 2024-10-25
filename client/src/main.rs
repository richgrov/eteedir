use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use std::io::{stdin,stdout,Write};

#[tokio::main]
async fn main() {
    let (mut socket, _) = connect_async("ws://localhost:80/")
        .await
        .expect("can't connect");
    socket.send("hi".into()).await.unwrap();
    let response = socket.next().await.unwrap().unwrap();
    println!("{}", response);
    read_from_client().await;
}

async fn read_from_client() {
    let (mut socket, _) = connect_async("ws://localhost:80/")
        .await
        .expect("can't connect");

    let mut exit = false;
    print!("Do something \n");
    loop {
        let mut message = String::new();
        stdin().read_line(&mut message).expect("Did not enter blah blah blah");
        let _ = stdout().flush();
        if let Some('\n') = message.chars().next_back() {
            message.pop();
        }
        if let Some('\r') = message.chars().next_back() {
            message.pop();
        }

        if message.eq("exit"){
            break;
        }

        socket.send(message.into()).await.unwrap();
    }
}
