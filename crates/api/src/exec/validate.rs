//! Validates edited exec information before it's saved.

use super::{Holder, Role};

pub fn roles(roles: Vec<Role>) -> Result<Vec<Role>, String> {
    let mut ids = std::collections::HashSet::new();
    let mut tidied = Vec::with_capacity(roles.len());
    for role in roles {
        let role = self::role(role)?;
        if !ids.insert(role.id.clone()) {
            return Err(format!("Two roles have the ID \"{}\".", role.id));
        }
        tidied.push(role);
    }
    Ok(tidied)
}

fn role(role: Role) -> Result<Role, String> {
    let title = role.title.trim().to_owned();
    if title.is_empty() {
        return Err("Every role needs a title.".to_owned());
    }
    // Name the role in every other message, so the editor can find it.
    let fail = |problem: &str| Err(format!("{title}: {problem}"));

    let id = role.id.trim().to_ascii_lowercase();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return fail("the ID must be letters, digits and hyphens.");
    }

    let emails = non_blank(
        role.emails
            .iter()
            .map(|email| email.trim().to_ascii_lowercase()),
    );
    if emails.is_empty() {
        return fail("it needs an email address.");
    }
    if let Some(email) = emails.iter().find(|email| !is_email(email)) {
        return fail(&format!("\"{email}\" isn't an email address."));
    }

    let holders = non_blank(
        role.holders
            .iter()
            .map(|holder| holder.name.trim().to_owned()),
    )
    .into_iter()
    .map(|name| Holder { name })
    .collect();

    let bio = role
        .bio
        .map(|bio| bio.trim().to_owned())
        .filter(|bio| !bio.is_empty());

    let photo_url = role
        .photo_url
        .map(|url| url.trim().to_owned())
        .filter(|url| !url.is_empty());

    // Photos are uploaded through the console, so they can only be in our bucket.
    if let Some(url) = &photo_url
        && !super::photo::is_photo_url(url)
    {
        return fail("the photo must be uploaded via the admin console page.");
    }

    let instagram = non_blank(
        role.instagram
            .iter()
            .map(|handle| handle.trim().trim_start_matches('@').to_owned()),
    );
    if let Some(handle) = instagram.iter().find(|handle| !is_instagram_handle(handle)) {
        return fail(&format!("\"{handle}\" isn't an Instagram handle."));
    }

    Ok(Role {
        id,
        title,
        emails,
        holders,
        bio,
        photo_url,
        instagram,
    })
}

/// Filter out empty strings, keeping the order.
fn non_blank(items: impl Iterator<Item = String>) -> Vec<String> {
    items.filter(|item| !item.is_empty()).collect()
}

/// A loose check: one "@" with something either side, and no spaces.
fn is_email(email: &str) -> bool {
    !email.chars().any(char::is_whitespace)
        && matches!(email.split_once('@'), Some((local, domain))
            if !local.is_empty() && domain.contains('.') && !domain.contains('@'))
}

