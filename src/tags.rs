use std::error::Error;
use std::path::PathBuf;

use id3::frame::Picture as Mp3Image;
use id3::frame::PictureType::CoverFront as MP3CoverFront;
use id3::{Error as ID3Error, Tag as Mp3Tag, TagLike, Version};
use metaflac::block::PictureType::CoverFront as FLACCoverFront;
use metaflac::{Error as FlacError, Tag as FlacTag};
use mp4ameta::{Data as Mp4Data, Error as MP4Error, Fourcc, Tag as Mp4Tag};

use crate::models::ParsedAlbumMeta;

fn set_vorbis(tag: &mut metaflac::Tag, key: &str, value: &str) {
    if !value.is_empty() {
        tag.set_vorbis(key, vec![value]);
    }
}

fn set_vorbis_num(tag: &mut metaflac::Tag, key: &str, n: u16) {
    if n > 0 {
        tag.set_vorbis(key, vec![n.to_string()]);
    }
}

fn write_flac_tags(track_path: &PathBuf, meta: &ParsedAlbumMeta) -> Result<(), FlacError> {
    let mut tag = FlacTag::read_from_path(&track_path)?;

    set_vorbis(&mut tag, "ALBUM", &meta.album_title);
    set_vorbis(&mut tag, "ALBUMARTIST", &meta.album_artist);
    set_vorbis(&mut tag, "ARTIST", &meta.artist);
    set_vorbis(&mut tag, "LABEL", &meta.label);
    set_vorbis(&mut tag, "TITLE", &meta.title);
    set_vorbis_num(&mut tag, "TRACKNUMBER", meta.track_num);
    set_vorbis_num(&mut tag, "TRACKTOTAL", meta.track_total);

    if !meta.cover_data.is_empty() {
        tag.add_picture("image/jpeg", FLACCoverFront, meta.cover_data.clone());
    }

    if let Some(genre) = &meta.genre {
        set_vorbis(&mut tag, "GENRE", genre);
    }

    if let Some(year) = meta.year {
        set_vorbis_num(&mut tag, "YEAR", year);
    }

    if let Some(lyrics) = &meta.untimed_lyrics {
        set_vorbis(&mut tag, "UNSYNCEDLYRICS", lyrics);
    }

    if let Some(lyrics) = &meta.timed_lyrics {
        set_vorbis(&mut tag, "LYRICS", lyrics);
    }

    tag.save()?;
    Ok(())
}

fn write_mp3_tags(track_path: &PathBuf, meta: &ParsedAlbumMeta) -> Result<(), ID3Error> {
    let mut tag = Mp3Tag::new();

    tag.set_album(&meta.album_title);
    tag.set_album_artist(&meta.album_artist);
    tag.set_artist(&meta.artist);
    tag.set_title(&meta.title);
    tag.set_track(meta.track_num as u32);
    tag.set_total_tracks(meta.track_total as u32);

    if !meta.cover_data.is_empty() {
        let pic = Mp3Image {
            mime_type: "image/jpeg".to_string(),
            picture_type: MP3CoverFront,
            description: String::new(),
            data: meta.cover_data.clone(),
        };
        tag.add_frame(pic);
    }

    if let Some(genre) = &meta.genre {
        tag.set_genre(genre);
    }

    if let Some(year) = meta.year {
        tag.set_year(year as i32);
    }

    tag.write_to_path(track_path, Version::Id3v24)?;
    Ok(())
}

fn write_mp4_tags(track_path: &PathBuf, meta: &ParsedAlbumMeta) -> Result<(), MP4Error> {
    let mut tag = Mp4Tag::read_from_path(&track_path)?;

    tag.set_album(&meta.album_title);
    tag.set_album_artist(&meta.album_artist);
    tag.set_artist(&meta.artist);
    tag.set_title(&meta.title);
    tag.set_track(meta.track_num, meta.track_total);

    let covr = Fourcc(*b"covr");
    if !meta.cover_data.is_empty() {
        tag.add_data(covr, Mp4Data::Jpeg(meta.cover_data.clone()));
    }

    if let Some(genre) = &meta.genre {
        tag.set_genre(genre);
    }

    if let Some(year) = meta.year {
        tag.set_year(year.to_string());
    }

    if let Some(lyrics) = &meta.timed_lyrics {
        tag.set_lyrics(lyrics);
    } else if let Some(lyrics) = &meta.untimed_lyrics {
        tag.set_lyrics(lyrics);
    }

    tag.write_to_path(&track_path)?;
    Ok(())
}

pub fn write_tags(track_path: &PathBuf, codec: &str, meta: &ParsedAlbumMeta) -> Result<(), Box<dyn Error>> {
    match codec {
        "flac-mp4" => write_flac_tags(track_path, meta)?,
        "mp3" | "mp3-mp4" => write_mp3_tags(track_path, meta)?,
        "aac-mp4" | "he-aac-mp4" => write_mp4_tags(track_path, meta)?,
        _ => {}
    }
    Ok(())
}