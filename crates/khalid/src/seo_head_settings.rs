//! `[seo.social]` and `[seo.verification]` from `site.toml`: the Twitter
//! account, the default share image, and the codes and files that prove the
//! site is the owner's (audit G11, KH-SEO-2, KH-SEO-7).

use khalid_core::{ImageSettings, SocialSettings, VerificationSettings};
use khalid_seo::{RootFile, SeoSite, ShareImage, TwitterHandle, VerificationCode};

use crate::seo_settings::SettingMistake;

/// What a Twitter handle may be, for the fix.
const HANDLE: &str = "up to 15 letters, digits and _, such as @serpdad";

/// What a verification code may be, for the fix.
const CODE: &str = "the code the search engine gave, such as aBc123";

/// Checks the Twitter account and the default share image and sets them on `site`.
///
/// # Errors
///
/// The first value `khalid-seo` refuses.
pub(crate) fn social(settings: &SocialSettings, site: &mut SeoSite) -> Result<(), SettingMistake> {
    if let Some(handle) = &settings.twitter {
        let handle = TwitterHandle::try_new(handle)
            .map_err(|error| SettingMistake::new("seo.social", "twitter", &error, Some(HANDLE)))?;
        site.twitter_site = Some(handle);
    }
    if let Some(image) = &settings.image {
        site.share_image = Some(share_image("seo.social.image", image)?);
    }
    Ok(())
}

/// The image in `table`, refused when it has no alternative text or no size.
///
/// # Errors
///
/// A mistake on `alt`, `width` or `height`.
pub(crate) fn share_image(
    table: &str,
    image: &ImageSettings,
) -> Result<ShareImage, SettingMistake> {
    if image.alt.trim().is_empty() {
        let expected = "one short sentence that says what the image shows";
        return Err(SettingMistake::new(
            table,
            "alt",
            &"the image has no alternative text",
            Some(expected),
        ));
    }
    for (key, pixels) in [("width", image.width), ("height", image.height)] {
        if pixels == 0 {
            let problem = format!("the image's {key} is 0 pixels");
            return Err(SettingMistake::new(
                table,
                key,
                &problem,
                Some("its size in pixels, such as 1200"),
            ));
        }
    }
    Ok(ShareImage::new(
        image.url.clone(),
        image.width,
        image.height,
        image.alt.trim(),
    ))
}

/// Checks each verification code and file and sets them on `site`.
///
/// # Errors
///
/// The first code or file name `khalid-seo` refuses.
pub(crate) fn verification(
    settings: &VerificationSettings,
    site: &mut SeoSite,
) -> Result<(), SettingMistake> {
    let codes = &mut site.verification;
    let slots = [
        ("google", &settings.google, &mut codes.google),
        ("bing", &settings.bing, &mut codes.bing),
        ("yandex", &settings.yandex, &mut codes.yandex),
        ("baidu", &settings.baidu, &mut codes.baidu),
        ("pinterest", &settings.pinterest, &mut codes.pinterest),
    ];
    for (key, code, slot) in slots {
        if let Some(code) = code {
            let code = VerificationCode::try_new(code).map_err(|error| {
                SettingMistake::new("seo.verification", key, &error, Some(CODE))
            })?;
            *slot = Some(code);
        }
    }
    for (name, body) in &settings.files {
        let file = RootFile::try_new(name, body)
            .map_err(|error| SettingMistake::new("seo.verification.files", name, &error, None))?;
        site.root_files.push(file);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::seo_settings::apply;
    use crate::seo_settings::testing::{settings, site};

    #[test]
    fn a_share_image_needs_alt_text_and_a_size() {
        let image = "[seo.social.image]\nurl = \"https://www.example.com/a.png\"\n";
        let no_alt = apply(&settings(&format!("{image}alt = \" \"\n")), &mut site()).unwrap_err();
        assert_eq!(
            (no_alt.table.as_str(), no_alt.key.as_str()),
            ("seo.social.image", "alt")
        );
        let flat = format!("{image}alt = \"A card\"\nheight = 0\n");
        let no_height = apply(&settings(&flat), &mut site()).unwrap_err();
        assert_eq!(
            no_height.problem,
            "the image's height is 0 pixels, expected its size in pixels, such as 1200"
        );
    }

    #[test]
    fn a_verification_code_with_a_space_is_refused() {
        let error = apply(
            &settings("[seo.verification]\nbaidu = \"a b\"\n"),
            &mut site(),
        )
        .unwrap_err();
        assert_eq!(error.key, "baidu");
        assert!(
            error
                .problem
                .ends_with("expected the code the search engine gave, such as aBc123"),
            "{}",
            error.problem
        );
    }
}
