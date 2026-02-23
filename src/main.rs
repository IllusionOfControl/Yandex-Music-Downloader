use clap::Parser;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crate::api::client::YandexMusicClient;
use crate::models::{DownloadFormat, MediaLink, Settings};
use crate::structs::{CliArgs, FileConfig};
use crate::utils::resolve_ffmpeg_path;

mod api;
mod metadata;
mod models;
mod processor;
mod structs;
mod tags;
mod utils;

fn bootstrap() -> Result<Settings, Box<dyn Error>> {
    let cli = CliArgs::parse();

    let exe_path = utils::get_exe_path()?;
    let config_path = exe_path.join("config.toml");
    let file_cfg: FileConfig = if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        toml::from_str(&content)?
    } else {
        FileConfig::default()
    };

    let token = cli
        .token
        .or(file_cfg.token.clone())
        .filter(|t| !t.trim().is_empty())
        .ok_or("Token is missing! Please set it in args or config.toml")?;

    let format_raw = cli.format
        .or(file_cfg.format)
        .unwrap_or(4);

    let format = DownloadFormat::from_u8(format_raw)
        .ok_or("Invalid format! Use 1-4.")?;

    let out_path = cli.out_path
        .or(file_cfg.out_path)
        .unwrap_or(PathBuf::from("downloads"));

    let ffmpeg_path = resolve_ffmpeg_path(file_cfg.ffmpeg_path, &exe_path);

    let album_template = cli.album_template
        .or(file_cfg.album_template)
        .unwrap_or("{album_artist} - {album_title}".to_string());
    let track_template = cli.track_template
        .or(file_cfg.track_template)
        .unwrap_or("{track_num_pad}. {title}".to_string());

    let keep_covers = cli.keep_covers || file_cfg.keep_covers.unwrap_or(false);
    let write_covers = cli.write_covers || file_cfg.write_covers.unwrap_or(false);
    let get_original_covers = cli.get_original_covers || file_cfg.get_original_covers.unwrap_or(false);
    let write_lyrics = cli.write_lyrics || file_cfg.write_lyrics.unwrap_or(false);
    let sleep = cli.sleep || file_cfg.sleep.unwrap_or(false);

    let processed_url_strings = utils::process_urls(&cli.urls)?;
    let mut media_links = Vec::new();

    for url in processed_url_strings {
        match utils::parse_url(&url) {
            Some(link) => media_links.push(link),
            None => eprintln!("Warning: Skipping invalid URL: {}", url),
        }
    }

    if media_links.is_empty() {
        return Err("No valid URLs to process!".into());
    }

    // Возвращаем готовую структуру
    Ok(Settings {
        token,
        format,
        out_path,
        ffmpeg_path,
        keep_covers,
        write_covers,
        get_original_covers,
        write_lyrics,
        album_template,
        track_template,
        sleep,
        media_links,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let settings = bootstrap().expect("Failed to initialize settings");

    println!("Signing in...");
    let mut client = YandexMusicClient::new(&settings.token)?;
    println!("Signed in as: {}\n", client.login);

    let total_links = settings.media_links.len();

    for (i, link) in settings.media_links.iter().enumerate() {
        let current_num = i + 1;
        println!("URL {} of {}:", current_num, total_links);

        let result = match link {
            MediaLink::Album { album_id } => {
                processor::process_album(&mut client, &settings, album_id, None, None)
            }
            MediaLink::Track { album_id, track_id } => {
                processor::process_album(&mut client, &settings, album_id, Some(track_id), None)
            }
            MediaLink::Playlist { uuid_or_login } => {
                processor::process_user_playlist(&mut client, &settings, uuid_or_login)
            }
            MediaLink::Artist { artist_id } => {
                processor::process_artist_albums(&mut client, &settings, artist_id)
            }
        };

        if let Err(e) = result {
            eprintln!("Error processing link: {}", e);
        }

        if settings.sleep && current_num < total_links {
            println!("Sleeping...");
            thread::sleep(Duration::from_secs(2));
        }
    }

    println!("\nDone!");
    Ok(())
}
