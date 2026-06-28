//! Bilingual (EN + HU) reminder email templates. No per-person language
//! preference exists yet, so every email carries both languages (minimal this
//! round). Wording tracks `.specs/LEGACY_I18N.md`.

use chrono::{DateTime, NaiveDate, Utc};

/// A rendered email: subject + plaintext body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedReminder {
    pub subject: String,
    pub body_text: String,
}

/// Context for the hourly last-call nudge. No pool — predictions are per-player.
#[derive(Debug)]
pub struct LastCallContext {
    pub group_name: String,
    /// The leaf group/match id — used for the My Tips deep link + anchor.
    pub group_id: String,
    pub deadline: DateTime<Utc>,
    /// SPA origin for absolute deep links (`XPOOL_PUBLIC_ORIGIN`).
    pub origin: String,
}

/// One line of the daily digest.
#[derive(Debug)]
pub struct DigestItem {
    pub group_name: String,
    pub group_id: String,
    pub deadline: DateTime<Utc>,
}

/// Context for the daily matchday digest. No pool.
#[derive(Debug)]
pub struct DigestContext {
    pub day: NaiveDate,
    pub origin: String,
    pub groups: Vec<DigestItem>,
}

fn fmt_deadline(d: DateTime<Utc>) -> String {
    d.format("%Y-%m-%d %H:%M UTC").to_string()
}

/// Deep link into the My Tips page for a group. The `/mytips/:groupId` route
/// resolves a leaf group id to the right round+group (`web/src/lib/groupRoute.ts`);
/// `#<group.id>` is the stable scroll anchor (knockout-subgroup-anchors).
pub fn mytips_link(origin: &str, group_id: &str) -> String {
    format!("{origin}/mytips/{group_id}#{group_id}")
}

/// The last-call (≈40min before deadline) email.
pub fn render_last_call(ctx: &LastCallContext) -> RenderedReminder {
    let when = fmt_deadline(ctx.deadline);
    let link = mytips_link(&ctx.origin, &ctx.group_id);
    let subject = format!(
        "Last call: your {group} predictions close soon \
         / Utolsó hívás: hamarosan lezárul a(z) {group} tippelés",
        group = ctx.group_name
    );
    let body_text = format!(
        "Hi there!\n\
         \n\
         The deadline for your {group} predictions is almost here — {when}. \
         You still have unlocked or missing tips, so jump in and finish them while there's time:\n\
         \n\
         {link}\n\
         \n\
         Lock them in before kick-off — good luck!\n\
         — xPool\n\
         \n\
         To stop these reminders, just reply to this email.\n\
         \n\
         ---\n\
         \n\
         Szia!\n\
         \n\
         A(z) {group} tippelési határidő mindjárt itt van — {when}. \
         Még van zárolatlan vagy hiányzó tipped, úgyhogy ugorj be, és fejezd be, amíg van idő:\n\
         \n\
         {link}\n\
         \n\
         Zárold le a kezdő sípszó előtt — sok sikert!\n\
         — xPool\n\
         \n\
         Ha nem kérsz több emlékeztetőt, válaszolj erre az emailre.\n",
        group = ctx.group_name,
        when = when,
        link = link,
    );
    RenderedReminder { subject, body_text }
}

