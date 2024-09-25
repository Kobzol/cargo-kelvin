use anyhow::Context;
use clap::Parser;
use ignore::DirEntry;
use log::LevelFilter;
use reqwest::StatusCode;
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Parser, Debug)]
#[command(author, version, about)]
#[clap(bin_name("cargo"))]
#[clap(disable_help_subcommand(true))]
enum Args {
    #[clap(author, version, about)]
    Kelvin(InnerArgs),
}

#[derive(Parser, Debug)]
struct InnerArgs {
    #[clap(subcommand)]
    subcmd: RootArgs,
}

#[derive(Parser, Debug)]
enum RootArgs {
    /// Submit the current directory to Kelvin.
    Submit(SubmitArgs),
}

#[derive(Parser, Debug)]
struct SubmitArgs {
    /// Assignment ID into which your code should be submitted.
    /// You can find it in the URL of the task, i.e. `https://kelvin.cs.vsb.cz/task/<assignment-id>/<your-login>`.
    assignment_id: u64,

    /// API token for submitting things to Kelvin.
    /// You can generate it at `https://kelvin.cs.vsb.cz/api_token`.
    /// You can pass it to `cargo kelvin` through an environment variable `KELVIN_API_TOKEN`.
    #[clap(long, env = "KELVIN_API_TOKEN")]
    token: String,

    #[clap(long, default_value = "https://kelvin.cs.vsb.cz")]
    kelvin_url: String,

    /// Do not open the browser after uploading the submit.
    #[clap(long, default_value_t = false)]
    no_open: bool,
}

#[derive(serde::Deserialize, Debug)]
struct SubmitData {
    id: u64,
    url: String,
}

#[derive(serde::Deserialize, Debug)]
struct TaskData {
    name: String,
}

#[derive(serde::Deserialize, Debug)]
struct Response {
    submit: SubmitData,
    task: TaskData,
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();

    let Args::Kelvin(InnerArgs {
        subcmd:
            RootArgs::Submit(SubmitArgs {
                assignment_id,
                token,
                kelvin_url,
                no_open,
            }),
    }) = Args::parse();

    let manifest = get_manifest_path()?;
    let archive = compress_workspace(manifest)?;

    let client = reqwest::blocking::Client::new();

    let file = reqwest::blocking::multipart::Part::bytes(archive).file_name("submit.zip");
    let form = reqwest::blocking::multipart::Form::new().part("solution", file);

    let res = client
        .post(format!("{kelvin_url}/api/submits/{assignment_id}"))
        .header("Authorization", format!("Bearer {token}"))
        .multipart(form)
        .send()
        .context("sending submit to Kelvin")?;
    if res.status() != StatusCode::OK {
        log::error!(
            "The submit was not successful. Status error: {}",
            res.status(),
        );
        log::debug!(
            "Response content: {}",
            res.text().context("getting content of HTTP response")?
        );
    } else {
        let response: Response = res.json().context("deserializing response")?;
        log::info!(
            "Created submit #{} for task {}",
            response.submit.id,
            response.task.name
        );
        log::info!("You can find the submit at {}", response.submit.url);
        if !no_open {
            open::that(response.submit.url).context("opening browser")?;
        }
    }

    Ok(())
}

fn get_manifest_path() -> anyhow::Result<PathBuf> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .context("getting cargo metadata")?;
    Ok(metadata
        .workspace_root
        .into_std_path_buf()
        .join("Cargo.toml"))
}

fn is_valid_path(entry: &DirEntry) -> bool {
    let path = entry.path();
    if path.is_dir() {
        return true;
    }
    path.is_file()
        && path.extension().map_or(false, |ext| {
            ext == "toml" || ext == "lock" || ext == "rs" || ext == "md" || ext == "txt"
        })
}

fn compress_workspace(manifest_path: PathBuf) -> anyhow::Result<Vec<u8>> {
    let root_dir = manifest_path.parent().expect("Manifest path has no parent");

    let mut zip = ZipWriter::new(std::io::Cursor::new(Vec::new()));

    let mut file_count = 0;
    let iter = ignore::WalkBuilder::new(&root_dir)
        .max_filesize(Some(1024 * 1024))
        .same_file_system(true)
        .filter_entry(is_valid_path)
        .build();
    for file in iter {
        match file {
            Ok(file) => {
                if !is_valid_path(&file) {
                    continue;
                }
                if file.path().is_dir() {
                    continue;
                }
                if file.path() == root_dir {
                    continue;
                }
                let Ok(relative_path) = file.path().strip_prefix(&root_dir) else {
                    continue;
                };
                if relative_path.starts_with("target") {
                    continue;
                }
                if let Err(error) = write_file_to_zip(&mut zip, relative_path, file.path()) {
                    log::warn!(
                        "Cannot write file {:?} to ZIP archive: {error:?}",
                        file.path()
                    );
                } else {
                    file_count += 1;
                }
            }
            Err(error) => log::warn!("Cannot include file {error:?}"),
        }
    }
    let data = zip
        .finish()
        .context("cannot create ZIP archive")?
        .into_inner();
    log::info!(
        "Compressed {file_count} file{}, total size: {}B",
        if file_count == 1 { "" } else { "s" },
        data.len()
    );
    Ok(data)
}

fn write_file_to_zip<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    relative_path: &Path,
    fs_path: &Path,
) -> anyhow::Result<()> {
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file_from_path(relative_path, options)
        .with_context(|| anyhow::anyhow!("Cannot store {relative_path:?} into ZIP archive"))?;
    let bytes = std::fs::read(fs_path)
        .with_context(|| anyhow::anyhow!("Cannot read file at {fs_path:?}"))?;
    zip.write(&bytes)
        .context("cannot write bytes into ZIP archive")?;
    Ok(())
}
