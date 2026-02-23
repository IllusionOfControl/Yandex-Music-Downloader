use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DownloadFormat {
    Aac64,  // 1 -> "lq"
    Aac192, // 2 -> "nq"
    Hq,     // 3 -> "hq" (AAC 256 / MP3 320)
    Flac,   // 4 -> "lossless"
}

impl DownloadFormat {
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            1 => Some(Self::Aac64),
            2 => Some(Self::Aac192),
            3 => Some(Self::Hq),
            4 => Some(Self::Flac),
            _ => None,
        }
    }

    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Aac64 => "lq",
            Self::Aac192 => "nq",
            Self::Hq => "hq",
            Self::Flac => "lossless",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MediaLink {
    Album { album_id: String },
    Track { album_id: String, track_id: String },
    Playlist { uuid_or_login: String },
    Artist { artist_id: String },
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub token: String,

    pub format: DownloadFormat,
    pub out_path: PathBuf,
    pub ffmpeg_path: PathBuf,

    pub keep_covers: bool,
    pub write_covers: bool,
    pub get_original_covers: bool,
    pub write_lyrics: bool,

    pub album_template: String,
    pub track_template: String,

    pub sleep: bool,

    pub media_links: Vec<MediaLink>,
}

#[derive(Debug, Clone)]
pub struct ParsedAlbumMeta {
    pub album_title: String,
    pub album_artist: String,
    pub artist: String,
    pub cover_data: Vec<u8>,
    pub genre: Option<String>,
    pub lyrics_avail: Option<bool>,
    pub is_track_only: bool,
    pub label: String,
    pub title: String,
    pub timed_lyrics: Option<String>,
    pub untimed_lyrics: Option<String>,
    pub track_num: u16,
    pub track_total: u16,
    pub year: Option<u16>,
}
