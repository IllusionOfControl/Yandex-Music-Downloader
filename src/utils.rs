use std::{env, fs, io};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Error as IoError};
use std::path::PathBuf;
use std::sync::OnceLock;
use regex::{Regex};
use aes::cipher::{KeyIvInit, StreamCipher};
use crate::models::{MediaLink, ParsedAlbumMeta};

type Aes128Ctr = ctr::Ctr128BE<aes::Aes128>;

// TODO: Remove
const URLS_REGEX_STRINGS: [&str; 3] = [
    r#"^https://music\.yandex\.(?:by|kz|ru)/album/(\d+)(?:/track/(\d+)(?:\?.+)?)?$"#,
    r#"^https://music\.yandex\.(?:by|kz|ru)/playlists/((?:[a-z]{2}\.|)[a-z\d]{8}-[a-z\d]{4}-[a-z\d]{4}-[a-z\d]{4}-[a-z\d]{12})$"#,
    r#"^https://music\.yandex\.(?:by|kz|ru)/artist/(\d+)(?:/albums)?(?:\?.+)?$"#,
];

pub fn get_exe_path() -> Result<PathBuf, Box<dyn Error>> {
    let exe_path = env::current_exe()?;
    let dir = exe_path.parent()
        .ok_or("failed to get path of executable")?
        .to_path_buf();
    Ok(dir)
}

pub fn resolve_ffmpeg_path(cfg_path: Option<PathBuf>, exe_dir: &PathBuf) -> PathBuf {
    let local_name = if cfg!(target_os = "windows") { "ffmpeg.exe" } else { "ffmpeg" };
    let local_path = exe_dir.join(local_name);
    if local_path.exists() { return local_path; }

    if let Some(p) = cfg_path {
        if p.exists() { return p; }
    }
    PathBuf::from("ffmpeg")
}

pub fn parse_url(url: &str) -> Option<MediaLink> {
    static RE_ALBUM_TRACK: OnceLock<Regex> = OnceLock::new();
    static RE_PLAYLIST: OnceLock<Regex> = OnceLock::new();
    static RE_ARTIST: OnceLock<Regex> = OnceLock::new();

    let re_album_track = RE_ALBUM_TRACK.get_or_init(|| {
        Regex::new(r"album/(\d+)(?:/track/(\d+))?").unwrap()
    });
    let re_playlist = RE_PLAYLIST.get_or_init(|| {
        Regex::new(r"playlists/([\w.-]+)").unwrap()
    });
    let re_artist = RE_ARTIST.get_or_init(|| {
        Regex::new(r"artist/(\d+)").unwrap()
    });

    if let Some(cap) = re_album_track.captures(url) {
        let album_id = cap.get(1)?.as_str().to_string();
        return if let Some(track_match) = cap.get(2) {
            Some(MediaLink::Track {
                album_id,
                track_id: track_match.as_str().to_string(),
            })
        } else {
            Some(MediaLink::Album { album_id })
        };
    }

    if let Some(cap) = re_playlist.captures(url) {
        return Some(MediaLink::Playlist {
            uuid_or_login: cap.get(1)?.as_str().to_string(),
        });
    }

    if let Some(cap) = re_artist.captures(url) {
        return Some(MediaLink::Artist {
            artist_id: cap.get(1)?.as_str().to_string(),
        });
    }
    None
}

pub fn sanitise(filename: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"[\\/:*?"><|]"#).unwrap()
    });

    let sanitised = re.replace_all(filename, "_");

    let result = sanitised.trim().trim_end_matches(".");

    if result.is_empty() {
        return "noname".to_string();
    }

    result.to_string()
}

pub fn decrypt_buff(
    buff: &mut [u8],
    key_hex: &str,
) -> Result<(), Box<dyn Error>> {
    let key_vec = hex::decode(key_hex)?;
    let key: [u8; 16] = key_vec.try_into().map_err(|_| "key must be 16 bytes")?;
    let nonce = [0u8; 16];

    let mut cipher = Aes128Ctr::new(&key.into(), &nonce.into());

    cipher.apply_keystream(buff);

    Ok(())
}

fn parse_template(template: &str, replacements: HashMap<&str, String>) -> String {
    let mut result = template.to_string();

    for (key, value) in replacements {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, &value);
    }

    sanitise(&result)
}

pub(crate) fn parse_album_template(template: &str, meta: &ParsedAlbumMeta) -> String {
    let m: HashMap<&str, String> = HashMap::from([
        ("album_artist", meta.album_artist.clone()),
        ("album_title", meta.album_title.clone()),
        ("label", meta.label.clone()),
        ("year", meta.year.map(|y| y.to_string()).unwrap_or_default()),
    ]);

    parse_template(template, m)
}

pub(crate) fn parse_track_template(
    template: &str,
    meta: &ParsedAlbumMeta,
    padding: &str,
) -> String {
    let m: HashMap<&str, String> = HashMap::from([
        ("track_num", meta.track_num.to_string()),
        ("track_num_pad", padding.to_string()),
        ("title", meta.title.clone()),
        ("artist", meta.artist.clone()),
    ]);

    parse_template(template, m)
}

fn contains(lines: &[String], value: &str) -> bool {
    lines.iter().any(|s| s.to_lowercase() == value.to_lowercase())
}

fn read_text_file_lines(filename: &str) -> Result<Vec<String>, IoError> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);

    let mut lines: Vec<String> = Vec::new();
    for result in br.lines() {
        match result {
            Ok(line) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    lines.push(trimmed.to_string());
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    Ok(lines)
}

pub fn clean_url(url: &str) -> String {
    let trimmed = url.trim();
    let stripped = trimmed.strip_suffix('/').unwrap_or(&trimmed);
    stripped.to_string()
}

pub fn process_urls(urls: &[String]) -> Result<Vec<String>, Box<dyn Error>> {
    let mut processed: Vec<String> = Vec::new();
    let mut text_paths: Vec<String> = Vec::new();

    for url in urls {
        if url.ends_with(".txt") {
            if contains(&text_paths, &url) {
                continue;
            }
            let text_lines = read_text_file_lines(&url)?;
            for text_line in text_lines {
                let cleaned_line = clean_url(&text_line);
                if !contains(&processed, &cleaned_line) {
                    processed.push(cleaned_line);
                }
            }
            text_paths.push(url.clone());
        } else {
            let cleaned_line = clean_url(&url);
            if !contains(&processed, &cleaned_line) {
                processed.push(cleaned_line);
            }
        }
    }

    Ok(processed)
}

pub fn file_exists(file_path: &PathBuf) -> Result<bool, IoError> {
    match fs::metadata(file_path) {
        Ok(meta) => Ok(meta.is_file()),
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                Ok(false)
            } else {
                Err(err)
            }
        }
    }
}


pub fn format_track_number(track_num: u16, track_total: u16) -> String {
    let padding = track_total.to_string().len();
    format!("{:0width$}", track_num, width = padding)
}
