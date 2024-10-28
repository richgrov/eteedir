use std::io::{stdin, stdout, Error, Split, Write};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() {
    let (socket, _) = connect_async("ws://localhost:80/")
        .await
        .expect("can't connect");

    let (write, read) = socket.split();
    tokio::spawn(write_to_server(write));
    receive_from_server(read).await.unwrap();
}

async fn write_to_server(mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>) {
    print!("Do something \n");
    loop {
        let mut message = String::new();
        stdin()
            .read_line(&mut message)
            .expect("Did not enter blah blah blah");
        let _ = stdout().flush();
        if let Some('\n') = message.chars().next_back() {
            message.pop();
        }
        if let Some('\r') = message.chars().next_back() {
            message.pop();
        }

        if message.eq("exit") {
            break;
        }

        write.send(message.into()).await.unwrap();
    }
}

async fn receive_from_server(mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>) -> Result<(), Error> {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => println!("Received: {}", text),
            Ok(Message::Binary(bin)) => println!("Received binary data: {:?}", bin),
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            },
            _ => panic!()
        }
    }
    Ok(())
}
