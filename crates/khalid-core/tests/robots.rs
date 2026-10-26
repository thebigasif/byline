//! Robots directives read from the text a writer puts in front matter, the
//! same words as a `<meta name="robots">` tag (KH-SEO-9).

use khalid_core::{
    Following, ImagePreview, Indexing, ParseRobotsError, PreviewLimit, Restriction,
    RobotsDirectives,
};

#[cfg(test)]
fn parse(text: &str) -> Result<RobotsDirectives, ParseRobotsError> {
    text.parse()
}

#[test]
fn reads_back_what_it_prints() {
    let text = "index, follow, noarchive, notranslate, max-snippet:160, \
                max-image-preview:standard, max-video-preview:0";
    let directives = parse(text).unwrap();
    let mut want = RobotsDirectives::default();
    want.restrictions = [Restriction::NoArchive, Restriction::NoTranslate].into();
    want.max_snippet = PreviewLimit::AtMost(160);
    want.max_image_preview = ImagePreview::Standard;
    want.max_video_preview = PreviewLimit::AtMost(0);
    assert_eq!(directives, want);
    assert_eq!(directives.to_string(), text);
}

#[test]
fn a_short_list_keeps_the_defaults_for_the_rest() {
    let directives = parse("noindex").unwrap();
    assert_eq!(directives.indexing, Indexing::NoIndex);
    assert_eq!(directives.following, Following::Follow);
    assert_eq!(directives.to_string(), "noindex, follow");
    assert_eq!(
        parse("nosnippet, max-snippet:-1").unwrap().to_string(),
        "index, follow, nosnippet, max-snippet:-1, max-image-preview:large, max-video-preview:-1"
    );
}

#[test]
fn ignores_case_and_spaces_and_knows_all_and_none() {
    assert_eq!(
        parse("  NoIndex ,NOFOLLOW ").unwrap(),
        parse("noindex, nofollow").unwrap()
    );
    assert_eq!(parse("none").unwrap(), parse("noindex, nofollow").unwrap());
    assert_eq!(parse("all").unwrap(), RobotsDirectives::default());
    assert_eq!(
        parse("noimageindex, nosnippet").unwrap().restrictions,
        [Restriction::NoSnippet, Restriction::NoImageIndex].into()
    );
}

#[test]
fn refuses_a_word_it_does_not_know_and_names_it() {
    let error = parse("noindex, nofolow").unwrap_err();
    assert_eq!(
        error,
        ParseRobotsError::Unknown {
            word: "nofolow".to_owned()
        }
    );
    let message = error.to_string();
    assert!(message.contains("\"nofolow\""), "{message}");
    assert!(message.contains("nofollow"), "{message}");
}

#[test]
fn refuses_directives_that_contradict_each_other() {
    assert_eq!(
        parse("index, noindex").unwrap_err(),
        ParseRobotsError::Contradicts {
            first: "index".to_owned(),
            second: "noindex".to_owned()
        }
    );
    assert!(matches!(
        parse("none, follow").unwrap_err(),
        ParseRobotsError::Contradicts { .. }
    ));
    assert_eq!(
        parse("max-snippet:1, max-snippet:2").unwrap_err(),
        ParseRobotsError::Repeated {
            name: "max-snippet".to_owned()
        }
    );
}

#[test]
fn refuses_a_limit_or_a_size_it_cannot_use() {
    for (text, directive, value) in [
        ("max-snippet:-2", "max-snippet", "-2"),
        ("max-video-preview:lots", "max-video-preview", "lots"),
        ("max-image-preview:huge", "max-image-preview", "huge"),
        ("max-snippet", "max-snippet", ""),
    ] {
        assert_eq!(
            parse(text).unwrap_err(),
            ParseRobotsError::BadValue {
                directive: directive.to_owned(),
                value: value.to_owned()
            },
            "{text}"
        );
    }
}

#[test]
fn refuses_an_empty_list() {
    assert_eq!(parse("").unwrap_err(), ParseRobotsError::Empty);
    assert_eq!(parse(" , ").unwrap_err(), ParseRobotsError::Empty);
}

#[test]
fn reads_from_a_settings_or_front_matter_string() {
    let directives: RobotsDirectives = serde_json::from_str("\"noindex, noarchive\"").unwrap();
    assert_eq!(directives, parse("noindex, noarchive").unwrap());
    let error = serde_json::from_str::<RobotsDirectives>("\"maybe\"").unwrap_err();
    assert!(error.to_string().contains("\"maybe\""), "{error}");
    let indexing: Indexing = serde_json::from_str("\"noindex\"").unwrap();
    assert_eq!(indexing, Indexing::NoIndex);
}
