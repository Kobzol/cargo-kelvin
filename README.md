# cargo kelvin
This is a simple utility that can be used to submit code submissions into the [Kelvin](https://github.com/mrlvsb/kelvin)
code evaluation tool. It is only useful for students of the [VSB-TUO](https://www.vsb.cz/en) university that submit Rust
code into Kelvin.

## Installation

```bash
$ cargo install --force --git https://github.com/kobzol/cargo-kelvin
```

## Usage
First, you need to [generate an API token](https://kelvin.cs.vsb.cz/api_token). Once you do that, you can pass it to
`cargo kelvin` either through an environment variable `KELVIN_API_TOKEN` or through the `--token` CLI flag.

You will also need to find out the `assignment ID` of the task that you are submitting your code into. You can either
find
it in the description of the task, or in the Kelvin URL (`kelvin.cs.vsb.cz/task/<assignment-id>/<your-username>/...`).

You are supposed to use `cargo kelvin` from within a Cargo project, ideally in the directory where `Cargo.toml` is
located.
It will find all (non-ignored) `.rs`, `.toml`, `.lock`, `.md` and `.txt` files in the current Cargo workspace, compress them into a ZIP archive
and upload the archive as a new submit into Kelvin.

```bash
$ cargo kelvin submit <assignment-id> [--token <kelvin-api-token>]
```

Run `cargo kelvin --help` to find out more.

❗Please do not upload new submits more often than once per minute, prefer running tests locally with `cargo test`. If you
spam Kelvin too much, we will ban your account.❗
