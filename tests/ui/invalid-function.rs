#![feature(generators)]

use futures_async_stream::async_stream;

#[async_stream(item = ())]
const fn constness() {} //~ ERROR async stream must be declared as async

#[async_stream(item = ())]
fn variadic(_: ...) {} //~ ERROR async stream may not be variadic

#[async_stream(item = ())]
fn asyncness() {} //~ ERROR async stream must be declared as async

#[async_stream(item = ())]
async fn output() -> i32 {} //~ ERROR async stream must return the unit type

#[async_stream(item = ())]
async fn unit() -> () {} // OK

fn main() {}
