use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Error as ReqwestErr;

use crate::api::client::YandexMusicClient;
use crate::models::{ParsedAlbumMeta, Settings};
use crate::utils;
use crate::metadata;
use crate::tags;

const BUF_SIZE: usize = 1024 * 1024;

pub fn process_artist_albums(c: &mut YandexMusicClient, settings: &Settings, artist_id: &str) -> Result<(), Box<dyn Error>> {
    let meta = c.get_artist_meta(artist_id)?;
    println!("Artist: {}", meta.artist.name);

    let sanitized_artist_name = utils::sanitise(&meta.artist.name);
    let artist_path = settings.out_path.join(&sanitized_artist_name);

    let album_ids = meta.albums;
    let album_total = album_ids.len();

    if album_total == 0 {
        return Err("Artist has no albums".into());
    }

    for (i, album) in album_ids.iter().enumerate() {
        let current_album = i + 1;
        println!("\nAlbum {} of {}:", current_album, album_total);

        if let Err(e) = process_album(c, settings, &album.id.to_string(), None, Some(&artist_path)) {
            eprintln!("Failed to process album ID {}: {}", album.id, e);
        }

        if settings.sleep && current_album < album_total {
            thread::sleep(Duration::from_secs(1));
        }
    }
    Ok(())
}

pub fn process_user_playlist(c: &mut YandexMusicClient, settings: &Settings, login: &str) -> Result<(), Box<dyn Error>> {
    let playlist = c.get_other_user_playlist_meta(login)?;
    if playlist.visibility.to_lowercase() != "public" {
        return Err("Playlist is private".into());
    }

    let meta = c.get_playlist_meta(&playlist.playlist_uuid)?;
    if !meta.available {
        return Err("Playlist is unavailable".into());
    }

    let plist_folder = format!("{} - {}", meta.owner.login, meta.title);
    println!("Playlist: {}", plist_folder);

    let san_album_folder = utils::sanitise(&plist_folder);
    let plist_path = settings.out_path.join(san_album_folder);
    fs::create_dir_all(&plist_path)?;

    let track_total = meta.tracks.len() as u16;

    for (mut track_num, t) in meta.tracks.into_iter().enumerate() {
        let track = t.track;
        if track.track_source.to_lowercase() != "own" {
            println!("Skipped user-uploaded track.");
            continue;
        }

        track_num += 1;

        if !track.available || !track.albums[0].available {
            println!("Track or Album is unavailable.");
            continue;
        }

        let mut parsed_meta = metadata::parse_album_meta_playlist(&track.albums[0], track_total);
        if let Some(uri) = &track.cover_uri {
            parsed_meta.cover_data = get_cover_data(c, uri, settings.get_original_covers)?;
        }

        metadata::parse_track_meta_playlist(&mut parsed_meta, &track, track_num as u16);
        if let Err(e) = process_track(c, &track.id, &mut parsed_meta, settings, &plist_path) {
            eprintln!("Track failed: {:?}", e);
        }
    }
    Ok(())
}

pub fn process_album(
    c: &mut YandexMusicClient,
    settings: &Settings,
    album_id: &str,
    single_track_id: Option<&String>,
    artist_path: Option<&PathBuf>,
) -> Result<(), Box<dyn Error>> {
    let mut album_meta = c.get_album_meta(album_id)?;
    if !album_meta.available {
        return Err("Album is unavailable".into());
    }

    let track_total: usize = album_meta.volumes.iter().map(|v| v.len()).sum();
    let mut parsed_meta = metadata::parse_album_meta(&album_meta, track_total as u16);

    let album_folder_name = utils::parse_album_template(&settings.album_template, &parsed_meta);
    let album_path = artist_path.unwrap_or(&settings.out_path).join(album_folder_name);

    fs::create_dir_all(&album_path)?;
    println!("Album: {} - {}", parsed_meta.album_artist, parsed_meta.album_title);

    if let Some(uri) = &album_meta.cover_uri {
        if settings.keep_covers || settings.write_covers {
            let cover_data = get_cover_data(c, uri, settings.get_original_covers)?;
            if settings.keep_covers {
                let cover_path = album_path.join("folder.jpg");
                if let Err(e) = fs::write(&cover_path, &cover_data) {
                    eprintln!("Warning: Failed to write cover.jpg file {:?}: {}", &cover_path, e);
                } else {
                    println!("Lyrics saved to: {:?}", &cover_path);
                }
            }
            if settings.write_covers {
                parsed_meta.cover_data = cover_data;
            }
        }
    }

    if let Some(tid) = single_track_id {
        for volume in &mut album_meta.volumes {
            volume.retain(|track| &track.id == tid);
        }
        if album_meta.volumes.iter().all(|v| v.is_empty()) {
            return Err("Track not found in this album".into());
        }
    }

    let mut global_track_num = 0;
    for volume in album_meta.volumes {
        for track in volume {
            global_track_num += 1;
            if !track.available {
                println!("Track is unavailable.");
                continue;
            }

            let mut track_meta = parsed_meta.clone();
            metadata::parse_track_meta(&mut track_meta, &track, global_track_num as u16, single_track_id.is_some());

            if let Err(e) = process_track(c, &track.id, &mut track_meta, settings, &album_path) {
                eprintln!("Failed to download track {}: {}", track.title, e);
            }
        }
    }
    Ok(())
}

