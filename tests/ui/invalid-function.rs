#![feature(generators)]

use futures_async_stream::stream;

#[stream(item = ())]
const fn constness() {} //~ ERROR async stream must be declared as async

#[stream(item = ())]
fn variadic(_: ...) {} //~ ERROR only foreign functions are allowed to be C-variadic

#[stream(item = ())]
fn asyncness() {} //~ ERROR async stream must be declared as async

#[stream(item = ())]
async fn output() -> i32 {} //~ ERROR async stream must return the unit type

#[stream(item = ())]
async fn unit() -> () {} // OK

fn main() {}
