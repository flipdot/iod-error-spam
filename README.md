# iod-error-spam

Retrieves errors from a dedicated MQTT topic and alerts users accordingly
(currently via IRC only).

## how to build / run / install (cargo 101)

First, make sure you have rust [installed][rust-install].

Now, choose what you want to do:

* Build only: `cargo build`
* Build and run: `cargo run`
* Build and install: `cargo install`

You can switch between `--debug` and `--release` builds. The default for `build`
and `run` is debug, `install` defaults to doing a release build.

The built executable will be in `target/debug` / `target/release`, and install
will copy it to `~/.cargo/bin`.

[rust-install]: https://www.rust-lang.org/en-US/install.html

## to do

* Handle MQTT errors more gracefully
  * E.g. try reconnecting when the connection is lost
* Last will: Send an error message somewhere.
* (?) Different ways of altering users
