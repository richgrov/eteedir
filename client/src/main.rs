use common::{MessagePacket, Packet, ServerboundHandshake};
use crossterm::event::KeyCode;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::sign::Signer;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Paragraph};
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
    inbound_message_send: mpsc::Sender<String>,
    inbound_message_recv: mpsc::Receiver<String>,
    outbound_message_send: mpsc::Sender<String>,
    should_exit: bool,
    input: TextArea<'a>,
    history: Vec<String>,

    pkey: PKey<openssl::pkey::Private>,
}

impl<'a> App<'a> {
    pub fn new(outbound_message_send: mpsc::Sender<String>) -> App<'a> {
        let (keyboard_send, keyboard_recv) = mpsc::channel(16);
        let (message_send, message_recv) = tokio::sync::mpsc::channel(16);

        let rsa = Rsa::generate(2048).expect("failed to generate RSA key");

        App {
            terminal: ratatui::init(),
            keyboard_send,
            keyboard_recv,
            inbound_message_send: message_send,
            inbound_message_recv: message_recv,
            outbound_message_send,
            should_exit: false,
            input: Self::create_input_textarea(),
            history: Vec::new(),

            pkey: PKey::from_rsa(rsa).expect("failed to convert RSA to PKey"),
        }
    }

    pub fn on_key_press(&mut self, event: crossterm::event::Event) {
        if let crossterm::event::Event::Key(k) = event {
            match k.code {
                KeyCode::Esc => {
                    self.should_exit = true;
                    return;
                }

                KeyCode::Enter => {
                    let msg = self.input.lines()[0].clone();
                    self.input = Self::create_input_textarea();
                    self.send_message(msg);
                }

                _ => {
                    let input_event: tui_textarea::Input = event.into();
                    self.input.input(input_event);
                }
            }
        }

        self.draw();
    }

    pub fn packet_received(&mut self, raw_data: String) {
        let (id, json_data) = common::network_decode(&raw_data).unwrap();

        macro_rules! parse_packets {
            ($($packet_type:ident => $func:ident),* $(,)?) => {
                match id {
                $(
                    $packet_type::ID => self.$func(serde_json::from_str(json_data).unwrap()),
                )*
                    other => eprintln!("unexpected packet ID {}", other),
                }
            }
        }

        parse_packets!(
            MessagePacket => handle_message,
        );
    }

    fn handle_message(&mut self, message: MessagePacket) {
        self.history.push(message.content);
        self.draw();
    }

    pub fn network_init(&mut self) {
        let pem = self
            .pkey
            .public_key_to_pem()
            .expect("failed to encode public key as PEM");

        self.queue_packet(ServerboundHandshake {
            public_key: String::from_utf8(pem).unwrap(),
        });
    }

    pub fn draw(&mut self) {
        let history_paragraph = Paragraph::new(self.history.join("\n"));

        self.terminal
            .draw(|frame| {
                let area = frame.area();
                let textbox_rect = Rect::new(0, area.height - 3, area.width, 3);
                frame.render_widget(&self.input, textbox_rect);

                let history_rect = Rect::new(0, 0, area.width, area.height - 3);
                frame.render_widget(&history_paragraph, history_rect);
            })
            .unwrap();
    }

    fn create_input_textarea() -> TextArea<'a> {
        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type a message...");
        textarea.set_block(Block::default().borders(Borders::ALL));
        textarea
    }

    fn send_message(&self, message: String) {
        let mut signer = Signer::new(MessageDigest::sha256(), &self.pkey)
            .expect("failed to create message signer");

        signer.update(message.as_bytes());
        let signature = signer.sign_to_vec().expect("failed to sign message");

        self.queue_packet(MessagePacket {
            content: message,
            signature,
        });
    }

    fn queue_packet<P: Packet>(&self, packet: P) {
        self.outbound_message_send
            .try_send(packet.network_encode())
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
    let (outbound_msg_send, outbound_msg_recv) = mpsc::channel(8);
    let mut app = App::new(outbound_msg_send);
    tokio::spawn(read_console_input(app.keyboard_send.clone()));
    tokio::spawn(send_to_server(write, outbound_msg_recv));
    tokio::spawn(receive_from_server(
        reader,
        app.inbound_message_send.clone(),
    ));

    app.network_init();
    app.terminal.clear().unwrap();
    app.draw();

    while !app.should_exit {
        tokio::select! {
            maybe_event = app.keyboard_recv.recv() => {
                if let Some(event) = maybe_event {
                    app.on_key_press(event);
                }
            },
            msg_recv = app.inbound_message_recv.recv() => {
                if let Some(m) = msg_recv {
                    app.packet_received(m)
                }
            }
        }
    }
    ratatui::restore();
    panic!();
}

async fn read_console_input(sending_channel: mpsc::Sender<crossterm::event::Event>) {
    loop {
        let event = crossterm::event::read().expect("kjsahfkjsahfkjadsf");

        if sending_channel.send(event).await.is_err() {
            break;
        }
    }
}

async fn send_to_server(
    mut write: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    mut outbound_messages: mpsc::Receiver<String>,
) {
    while let Some(msg) = outbound_messages.recv().await {
        if let Err(e) = write.send(Message::Text(msg)).await {
            eprintln!("failed to send message: {}", e);
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
