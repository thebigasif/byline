//! What a key may do: the key kinds, the scopes from
//! `docs/standards/api.md`, and the staff roles from PRD KH-EDT-7.

use khalid_api::keys::{Access, KeyKind, KeyToken, Role, Scope};

#[test]
fn prefixes_name_the_kind() {
    let secret = "aaaabbbbccccddddeeeeffff00001111aaaabbbbccccddddeeeeffff00001111";
    for (prefix, kind) in [
        ("kc", KeyKind::Content),
        ("ki", KeyKind::Integration),
        ("ks", KeyKind::Staff),
    ] {
        let token: KeyToken = format!("{prefix}_1111222233334444_{secret}")
            .parse()
            .unwrap();
        assert_eq!(token.kind(), kind, "{prefix}");
    }
}

#[test]
fn scopes_read_and_write_the_names_from_the_api_standard() {
    let names = [
        "content:read",
        "posts:write",
        "media:write",
        "settings:write",
        "webhooks:manage",
        "subscribers:write",
        "admin:all",
    ];
    let read: Vec<&str> = names
        .iter()
        .map(|name| name.parse::<Scope>().unwrap().as_str())
        .collect();
    assert_eq!(read, names);
    let error = "posts:delete".parse::<Scope>().unwrap_err();
    assert_eq!(error.to_string(), "posts:delete is not a key scope");
}

#[test]
fn an_integration_key_holds_only_its_scopes() {
    let access = Access::Integration(vec![Scope::PostsWrite, Scope::MediaWrite]);
    assert!(access.allows(Scope::PostsWrite));
    assert!(access.allows(Scope::MediaWrite));
    assert!(!access.allows(Scope::SettingsWrite));
    assert!(!access.allows(Scope::ContentRead));
}

#[test]
fn admin_all_and_owner_allow_everything_and_content_keys_only_read() {
    let all = Access::Integration(vec![Scope::AdminAll]);
    for scope in Scope::ALL {
        assert!(all.allows(scope), "{scope:?}");
        assert!(Access::Staff(Role::Owner).allows(scope), "{scope:?}");
        assert!(Access::Staff(Role::Admin).allows(scope), "{scope:?}");
        let content = scope == Scope::ContentRead;
        assert_eq!(Access::Content.allows(scope), content, "{scope:?}");
    }
}

#[test]
fn roles_below_admin_write_posts_but_not_settings() {
    for role in [Role::Editor, Role::Writer, Role::Contributor] {
        let access = Access::Staff(role);
        assert!(access.allows(Scope::PostsWrite), "{role:?}");
        assert!(!access.allows(Scope::SettingsWrite), "{role:?}");
        assert!(!access.allows(Scope::WebhooksManage), "{role:?}");
    }
    assert!(Access::Staff(Role::Writer).allows(Scope::MediaWrite));
    assert!(!Access::Staff(Role::Contributor).allows(Scope::MediaWrite));
}
