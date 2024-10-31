use crossterm::event::KeyCode;
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
    keyboard_send: broadcast::Sender<crossterm::event::Event>,
    keyboard_recv: broadcast::Receiver<crossterm::event::Event>,
    message_send: mpsc::Sender<String>,
    message_recv: mpsc::Receiver<String>,
    should_exit: bool,
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
            should_exit: false,
        }
    }

    pub fn on_key_press(&mut self, event: crossterm::event::Event) {
        if let crossterm::event::Event::Key(k) = event {
            if let KeyCode::Esc = k.code {
                self.should_exit = true;
            }
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
    tokio::spawn(write_to_server(write, app.keyboard_send.clone()));
    tokio::spawn(receive_from_server(reader, app.message_send.clone()));
    terminal.clear().unwrap();
    while !app.should_exit {
        tokio::select! {
            key = app.keyboard_recv.recv() => {
                match key {
                    Ok(k) => app.on_key_press(k),
                    Err(e) => {
                        eprintln!("uh oh! {}", e);
                        break;
                    }
                }
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
    }
    ratatui::restore();
}

fn draw(frame: &mut Frame) {
    let test = Text::raw("hello");
    frame.render_widget(test, frame.area());
}

async fn write_to_server(
    mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    sending_channel: tokio::sync::broadcast::Sender<crossterm::event::Event>,
) {
    loop {
        let event = crossterm::event::read().expect("kjsahfkjsahfkjadsf");

        sending_channel
            .send(event)
            .expect("Couldn't exit successfully");
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