/// The daily matchday digest email.
pub fn render_digest(ctx: &DigestContext) -> RenderedReminder {
    debug_assert!(
        !ctx.groups.is_empty(),
        "render_digest expects non-empty groups; the sweep skips empty digests"
    );
    let subject = format!(
        "Today's matches ({day}) — finish your predictions \
         / Mai meccsek ({day}) — fejezd be a tippeket",
        day = ctx.day
    );
    let lines: String = ctx
        .groups
        .iter()
        .map(|g| {
            format!(
                "  - {} ({})\n    {}\n",
                g.group_name,
                fmt_deadline(g.deadline),
                mytips_link(&ctx.origin, &g.group_id)
            )
        })
        .collect();
    let body_text = format!(
        "Hi there!\n\
         \n\
         Matches kick off today ({day}) that you still have unlocked or missing tips for:\n\
         \n\
         {lines}\n\
         Pop in and lock your tips before each deadline — good luck!\n\
         — xPool\n\
         \n\
         To stop these reminders, just reply to this email.\n\
         \n\
         ---\n\
         \n\
         Szia!\n\
         \n\
         Ma ({day}) ilyen meccsek jönnek, amikhez még van zárolatlan vagy hiányzó tipped:\n\
         \n\
         {lines}\n\
         Ugorj be, és zárold le a tippjeidet minden határidő előtt — sok sikert!\n\
         — xPool\n\
         \n\
         Ha nem kérsz több emlékeztetőt, válaszolj erre az emailre.\n",
        day = ctx.day,
        lines = lines,
    );
    RenderedReminder { subject, body_text }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn mytips_link_targets_the_group_route_and_anchor() {
        // /mytips/<group.id>#<group.id> — leaf group id resolves round+group,
        // the hash is the stable anchor (knockout-subgroup-anchors).
        assert_eq!(
            mytips_link("https://pool.xczimi.com", "M76"),
            "https://pool.xczimi.com/mytips/M76#M76"
        );
    }

    #[test]
    fn last_call_is_bilingual_with_deadline_and_deep_link() {
        let r = render_last_call(&LastCallContext {
            group_name: "Group A".into(),
            group_id: "A".into(),
            deadline: Utc.with_ymd_and_hms(2026, 6, 20, 18, 0, 0).unwrap(),
            origin: "https://pool.xczimi.com".into(),
        });
        // Subject is bilingual and names the group.
        assert!(r.subject.contains("Last call: your Group A")); // EN
        assert!(r.subject.contains("Utolsó hívás")); // HU (accents preserved)
                                                     // EN body: warm greeting + deadline + the call to action.
        assert!(r.body_text.contains("Hi there!"));
        assert!(r
            .body_text
            .contains("The deadline for your Group A predictions is almost here"));
        // HU body: greeting + the deadline noun (accents preserved).
        assert!(r.body_text.contains("Szia!"));
        assert!(r.body_text.contains("tippelési határidő"));
        // The deadline timestamp is present.
        assert!(r.body_text.contains("2026-06-20 18:00 UTC"));
        // The My Tips deep link keeps its exact shape.
        assert!(r.body_text.contains("https://pool.xczimi.com/mytips/A#A"));
        // Brand sign-off in both blocks.
        assert_eq!(r.body_text.matches("— xPool").count(), 2);
        // R4: opt-out lines in both language blocks.
        assert!(r
            .body_text
            .contains("To stop these reminders, just reply to this email."));
        assert!(r
            .body_text
            .contains("Ha nem kérsz több emlékeztetőt, válaszolj erre az emailre."));
    }

    #[test]
    fn digest_lists_every_group_in_both_languages_with_links() {
        let r = render_digest(&DigestContext {
            day: chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap(),
            origin: "https://pool.xczimi.com".into(),
            groups: vec![
                DigestItem {
                    group_name: "Group A".into(),
                    group_id: "A".into(),
                    deadline: Utc.with_ymd_and_hms(2026, 6, 20, 18, 0, 0).unwrap(),
                },
                DigestItem {
                    group_name: "Group B".into(),
                    group_id: "B".into(),
                    deadline: Utc.with_ymd_and_hms(2026, 6, 20, 21, 0, 0).unwrap(),
                },
            ],
        });
        // Subject is bilingual and carries the day.
        assert!(r.subject.contains("2026-06-20"));
        assert!(r.subject.contains("Today's matches")); // EN
        assert!(r.subject.contains("Mai meccsek")); // HU
                                                    // Both language blocks open with a warm greeting.
        assert!(r.body_text.contains("Hi there!")); // EN
        assert!(r.body_text.contains("Szia!")); // HU
                                                // EN body lead-in + HU body lead-in (accents preserved).
        assert!(r.body_text.contains("Matches kick off today"));
        assert!(r.body_text.contains("hiányzó tipped"));
        // Every group is listed with its deadline + deep link.
        assert!(r.body_text.contains("Group A"));
        assert!(r.body_text.contains("Group B"));
        assert!(r.body_text.contains("2026-06-20 18:00 UTC"));
        assert!(r.body_text.contains("2026-06-20 21:00 UTC"));
        assert!(r.body_text.contains("/mytips/A#A"));
        assert!(r.body_text.contains("/mytips/B#B"));
        // Brand sign-off in both blocks.
        assert_eq!(r.body_text.matches("— xPool").count(), 2);
        // R4: opt-out lines in both language blocks.
        assert!(r
            .body_text
            .contains("To stop these reminders, just reply to this email."));
        assert!(r
            .body_text
            .contains("Ha nem kérsz több emlékeztetőt, válaszolj erre az emailre."));
    }
}
