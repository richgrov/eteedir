use crossterm::event::KeyCode;
use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::{text::Text, Frame};
use std::io::Error;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tui_textarea::TextArea;

struct App<'a> {
    terminal: ratatui::Terminal<CrosstermBackend<std::io::Stdout>>,
    keyboard_send: mpsc::Sender<crossterm::event::Event>,
    keyboard_recv: mpsc::Receiver<crossterm::event::Event>,
    message_send: mpsc::Sender<String>,
    message_recv: mpsc::Receiver<String>,
    should_exit: bool,
    input: TextArea<'a>,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let (keyboard_send, keyboard_recv) = mpsc::channel(16);
        let (message_send, message_recv) = tokio::sync::mpsc::channel(16);

        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type a message...");

        App {
            terminal: ratatui::init(),
            keyboard_send,
            keyboard_recv,
            message_send,
            message_recv,
            should_exit: false,
            input: textarea,
        }
    }

    pub fn on_key_press(&mut self, event: crossterm::event::Event) {
        if let crossterm::event::Event::Key(k) = event {
            if let KeyCode::Esc = k.code {
                self.should_exit = true;
                return;
            }

            let input_event: tui_textarea::Input = event.into();
            self.input.input(input_event);
        }

        self.draw();
    }

    pub fn draw(&mut self) {
        self.terminal.draw(draw).expect("uh oh");
        let rect = ratatui::layout::Rect::new(100, 100, 100, 100);

        self.terminal
            .draw(|frame| {
                frame.render_widget(&self.input, frame.area());
            })
            .unwrap();
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

    let (socket, _) = connect_async(format!("ws://{}/", address))
        .await
        .expect("can't connect");

    let (write, reader) = socket.split();
    let mut app = App::new();
    tokio::spawn(read_console_input(app.keyboard_send.clone()));
    tokio::spawn(receive_from_server(reader, app.message_send.clone()));

    app.terminal.clear().unwrap();
    app.draw();

    while !app.should_exit {
        tokio::select! {
            maybe_event = app.keyboard_recv.recv() => {
                if let Some(event) = maybe_event {
                    app.on_key_press(event);
                }
            },
            msg_recv = app.message_recv.recv() => {
                if let Some(m) = msg_recv {
                    //println!("Received: {}", m);
                }
            }
        }
    }
    ratatui::restore();
    panic!();
}

fn draw(frame: &mut Frame) {
    let test = Text::raw("hello");
    frame.render_widget(test, frame.area());
}

async fn read_console_input(sending_channel: mpsc::Sender<crossterm::event::Event>) {
    loop {
        let event = crossterm::event::read().expect("kjsahfkjsahfkjadsf");

        if sending_channel.send(event).await.is_err() {
            break;
        }
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
