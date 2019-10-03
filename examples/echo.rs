#![feature(stmt_expr_attributes, proc_macro_hygiene)]

use futures_async_stream::for_await;
use std::env;
use tokio::{io::AsyncReadExt, net::TcpListener};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on {}", addr);

    #[for_await]
    for stream in listener.incoming() {
        let mut stream = stream?;

        tokio::spawn(async move {
            let (mut reader, mut writer) = stream.split();
            reader.copy(&mut writer).await.expect("failed to copy data from socket into socket");
        });
    }

    Ok(())
}