/// Up to 30 letters, digits, dots and underscores.
fn is_instagram_handle(handle: &str) -> bool {
    !handle.is_empty()
        && handle.len() <= 30
        && handle
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHOTO: &str = "https://storage.googleapis.com/kcsu-admin-public/exec/president.webp";

    /// A role with every field set, already tidy.
    fn president() -> Role {
        Role {
            id: "president".to_owned(),
            title: "President".to_owned(),
            emails: vec!["president@kcsu.org.uk".to_owned()],
            holders: vec![Holder {
                name: "A Person".to_owned(),
            }],
            bio: Some("Hello".to_owned()),
            photo_url: Some(PHOTO.to_owned()),
            instagram: vec!["kcsu".to_owned()],
        }
    }

    fn holder(name: &str) -> Holder {
        Holder {
            name: name.to_owned(),
        }
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|&item| item.to_owned()).collect()
    }

    #[test]
    fn keeps_a_tidy_role_as_it_is() {
        assert_eq!(roles(vec![president()]), Ok(vec![president()]));
    }

    #[test]
    fn needs_only_a_title_an_id_and_an_email() {
        let minimal = Role {
            id: "welfare".to_owned(),
            title: "Welfare".to_owned(),
            emails: strings(&["welfare@kcsu.org.uk"]),
            ..Role::default()
        };
        assert_eq!(roles(vec![minimal.clone()]), Ok(vec![minimal]));
    }

    #[test]
    fn tidies_whitespace_case_and_blank_entries() {
        let messy = Role {
            id: " President ".to_owned(),
            title: "  President ".to_owned(),
            emails: strings(&[" President@KCSU.org.uk ", "", "  "]),
            holders: vec![holder(" A Person "), holder("  ")],
            bio: Some(" Hello\n".to_owned()),
            photo_url: Some(format!(" {PHOTO} ")),
            instagram: strings(&["@kcsu", " ", ""]),
        };
        assert_eq!(roles(vec![messy]), Ok(vec![president()]));
    }

    #[test]
    fn blank_bio_and_photo_become_none() {
        let tidied = roles(vec![Role {
            bio: Some(" \n ".to_owned()),
            photo_url: Some(" ".to_owned()),
            ..president()
        }])
        .unwrap();
        assert_eq!(tidied[0].bio, None);
        assert_eq!(tidied[0].photo_url, None);
    }

    #[test]
    fn keeps_several_holders_emails_and_handles_in_order() {
        let shared = Role {
            emails: strings(&["b@kcsu.org.uk", "a@kcsu.org.uk"]),
            holders: vec![holder("Second"), holder("First")],
            instagram: strings(&["zz", "aa"]),
            ..president()
        };
        assert_eq!(roles(vec![shared.clone()]), Ok(vec![shared]));
    }

    #[test]
    fn keeps_roles_in_order() {
        let welfare = Role {
            id: "welfare".to_owned(),
            title: "Welfare".to_owned(),
            ..president()
        };
        let tidied = roles(vec![welfare, president()]).unwrap();
        let ids: Vec<&str> = tidied.iter().map(|role| role.id.as_str()).collect();
        assert_eq!(ids, ["welfare", "president"]);
    }

    #[test]
    fn allows_a_vacant_role() {
        let vacant = Role {
            holders: vec![],
            ..president()
        };
        assert!(roles(vec![vacant]).is_ok());
    }

    #[test]
    fn explains_what_is_wrong_and_with_which_role() {
        let cases = [
            (
                Role {
                    title: " ".to_owned(),
                    ..president()
                },
                "Every role needs a title.",
            ),
            (
                Role {
                    id: "has space".to_owned(),
                    ..president()
                },
                "President: the ID must be letters, digits and hyphens.",
            ),
            (
                Role {
                    emails: strings(&[" "]),
                    ..president()
                },
                "President: it needs an email address.",
            ),
            (
                Role {
                    emails: strings(&["president@kcsu.org.uk", "Nope"]),
                    ..president()
                },
                // As tidied, so lowercase.
                "President: \"nope\" isn't an email address.",
            ),
            (
                Role {
                    photo_url: Some("https://example.com/a.jpg".to_owned()),
                    ..president()
                },
                "President: the photo must be uploaded via the admin console page.",
            ),
            (
                Role {
                    instagram: strings(&["@has space"]),
                    ..president()
                },
                "President: \"has space\" isn't an Instagram handle.",
            ),
        ];
        for (role, message) in cases {
            assert_eq!(roles(vec![role]), Err(message.to_owned()));
        }
    }

    #[test]
    fn rejects_invalid_ids() {
        for id in ["", "  ", "has space", "under_score", "slash/es", "é"] {
            let role = Role {
                id: id.to_owned(),
                ..president()
            };
            assert!(roles(vec![role]).is_err(), "{id:?}");
        }
    }

    #[test]
    fn rejects_photos_from_anywhere_but_our_bucket() {
        for url in [
            "https://example.com/a.jpg",
            "https://storage.googleapis.com/another-bucket/exec/a.webp",
            "https://storage.googleapis.com/kcsu-admin-public/exec/../a.webp",
            "javascript:alert(1)",
        ] {
            let role = Role {
                photo_url: Some(url.to_owned()),
                ..president()
            };
            assert!(roles(vec![role]).is_err(), "{url}");
        }
    }

    #[test]
    fn rejects_duplicate_ids_after_tidying() {
        let other = Role {
            id: " President ".to_owned(),
            title: "Also President".to_owned(),
            ..president()
        };
        assert_eq!(
            roles(vec![president(), other]),
            Err("Two roles have the ID \"president\".".to_owned())
        );
    }

    #[test]
    fn checks_emails_loosely() {
        for email in [
            "president@kcsu.org.uk",
            "first.last+exec@kcsu.org.uk",
            "abc123@cam.ac.uk",
        ] {
            assert!(is_email(email), "{email}");
        }
        for email in [
            "",
            "president",
            "@kcsu.org.uk",
            "president@",
            "president@kcsu",
            "pres ident@kcsu.org.uk",
            "a@b@kcsu.org.uk",
        ] {
            assert!(!is_email(email), "{email}");
        }
    }

    #[test]
    fn checks_instagram_handles() {
        for handle in ["kcsu", "kings_college.su", "kcsu2026", &"a".repeat(30)] {
            assert!(is_instagram_handle(handle), "{handle}");
        }
        for handle in ["", "has space", "kcsu!", "kcsu-su", "café", &"a".repeat(31)] {
            assert!(!is_instagram_handle(handle), "{handle}");
        }
    }
}
