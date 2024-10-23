fn main() {
    let options = ewebsock::Options::default();
    // see documentation for more options
    let (mut sender, receiver) = ewebsock::connect("ws://wss.postman-echo.com/raw", options).unwrap();
    sender.send(ewebsock::WsMessage::Text("Hello!".into()));
    while let Some(event) = receiver.try_recv() {
        println!("Received {:?}", event);
    }
    println!("Shutting down...");
}
