use std::io::{stdin, stdout, Write};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

#[tokio::main]
async fn main() {
    let (socket, _) = connect_async("ws://localhost:80/")
        .await
        .expect("can't connect");

    read_from_client(socket).await;
}

async fn read_from_client(socket: WebSocketStream<MaybeTlsStream<TcpStream>>) {
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

        socket.send(message.into()).await.unwrap();
    }
}
