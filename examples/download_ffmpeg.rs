use ffmpeg_sidecar::{
  command::ffmpeg_is_installed,
  download::{check_latest_version, download_ffmpeg_package, ffmpeg_download_url, unpack_ffmpeg},
  paths::sidecar_dir,
  version::ffmpeg_version,
};
use std::{
  env::current_exe,
  path::{self, Component, Path, PathBuf},
};

fn main() -> anyhow::Result<()> {
  if ffmpeg_is_installed() {
    println!("FFmpeg is already installed! 🎉");
    println!("For demo purposes, we'll re-download and unpack it anyway.");
    println!("TIP: Use `auto_download()` to skip manual customization.");
  }

  // Short version without customization:
  // ```rust
  // ffmpeg_sidecar::download::auto_download().unwrap();
  // ```

  // Checking the version number before downloading is actually not necessary,
  // but it's a good way to check that the download URL is correct.
  match check_latest_version() {
    Ok(version) => println!("Latest available version: {}", version),
    Err(_) => println!("Skipping version check on this platform."),
  }

  // These defaults will automatically select the correct download URL for your
  // platform.
  let download_url = ffmpeg_download_url()?;
  let cli_arg = std::env::args().nth(1);
  let destination = match cli_arg {
    Some(arg) => resolve_relative_path(current_exe()?.parent().unwrap().join(arg)),
    None => sidecar_dir()?,
  };

  // By default the download will use a `curl` command. You could also write
  // your own download function and use another package like `reqwest` instead.
  println!("Downloading from: {:?}", download_url);
  let archive_path = download_ffmpeg_package(download_url, &destination)?;
  println!("Downloaded package: {:?}", archive_path);

  // Extraction uses `tar` on all platforms (available in Windows since version 1803)
  println!("Extracting...");
  unpack_ffmpeg(&archive_path, &destination)?;

  // Use the freshly installed FFmpeg to check the version number
  let version = ffmpeg_version()?;
  println!("FFmpeg version: {}", version);

  println!("Done! 🏁");
  Ok(())
}

fn resolve_relative_path(path_buf: PathBuf) -> PathBuf {
  let mut components: Vec<PathBuf> = vec![];
  for component in path_buf.as_path().components() {
    match component {
      Component::Prefix(_) | Component::RootDir => components.push(component.as_os_str().into()),
      Component::CurDir => (),
      Component::ParentDir => {
        if !components.is_empty() {
          components.pop();
        }
      }
      Component::Normal(component) => components.push(component.into()),
    }
  }
  PathBuf::from_iter(components)
}
