//! The permission table in the crate's README is the one the code grants,
//! role by role, and every role boundary holds.

use khalid_api::keys::{Access, Role, Scope};
use khalid_auth::{Permission, may_assign, may_manage};

const README: &str = include_str!("../README.md");

/// The rows of the README table whose first cell is a permission name in
/// backticks, as the name and its five cells, owner first.
fn readme_rows(start: &str) -> Vec<(String, Vec<String>)> {
    let Some((_, table)) = README.split_once(start) else {
        return Vec::new();
    };
    table
        .lines()
        .skip(1)
        .take_while(|line| line.starts_with('|'))
        .filter_map(|line| {
            let cells: Vec<String> = line
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_owned())
                .collect();
            let (first, rest) = cells.split_first()?;
            let name = first.strip_prefix('`')?.strip_suffix('`')?;
            Some((name.to_owned(), rest.to_vec()))
        })
        .collect()
}

/// `yes` or `no` for each role, owner first, as the README writes them.
fn cells(granted: impl Fn(Role) -> bool) -> Vec<&'static str> {
    Role::ALL
        .into_iter()
        .map(|role| if granted(role) { "yes" } else { "no" })
        .collect()
}

#[test]
fn the_readme_table_lists_every_permission_once_with_what_each_role_gets() {
    let rows = readme_rows("| Permission |");
    let names: Vec<&str> = rows.iter().map(|(name, _)| name.as_str()).collect();
    let want: Vec<&str> = Permission::ALL.iter().map(|p| p.as_str()).collect();
    assert_eq!(
        names, want,
        "the README lists the permissions in code order"
    );
    for (name, row) in &rows {
        let permission: Permission = name.parse().expect("a permission name");
        let granted = cells(|role| permission.granted_to(role));
        assert_eq!(
            row, &granted,
            "{name}: owner, admin, editor, writer, contributor"
        );
    }
}

#[test]
fn the_readme_table_says_which_roles_each_role_may_manage() {
    let rows = readme_rows("| Manages |");
    let names: Vec<&str> = rows.iter().map(|(name, _)| name.as_str()).collect();
    let want: Vec<&str> = Role::ALL.iter().map(|role| role.as_str()).collect();
    assert_eq!(names, want, "one row per role, owner first");
    for (name, row) in &rows {
        let actor: Role = name.parse().expect("a role name");
        let managed = cells(|target| may_manage(actor, target));
        assert_eq!(
            row, &managed,
            "{name} manages owner, admin, editor, writer, contributor"
        );
    }
}

#[test]
fn nobody_manages_the_owner_and_nobody_below_editor_manages_anyone() {
    for actor in Role::ALL {
        assert!(
            !may_manage(actor, Role::Owner),
            "{actor:?} manages the owner"
        );
    }
    for actor in [Role::Writer, Role::Contributor] {
        for target in Role::ALL {
            assert!(!may_manage(actor, target), "{actor:?} manages {target:?}");
        }
    }
}

#[test]
fn a_role_change_needs_the_right_to_manage_both_the_old_and_the_new_role() {
    assert!(may_assign(Role::Admin, Role::Editor, Role::Writer));
    assert!(may_assign(Role::Editor, Role::Contributor, Role::Writer));
    assert!(
        !may_assign(Role::Editor, Role::Writer, Role::Editor),
        "an editor cannot make another editor"
    );
    assert!(
        !may_assign(Role::Editor, Role::Editor, Role::Writer),
        "an editor cannot demote another editor"
    );
    assert!(
        !may_assign(Role::Admin, Role::Owner, Role::Admin),
        "the owner is changed only by a transfer"
    );
    assert!(
        !may_assign(Role::Owner, Role::Admin, Role::Owner),
        "nobody is made owner by a role change"
    );
}

#[test]
fn a_staff_token_and_a_signed_in_account_with_the_same_role_may_do_the_same() {
    let pairs = [
        (Scope::PostsWrite, Permission::WriteOwnPosts),
        (Scope::MediaWrite, Permission::UploadMedia),
        (Scope::SettingsWrite, Permission::ManageSettings),
        (Scope::WebhooksManage, Permission::ManageIntegrations),
        (Scope::SubscribersWrite, Permission::ManageSubscribers),
    ];
    for role in Role::ALL {
        for (scope, permission) in pairs {
            assert_eq!(
                Access::Staff(role).allows(scope),
                permission.granted_to(role),
                "{role:?}: {} and {}",
                scope.as_str(),
                permission.as_str()
            );
        }
    }
}

#[test]
fn every_permission_name_reads_back() {
    for permission in Permission::ALL {
        assert_eq!(
            permission.as_str().parse::<Permission>().ok(),
            Some(permission)
        );
    }
    assert!("posts:everything".parse::<Permission>().is_err());
}
