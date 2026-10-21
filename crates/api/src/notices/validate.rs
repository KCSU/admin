//! Validates edited status page notices before they're saved.

use std::collections::HashSet;

use uuid::Uuid;

use super::{Notice, Notices, Update};

pub fn notices(notices: Notices) -> Result<Notices, String> {
    let mut ids = HashSet::new();
    let mut tidied = Vec::with_capacity(notices.notices.len());
    for notice in notices.notices {
        let notice = self::notice(notice)?;
        if !ids.insert(notice.id.clone()) {
            return Err(format!("{}: another notice has the same ID.", notice.title));
        }
        tidied.push(notice);
    }
    // Newest first.
    tidied.sort_by_key(|notice| std::cmp::Reverse(notice.starts_at));
    Ok(Notices { notices: tidied })
}

fn notice(notice: Notice) -> Result<Notice, String> {
    let title = notice.title.trim().to_owned();
    if title.is_empty() {
        return Err("Every incident needs a title.".to_owned());
    }

    let fail = |problem: &str| Err(format!("{title}: {problem}"));

    // New notices get a new UUIDv7 here. Others keep theirs.
    let id = match notice.id.trim() {
        "" => Uuid::now_v7(),
        id => match Uuid::try_parse(id) {
            Ok(id) => id,
            Err(_) => return fail("the ID must be a UUID."),
        },
    }
    .to_string();

    let body = notice
        .body
        .map(|body| body.trim().to_owned())
        .filter(|body| !body.is_empty());

    let mut site_ids = Vec::with_capacity(notice.site_ids.len());
    for site_id in notice.site_ids {
        if !uptime::sites::SITES.iter().any(|site| site.id == site_id) {
            return fail(&format!("there's no site \"{site_id}\"."));
        }
        if !site_ids.contains(&site_id) {
            site_ids.push(site_id);
        }
    }

    if notice.starts_at <= 0 {
        return fail("it has no start time.");
    }
    if notice
        .ends_at
        .is_some_and(|ends_at| ends_at < notice.starts_at)
    {
        return fail("it can't end before it starts.");
    }

    let mut updates = Vec::with_capacity(notice.updates.len());
    for update in notice.updates {
        let text = update.text.trim().to_owned();
        if text.is_empty() {
            return fail("an update is empty.");
        }
        if update.at <= 0 {
            return fail("an update has no time.");
        }
        updates.push(Update {
            at: update.at,
            text,
        });
    }
    updates.sort_by_key(|update| update.at);

    Ok(Notice {
        id,
        kind: notice.kind,
        title,
        body,
        site_ids,
        starts_at: notice.starts_at,
        ends_at: notice.ends_at,
        updates,
    })
}

#[cfg(test)]
mod tests {
    use super::super::Kind;
    use super::*;

    const ID: &str = "01a15a90-e640-77f3-acd4-3d47c9267852";

    fn incident() -> Notice {
        Notice {
            id: ID.to_owned(),
            kind: Kind::Incident,
            title: "Security issue affecting KORDs".to_owned(),
            body: None,
            site_ids: vec!["kords".to_owned()],
            starts_at: 100,
            ends_at: None,
            updates: vec![Update {
                at: 100,
                text: "KORDs is unavailable.".to_owned(),
            }],
        }
    }

    fn validate(notice: Notice) -> Result<Notice, String> {
        notices(Notices {
            notices: vec![notice],
        })
        .map(|mut notices| notices.notices.remove(0))
    }

    #[test]
    fn accepts_and_tidies() {
        let notice = validate(Notice {
            id: format!(" {} ", ID.to_ascii_uppercase()),
            title: "  Title ".to_owned(),
            body: Some("   ".to_owned()),
            site_ids: vec!["kords".to_owned(), "kords".to_owned()],
            updates: vec![
                Update {
                    at: 200,
                    text: " Second ".to_owned(),
                },
                Update {
                    at: 100,
                    text: "First".to_owned(),
                },
            ],
            ..incident()
        })
        .unwrap();
        assert_eq!(notice.id, ID);
        assert_eq!(notice.title, "Title");
        assert_eq!(notice.body, None);
        assert_eq!(notice.site_ids, ["kords"]);
        let texts: Vec<_> = notice.updates.iter().map(|u| u.text.as_str()).collect();
        assert_eq!(texts, ["First", "Second"]);
    }

    #[test]
    fn rejects_bad_notices() {
        let cases = [
            Notice {
                title: " ".to_owned(),
                ..incident()
            },
            Notice {
                id: "2026-10-21-kords".to_owned(),
                ..incident()
            },
            Notice {
                site_ids: vec!["nope".to_owned()],
                ..incident()
            },
            Notice {
                starts_at: 0,
                ..incident()
            },
            Notice {
                ends_at: Some(50),
                ..incident()
            },
            Notice {
                updates: vec![Update {
                    at: 100,
                    text: " ".to_owned(),
                }],
                ..incident()
            },
            Notice {
                updates: vec![Update {
                    at: 0,
                    text: "x".to_owned(),
                }],
                ..incident()
            },
        ];
        for case in cases {
            assert!(validate(case).is_err());
        }
    }

    #[test]
    fn gives_new_notices_an_id() {
        let first = validate(Notice {
            id: String::new(),
            ..incident()
        })
        .unwrap();
        let second = validate(Notice {
            id: " ".to_owned(),
            ..incident()
        })
        .unwrap();
        let first = Uuid::try_parse(&first.id).unwrap();
        assert_eq!(first.get_version_num(), 7);
        assert_ne!(first.to_string(), second.id);
    }

    #[test]
    fn rejects_duplicate_ids() {
        let result = notices(Notices {
            notices: vec![incident(), incident()],
        });
        assert!(result.is_err());
    }
}
