# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org).

Releases may yanked if there is a security bug, a soundness bug, or a regression.

<!--
Note: In this file, do not use the hard wrap in the middle of a sentence for compatibility with GitHub comment style markdown rendering.
-->

## [Unreleased]

## [0.2.12] - 2024-08-23

- Disable `derive` and `clone-impls` features of `syn` dependency.

## [0.2.11] - 2024-04-27

- Fix build failure on the latest nightly (nightly-2024-04-25+). ([#97](https://github.com/taiki-e/futures-async-stream/issues/97))

## [0.2.10] - 2024-01-12

- Fix "unexpected token" error on the comma following the async block. ([#94](https://github.com/taiki-e/futures-async-stream/pull/94), thanks @ethe)

## [0.2.9] - 2023-10-21

- Update to [new coroutine API since nightly-2023-10-21](https://github.com/rust-lang/rust/pull/116958). ([5860296](https://github.com/taiki-e/futures-async-stream/commit/5860296c2b0252d389dd5646f05febc524caddb5))

  On this crate's user side, the only change required is the following:

  ```diff
  - #![feature(generators)]
  + #![feature(coroutines)]
  ```

## [0.2.8] - 2023-10-09

- Update to syn 2.0. ([#92](https://github.com/taiki-e/futures-async-stream/pull/92))

## [0.2.7] - 2023-06-29

- Fix build error from dependency when built with `-Z minimal-versions`.

## [0.2.6] - 2023-01-05

- Fix "generator cannot be sent between threads safely" error since nightly-2023-01-04.

## [0.2.5] - 2021-01-05

- Exclude unneeded files from crates.io.

## [0.2.4] - 2020-12-29

- Documentation improvements.

## [0.2.3] - 2020-11-08

- Update `pin-project` to 1.0.

## [0.2.2] - 2020-06-11

- All macros can now compile with `forbid(unsafe_code)`.

- Support overlapping lifetime names in HRTB.

- Diagnostic improvements.

## [0.2.1] - 2020-05-21

- Diagnostic improvements.

## [0.2.0] - 2020-05-10

- Rename `#[async_stream]` to `#[stream]`. ([#45](https://github.com/taiki-e/futures-async-stream/pull/45))

- Rename `#[async_try_stream]` to `#[stream]`. ([#45](https://github.com/taiki-e/futures-async-stream/pull/45))

- Rename `async_stream_block!` to `stream_block!`. ([#45](https://github.com/taiki-e/futures-async-stream/pull/45))

- Rename `async_try_stream_block!` to `try_stream_block!`. ([#45](https://github.com/taiki-e/futures-async-stream/pull/45))

- Bug fixes.

- Diagnostic improvements.

## [0.1.5] - 2020-05-04

- `#[async_stream]` and `#[async_try_stream]` attributes can now be used on async blocks. ([#44](https://github.com/taiki-e/futures-async-stream/pull/44))

## [0.1.4] - 2020-04-22

- futures-async-stream now works on no-std environment. ([#34](https://github.com/taiki-e/futures-async-stream/pull/34))

## [0.1.3] - 2020-02-20

- Fix build failure on the latest nightly. ([#33](https://github.com/taiki-e/futures-async-stream/pull/33))

## [0.1.2] - 2019-12-10

- Fix build failure on the latest nightly. ([#31](https://github.com/taiki-e/futures-async-stream/pull/31))

## [0.1.1] - 2019-11-13

- Fix duplicate documentation.

## [0.1.0] - 2019-11-13

- Update `futures` to 0.3.0.

## [0.1.0-alpha.7] - 2019-09-25

- Update `pin-project` to 0.4.

## [0.1.0-alpha.6] - 2019-09-06

- Add `async_try_stream` to support `?` operator in async stream. ([#15](https://github.com/taiki-e/futures-async-stream/pull/15))

  For example, you can write the following:

  ```rust
  #![feature(generators)]
  use futures::stream::Stream;
  use futures_async_stream::async_try_stream;

  #[async_try_stream(ok = i32, error = Box<dyn std::error::Error>)]
  async fn foo(stream: impl Stream<Item = String>) {
      #[for_await]
      for x in stream {
          yield x.parse()?;
      }
  }
  ```

- Update `pin-project` to 0.4.0-alpha.9.

## [0.1.0-alpha.5] - 2019-08-29

- Pin the version of pin-project dependency.

## [0.1.0-alpha.4] - 2019-08-23

- Remove usage of some feature gates.

- Update `pin-project` to 0.4.0-alpha.4.

## [0.1.0-alpha.3] - 2019-08-14

- Update `proc-macro2`, `syn`, and `quote` to 1.0.

## [0.1.0-alpha.2] - 2019-08-07

- You can now use async stream functions in traits. ([#12](https://github.com/taiki-e/futures-async-stream/pull/12))

  For example, you can write the following:

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
          while self.0 < u32::MAX {
              self.0 += 1;
              yield self.0;
          }
      }
  }
  ```

## [0.1.0-alpha.1] - 2019-07-31

Initial release

[Unreleased]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.12...HEAD
[0.2.12]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.11...v0.2.12
[0.2.11]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.10...v0.2.11
[0.2.10]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.9...v0.2.10
[0.2.9]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.8...v0.2.9
[0.2.8]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.7...v0.2.8
[0.2.7]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/taiki-e/futures-async-stream/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.5...v0.2.0
[0.1.5]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.4...v0.1.5
[0.1.4]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.3...v0.1.4
[0.1.3]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.2...v0.1.3
[0.1.2]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.0-alpha.7...v0.1.0
[0.1.0-alpha.7]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.0-alpha.6...v0.1.0-alpha.7
[0.1.0-alpha.6]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.0-alpha.5...v0.1.0-alpha.6
[0.1.0-alpha.5]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.0-alpha.4...v0.1.0-alpha.5
[0.1.0-alpha.4]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.0-alpha.3...v0.1.0-alpha.4
[0.1.0-alpha.3]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.0-alpha.2...v0.1.0-alpha.3
[0.1.0-alpha.2]: https://github.com/taiki-e/futures-async-stream/compare/v0.1.0-alpha.1...v0.1.0-alpha.2
[0.1.0-alpha.1]: https://github.com/taiki-e/futures-async-stream/releases/tag/v0.1.0-alpha.1
