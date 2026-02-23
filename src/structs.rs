use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;


#[derive(Parser, Debug)]
#[command(name = "Yandex Music Downloader", version = env!("CARGO_PKG_VERSION"))]
pub struct CliArgs {
    #[clap(short, long)]
    pub token: Option<String>,

    #[clap(short, long)]
    pub format: Option<u8>,

    #[clap(short, long)]
    pub get_original_covers: bool,

    #[clap(short, long)]
    pub keep_covers: bool,

    #[clap(short, long)]
    pub out_path: Option<PathBuf>,

    #[clap(short, long)]
    pub sleep: bool,

    #[clap(long)]
    pub write_covers: bool,

    #[clap(long)]
    pub write_lyrics: bool,

    #[clap(long)]
    pub album_template: Option<String>,

    #[clap(long)]
    pub track_template: Option<String>,

    #[clap(short, long, num_args = 1.., required = true)]
    pub urls: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct FileConfig {
    pub token: Option<String>,
    pub ffmpeg_path: Option<PathBuf>,
    pub format: u8,
    pub out_path: PathBuf,
    pub keep_covers: bool,
    pub get_original_covers: bool,
    pub write_covers: bool,
    pub write_lyrics: bool,
    pub sleep: bool,
    pub album_template: String,
    pub track_template: String,
}

impl Default for FileConfig {
    fn default() -> Self {
        Self{
            token: None,
            ffmpeg_path: None,
            format: 4,
            out_path: PathBuf::from("Yandex Music downloads"),
            album_template: "{album_artist} - {album_title}".to_string(),
            track_template: "{track_num_pad}. {title}".to_string(),
            keep_covers: false,
            write_covers: false,
            get_original_covers: false,
            write_lyrics: false,
            sleep: false,
        }
    }
}