fn process_track(
    c: &mut YandexMusicClient,
    track_id: &str,
    meta: &mut ParsedAlbumMeta,
    settings: &Settings,
    album_path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let info = c.get_file_info(track_id, settings.format.as_api_str())?;

    let (specs, file_ext) = metadata::parse_specs(&info.codec, info.bitrate)
        .ok_or_else(|| format!("Unknown codec returned: {}", info.codec))?;

    if meta.is_track_only {
        println!("Track 1 of 1: {} - {}", meta.title, specs);
    } else {
        println!("Track {}/{}: {} - {}", meta.track_num, meta.track_total, meta.title, specs);
    }

    let padding = utils::format_track_number(meta.track_num, meta.track_total);
    let track_filename = utils::parse_track_template(&settings.track_template, meta, &padding);

    let mut track_path_base = album_path.join(track_filename);
    let mut final_path = track_path_base.with_extension(&file_ext);

    match utils::file_exists(&final_path) {
        Ok(true) => {
            println!("Track already exists locally.");
            return Ok(());
        }
        Ok(false) => {}
        Err(err) if cfg!(target_os = "windows") && err.raw_os_error() == Some(206) => {
            track_path_base = album_path.join(padding);
            final_path = track_path_base.with_extension(&file_ext);
            println!("Path too long, renaming to track number only.");
        }
        Err(err) => return Err(err.into()),
    }

    let mut track_buff = get_track_data(c, &info.url)?;

    println!("Decrypting...");
    if let Err(e) = utils::decrypt_buff(&mut track_buff, &info.key) {
        return Err(format!("Decryption failed: {}", e).into());
    }

    fs::write(&final_path, &track_buff)?;

    drop(track_buff);

    println!("Muxing...");
    if let Err(e) = mux(&final_path, &final_path, &settings.ffmpeg_path) {
        let _ = fs::remove_file(file_ext);
        return Err(format!("FFmpeg muxing failed: {}", e).into());
    }

    let _ = fs::remove_file(&final_path);

    if let Some(has_lyrics) = meta.lyrics_avail {
        if let Ok(lyrics_text) = get_lyrics_text(c, track_id, has_lyrics) {
            if has_lyrics && settings.write_lyrics {
                let lyric_path = track_path_base.with_extension("lrc");

                if let Err(e) = fs::write(&lyric_path, &lyrics_text) {
                    eprintln!("Warning: Failed to write lyrics file {:?}: {}", lyric_path, e);
                } else {
                    println!("Lyrics saved to: {:?}", lyric_path);
                }
            }
            if has_lyrics {
                meta.timed_lyrics = Some(lyrics_text);
            } else {
                meta.untimed_lyrics = Some(lyrics_text);
            }
        }
    }

    tags::write_tags(&final_path, &info.codec, meta)?;

    Ok(())
}

fn get_track_data(c: &mut YandexMusicClient, url: &str) -> Result<(Vec<u8>), Box<dyn Error>> {
    let mut resp = c.get_file_resp(url, true)?;

    let total_size = resp.content_length().unwrap_or(0);

    let mut track_buff: Vec<u8> = if total_size > 0 {
        Vec::with_capacity(total_size as usize)
    } else {
        Vec::new()
    };

    let mut buf = vec![0u8; BUF_SIZE];
    let mut downloaded: usize = 0;

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% at {binary_bytes_per_sec}, {bytes}/{total_bytes} (ETA: {eta})"
        )?.progress_chars("#>-")
    );

    loop {
        let bytes_read = resp.read(&mut buf)?;
        if bytes_read == 0 { break; }

        track_buff.extend_from_slice(&buf[..bytes_read]);

        downloaded += bytes_read;
        pb.set_position(downloaded as u64);
    }
    pb.finish();
    Ok(track_buff)
}

fn mux(in_path: &PathBuf, out_path: &PathBuf, ffmpeg_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let cmd = Command::new(ffmpeg_path)
        .arg("-i").arg(in_path)
        .arg("-c:a").arg("copy")
        .arg(out_path)
        .stderr(Stdio::piped())
        .output()?;

    if !cmd.status.success() {
        return Err(String::from_utf8_lossy(&cmd.stderr).into());
    }
    Ok(())
}

fn get_cover_data(c: &mut YandexMusicClient, url: &str, original: bool) -> Result<Vec<u8>, Box<ReqwestErr>> {
    let to_replace = if original { "/orig" } else { "/1000x1000" };
    let full_url = format!("https://{}", url.replace("/%%", to_replace));
    let resp = c.get_file_resp(&full_url, false)?;
    Ok(resp.bytes()?.into_iter().collect())
}

fn get_lyrics_text(c: &mut YandexMusicClient, track_id: &str, timed: bool) -> Result<String, Box<dyn Error>> {
    let lyrics_meta = c.get_lyrics_meta(track_id, timed)?;
    let resp = c.get_file_resp(&lyrics_meta.download_url, false)?;
    Ok(resp.text()?)
}