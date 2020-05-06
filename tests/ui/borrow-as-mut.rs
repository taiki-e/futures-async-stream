#![feature(generators)]

use futures_async_stream::stream;

#[stream(item = ())]
async fn borrow_mut(a: Vec<u8>) {
    a.clear(); //~ ERROR E0596
}

fn main() {}
