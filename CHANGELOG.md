# Unreleased

# 0.1.0-alpha.5 - 2019-08-29

* Pined the version of pin-project dependency

# 0.1.0-alpha.4 - 2019-08-23

* Removed usage of some feature gates.

* Updated `pin-project` to 0.4.0-alpha.4.

# 0.1.0-alpha.3 - 2019-08-14

* Updated `proc-macro2`, `syn`, and `quote` to 1.0.

# 0.1.0-alpha.2 - 2019-08-07

* [You can now use async stream functions in traits.][12] e.g. You can write the following:

  ```rust
  #![feature(async_await, generators)]
  use futures_async_stream::async_stream;

  trait Foo {
      #[async_stream(boxed, item = u32)]
      async fn method(&mut self);
  }

  struct Bar(u32);

  impl Foo for Bar {
      #[async_stream(boxed, item = u32)]
      async fn method(&mut self) {
          while self.0 < u32::max_value() {
              self.0 += 1;
              yield self.0;
          }
      }
  }
  ```

[12]: https://github.com/taiki-e/futures-async-stream/pull/12

# 0.1.0-alpha.1 - 2019-07-31

Initial release
