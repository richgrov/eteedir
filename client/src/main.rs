use crossterm::event::{self, read, Event, KeyCode};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use ratatui::{backend, text::Text, Frame};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{stdin, stdout, Error, Write};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tui_textarea::TextArea;

struct App {
    keyboard_send: broadcast::Sender<()>,
    keyboard_recv: broadcast::Receiver<()>,
    message_send: mpsc::Sender<String>,
    message_recv: mpsc::Receiver<String>,
}

impl App {
    pub fn new() -> App {
        let (keyboard_send, keyboard_recv) = tokio::sync::broadcast::channel(16);
        let (message_send, message_recv) = tokio::sync::mpsc::channel(16);

        App {
            keyboard_send,
            keyboard_recv,
            message_send,
            message_recv,
        }
    }
}

#[tokio::main]
async fn main() {
    let address = match std::env::args().nth(1) {
        Some(a) => a,
        None => {
            eprintln!("error: specify a server address");
            return;
        }
    };

    let mut terminal = ratatui::init();
    let mut textarea = TextArea::default();
    let (socket, _) = connect_async(format!("ws://{}/", address))
        .await
        .expect("can't connect");

    let (write, reader) = socket.split();
    let mut app = App::new();
    tokio::spawn(write_to_server(write, app.keyboard_send));
    tokio::spawn(receive_from_server(reader, app.message_send));
    loop {
        tokio::select! {
            exit_broadcast = app.keyboard_recv.recv() => {
                break;
            },
            msg_recv = app.message_recv.recv() => {
                if let Some(m) = msg_recv {
                    println!("Received: {}", m);
                }
            }
        }
        terminal.draw(draw).expect("uh oh");
        let rect = ratatui::layout::Rect::new(100, 100, 100, 100);

        // terminal.draw(|f| {
        //     f.render_widget(&textarea, rect);
        // });
        // if let Event::Key(key) = read().expect("kjsahfkjsahfkjadsf") {
        //     if key.code == KeyCode::Esc {
        //         break;
        //     }
        //     textarea.input(key);
        // }
    }
    ratatui::restore();
}

fn draw(frame: &mut Frame) {
    let test = Text::raw("hello");
    frame.render_widget(test, frame.area());
}

async fn write_to_server(
    mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    sending_channel: tokio::sync::broadcast::Sender<()>,
) {
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
            sending_channel
                .send(())
                .expect("Couldn't exit successfully");
            break;
        }

        write.send(message.into()).await.unwrap();
    }
}

async fn receive_from_server(
    mut read: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    sending_channel: tokio::sync::mpsc::Sender<String>,
) -> Result<(), Error> {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                sending_channel
                    .send(text)
                    .await
                    .expect("Couldn't receive message from server.");
            }
            Ok(Message::Binary(bin)) => println!("Received binary data: {:?}", bin),
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
            _ => panic!(),
        }
    }
    Ok(())
}
