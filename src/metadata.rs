use crate::api::structs::{AlbumResult, AlbumResultInPlaylist, Artist, Label, PlaylistTrack, Volume};
use crate::models::ParsedAlbumMeta;

pub fn parse_artists(artists: &[Artist]) -> String {
    artists.iter().map(|a| a.name.clone()).collect::<Vec<String>>().join(", ")
}

pub fn parse_labels(labels: &[Label]) -> String {
    labels.iter().map(|l| l.name.clone()).collect::<Vec<String>>().join(", ")
}

pub fn parse_title(title: &str, version: Option<String>) -> String {
    format!("{}{}", title, version.map_or("".to_string(), |v| format!(" ({})", v)))
}

pub fn parse_album_meta(meta: &AlbumResult, track_total: u16) -> ParsedAlbumMeta {
    ParsedAlbumMeta {
        album_artist: parse_artists(&meta.artists),
        album_title: parse_title(&meta.title, meta.version.clone()),
        artist: String::new(),
        cover_data: Vec::new(),
        genre: meta.genre.clone(),
        lyrics_avail: None,
        is_track_only: false,
        title: String::new(),
        track_num: 0,
        track_total,
        label: parse_labels(&meta.labels),
        timed_lyrics: None,
        untimed_lyrics: None,
        year: meta.year,
    }
}

pub fn parse_album_meta_playlist(meta: &AlbumResultInPlaylist, track_total: u16) -> ParsedAlbumMeta {
    ParsedAlbumMeta {
        album_artist: parse_artists(&meta.artists),
        album_title: parse_title(&meta.title, meta.version.clone()),
        artist: String::new(),
        cover_data: Vec::new(),
        genre: meta.genre.clone(),
        lyrics_avail: None,
        is_track_only: false,
        title: String::new(),
        track_num: 0,
        track_total,
        timed_lyrics: None,
        untimed_lyrics: None,
        label: parse_labels(&meta.labels),
        year: meta.year,
    }
}

pub fn parse_track_meta(meta: &mut ParsedAlbumMeta, track_meta: &Volume, track_num: u16, is_track_only: bool) {
    meta.artist = parse_artists(&track_meta.artists);
    meta.title = parse_title(&track_meta.title, track_meta.version.clone());
    meta.track_num = track_num;
    if let Some(lyrics) = &track_meta.lyrics_info {
        meta.lyrics_avail = lyrics.check_availibility();
    }
    meta.is_track_only = is_track_only;
}

pub fn parse_track_meta_playlist(meta: &mut ParsedAlbumMeta, track_meta: &PlaylistTrack, track_num: u16) {
    meta.artist = parse_artists(&track_meta.artists);
    meta.title = parse_title(&track_meta.title, track_meta.version.clone());
    meta.track_num = track_num;
    if let Some(lyrics) = &track_meta.lyrics_info {
        meta.lyrics_avail = lyrics.check_availibility();
    }
}

pub fn parse_specs(codec: &str, bitrate: u16) -> Option<(String, String)> {
    match codec {
        "flac-mp4" => Some(("FLAC".to_string(), "flac".to_string())),
        "mp3" | "mp3-mp4" => Some((format!("{} Kbps MP3", bitrate), "mp3".to_string())),
        "aac-mp4" | "he-aac-mp4" => Some((format!("{} Kbps AAC", bitrate), "m4a".to_string())),
        _ => None,
    }
}