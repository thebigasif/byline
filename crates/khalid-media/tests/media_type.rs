//! An upload's type comes from its bytes, never its name (RULES.md
//! BL-SEC-6). Each case is the first bytes of a real file of that type.

use khalid_media::{Kind, MediaType, Refusal};

#[cfg(test)]
fn assert_refused(heads: &[&[u8]], why: Refusal) {
    for head in heads {
        assert_eq!(MediaType::sniff(head), Err(why), "{head:?}");
    }
}

#[test]
fn knows_each_accepted_type_by_its_first_bytes() {
    let cases: [(&[u8], MediaType); 16] = [
        (b"\xFF\xD8\xFF\xE0\x00\x10JFIF", MediaType::Jpeg),
        (b"\x89PNG\r\n\x1A\n\x00\x00\x00\x0DIHDR", MediaType::Png),
        (b"RIFF\x24\x00\x00\x00WEBPVP8 ", MediaType::Webp),
        (b"\x00\x00\x00\x20ftypisom\x00\x00\x02\x00", MediaType::Mp4),
        (b"\x00\x00\x00\x18ftypmp42\x00\x00\x00\x00", MediaType::Mp4),
        (
            b"\x1A\x45\xDF\xA3\x9F\x42\x86\x81\x01\x42\x82\x84webm",
            MediaType::Webm,
        ),
        (b"ID3\x04\x00\x00\x00\x00\x00\x00", MediaType::Mp3),
        (b"\xFF\xFB\x90\x64\x00", MediaType::Mp3),
        (b"\x00\x00\x00\x20ftypM4A \x00\x00\x00\x00", MediaType::M4a),
        (b"OggS\x00\x02\x00\x00", MediaType::Ogg),
        (b"RIFF\x24\x08\x00\x00WAVEfmt ", MediaType::Wav),
        (b"fLaC\x00\x00\x00\x22", MediaType::Flac),
        (b"%PDF-1.7\n%\xE2\xE3\xCF\xD3", MediaType::Pdf),
        (b"PK\x03\x04\x14\x00", MediaType::Zip),
        (
            "Keyword research\n\t1. Seed terms\r\n".as_bytes(),
            MediaType::Text,
        ),
        (
            "\u{feff}title,clicks\nchoosing a crm,120\n".as_bytes(),
            MediaType::Text,
        ),
    ];
    for (head, want) in cases {
        assert_eq!(MediaType::sniff(head), Ok(want), "{head:?}");
    }
}

#[test]
fn refuses_programs_whatever_their_name() {
    let programs: [&[u8]; 6] = [
        b"\x7FELF\x02\x01\x01\x00",
        b"MZ\x90\x00\x03\x00\x00\x00",
        b"\xCF\xFA\xED\xFE\x07\x00\x00\x01",
        b"\xCA\xFE\xBA\xBE\x00\x00\x00\x34",
        b"\x00asm\x01\x00\x00\x00",
        b"#!/bin/sh\necho hello\n",
    ];
    assert_refused(&programs, Refusal::Program);
}

#[test]
fn refuses_html_svg_and_xml() {
    let markup: [&[u8]; 4] = [
        b"<!doctype html><title>Choosing a CRM</title>",
        b"  <svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>",
        b"<?xml version=\"1.0\"?><rss/>",
        b"\xEF\xBB\xBF\n<html><body onload=\"steal()\">",
    ];
    assert_refused(&markup, Refusal::Markup);
}

#[test]
fn refuses_empty_files_and_unknown_types() {
    assert_refused(&[b""], Refusal::Empty);
    let unknown: [&[u8]; 5] = [
        b"GIF89a\x01\x00\x01\x00",
        b"\x00\x00\x00\x1Cftypheic\x00\x00\x00\x00",
        b"\x00\x00\x00\x14ftypqt  \x00\x00\x00\x00",
        b"\x1A\x45\xDF\xA3\x9F\x42\x86\x81\x01\x42\x82\x88matroska",
        b"plain words\x00then a zero byte",
    ];
    assert_refused(&unknown, Refusal::Unknown);
}

#[test]
fn each_type_has_one_kind_extension_and_mime_type() {
    assert_eq!(MediaType::Webm.kind(), Kind::Video);
    assert_eq!(MediaType::Flac.kind(), Kind::Audio);
    assert_eq!(MediaType::Pdf.kind(), Kind::File);
    assert_eq!(MediaType::Jpeg.extension(), "jpg");
    assert_eq!(MediaType::M4a.mime(), "audio/mp4");
    for mime in MediaType::ALL.map(MediaType::mime) {
        assert_eq!(mime.parse::<MediaType>().map(MediaType::mime), Ok(mime));
    }
    assert!("image/svg+xml".parse::<MediaType>().is_err());
}

#[test]
fn each_kind_has_its_size_limit() {
    const MB: u64 = 1024 * 1024;
    assert_eq!(Kind::Audio.max_bytes(), 50 * MB);
    assert_eq!(Kind::Image.check_size(20 * MB), Ok(()));
    let too_large = Kind::Video.check_size(100 * MB + 1).unwrap_err();
    assert_eq!(
        too_large.to_string(),
        "the file is larger than 100 MB, the most a video may be"
    );
}
