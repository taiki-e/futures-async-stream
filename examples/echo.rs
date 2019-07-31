#![feature(async_await, stmt_expr_attributes, proc_macro_hygiene)]

// TODO: switch to tokio 0.2-alpha

use std::{env, net::SocketAddr};

use futures::{
    executor::{self, ThreadPool},
    io::AsyncReadExt,
    task::SpawnExt,
};
use romio::TcpListener;

use futures_async_stream::for_await;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse::<SocketAddr>()?;

    executor::block_on(async {
        let mut threadpool = ThreadPool::new()?;

        let mut listener = TcpListener::bind(&addr)?;
        println!("Listening on {}", addr);

        #[for_await]
        for stream in listener.incoming() {
            let stream = stream?;

            threadpool
                .spawn(async move {
                    let (reader, mut writer) = stream.split();
                    reader
                        .copy_into(&mut writer)
                        .await
                        .expect("failed to copy data from socket into socket");
                })
                .unwrap();
        }

        Ok(())
    })
}
