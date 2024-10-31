use std::io::{stdin, stdout, Error, Write};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() {
    let address = match std::env::args().nth(1) {
        Some(a) => a,
        None => {
            eprintln!("error: specify a server address");
            return;
        }
    };

    let (socket, _) = connect_async(format!("ws://{}/", address))
        .await
        .expect("can't connect");

    let (write_to_server_channel, mut write_to_server_broadcast) = tokio::sync::broadcast::channel(16);
    let (receive_from_server_channel, mut incoming_from_server_channel) = tokio::sync::mpsc::channel(16);
    let (write, read) = socket.split();
    tokio::spawn(write_to_server(write, write_to_server_channel));
    tokio::spawn(receive_from_server(read, receive_from_server_channel));
    loop {
        tokio::select! {
            exit_broadcast = write_to_server_broadcast.recv() => {
                break;
            },
            msg_recv = incoming_from_server_channel.recv() => {
                if let Some(m) = msg_recv {
                    println!("Received: {}", m);
                }
            }
        }
    }
}

async fn write_to_server(mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>, sending_channel: tokio::sync::broadcast::Sender<()>) {
    loop {
        let mut message = String::new();

        stdin()
            .read_line(&mut message)
            .expect("Did not enter valid value");

        let _ = stdout().flush();

        if let Some('\n') = message.chars().next_back() {
            message.pop();
        }

        if let Some('\r') = message.chars().next_back() {
            message.pop();
        }

        if message.to_lowercase().eq("exit") {
            sending_channel.send(()).expect("Couldn't exit successfully");
            break;
        }

        write.send(message.into()).await.unwrap();
    }
}

async fn receive_from_server(mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>, sending_channel: tokio::sync::mpsc::Sender<String>) -> Result<(), Error> {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                sending_channel.send(text).await.expect("Couldn't receive message from server.");
            },
            Ok(Message::Binary(bin)) => println!("Received binary data: {:?}", bin),
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
            _ => panic!()
        }
    };
    Ok(())
}
