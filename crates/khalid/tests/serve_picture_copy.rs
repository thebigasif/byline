//! Turning and cropping a library picture on a running `khalid serve`: the
//! copy is served from the blog and the original stays (plan 4.6, KH-EDT-1,
//! decisions E9 and E10).

mod common;

use std::io::Cursor;

use image::{ImageFormat, Rgb, RgbImage};
use serde_json::{Value, json};

#[test]
fn a_turned_copy_is_served_beside_the_original() {
    let site = common::demo_copy();
    let key = common::new_key(site.path(), &["--name", "Maria", "--scope", "media:write"]);
    let server = common::start(site.path(), &key);
    let mut png = Cursor::new(Vec::new());
    let photo = RgbImage::from_pixel(300, 100, Rgb([20, 90, 160]));
    photo.write_to(&mut png, ImageFormat::Png).unwrap();
    let auth = format!("Khalid {key}");
    let headers = [("Authorization", auth.as_str())];
    let upload = "/api/admin/v1/media/?name=Team%20photo.png";
    let picture = (png.get_ref().as_slice(), Some("application/octet-stream"));
    let (status, body) = server.send("POST", upload, &headers, picture);
    assert_eq!(status, 201, "{body}");
    let original: Value = serde_json::from_str(&body).unwrap();
    let id = original["media"][0]["id"].as_str().unwrap();

    let copies = format!("/api/admin/v1/media/{id}/copies/");
    let (status, made) = server.admin("POST", &copies, Some(&json!({ "turn": 270 })));
    assert_eq!(status, 201, "{made}");
    let copy = &made["media"][0];
    let turned = (copy["width"].as_u64(), copy["height"].as_u64());
    assert_eq!(turned, (Some(100), Some(300)));
    let url = copy["url"].as_str().unwrap();
    let path = url.strip_prefix("https://www.example.com").unwrap();
    assert!(server.answers(path, 200), "the copy is served at {path}");
    let (status, kept) = server.admin("GET", &format!("/api/admin/v1/media/{id}/"), None);
    assert_eq!(status, 200, "{kept}");
    assert_eq!(kept["media"][0]["width"], 300, "the original stays");
}
