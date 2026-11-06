//! In-memory fixtures for inspecting history without touching the database.
use crate::persistence::Session;
use chrono::{DateTime, Days, Local, TimeZone};

pub(super) fn sessions(now: DateTime<Local>) -> Vec<Session> {
    let profiles = [
        (-1, "Sample · Client work", "#0083c9", 7500, "USD"),
        (-2, "Sample · Design", "#a855f7", 6000, "EUR"),
        (-3, "Sample · Learning", "#008c69", 0, "USD"),
        (-4, "Sample · Archived project", "#e58a23", 4500, "USD"),
    ];
    let notes = [
        "Implement the dashboard and review feedback",
        "Sketch navigation and refine the color palette",
        "Read a chapter and practice new concepts",
        "",
        "Investigate an intermittent issue\nAdd a regression test and document the fix.",
        "Plan the next release, review open tasks, and write a detailed handover for the team so the next session can pick up where this one ended.",
    ];
    let today = now.date_naive();
    let mut sessions = Vec::new();
    for day in 0..400_u64 {
        // Include quiet days and a range of calendar intensities.
        let count = if day == 0 { 4 } else { (day * 7 + 3) % 10 };
        let date = today.checked_sub_days(Days::new(day)).unwrap();
        for index in 0..count {
            let slot = (day + index) as usize;
            let (profile_id, name, color, rate, currency) = profiles[slot % profiles.len()];
            // The fourth profile only has old history, like an archived profile.
            if profile_id == -4 && day < 90 {
                continue;
            }
            let seconds = [25 * 60, 50 * 60, 15 * 60, 45 * 60][slot % 4];
            let finished_at = if day == 0 {
                now.timestamp() - index as i64 * 3600
            } else {
                let time = date.and_hms_opt(9 + index as u32, 0, 0).unwrap();
                let Some(finished) = Local.from_local_datetime(&time).earliest() else {
                    continue;
                };
                finished.timestamp()
            };
            let started_at = finished_at - seconds as i64;
            // Keep today's fixtures within today's local calendar date.
            if day == 0 && Local.timestamp_opt(started_at, 0).unwrap().date_naive() != today {
                continue;
            }
            sessions.push(Session {
                id: -(sessions.len() as i64 + 1),
                profile_id,
                profile_name: name.into(),
                color: color.into(),
                started_at,
                finished_at,
                seconds,
                note: notes[slot % notes.len()].into(),
                rate,
                currency: currency.into(),
            });
        }
    }
    sessions.sort_by_key(|session| std::cmp::Reverse((session.finished_at, session.id)));
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn fixtures_cover_filters_billing_and_calendar_without_future_sessions() {
        let now = Local.with_ymd_and_hms(2026, 11, 6, 18, 30, 0).unwrap();
        let sessions = sessions(now);
        assert!(sessions.len() > 1000);
        assert_eq!(
            sessions
                .iter()
                .map(|s| s.profile_id)
                .collect::<BTreeSet<_>>()
                .len(),
            4
        );
        assert_eq!(
            sessions
                .iter()
                .map(|s| &s.currency)
                .collect::<BTreeSet<_>>()
                .len(),
            2
        );
        assert!(sessions.iter().any(|s| s.rate == 0));
        assert!(sessions.iter().any(|s| s.earnings() > 0));
        assert!(sessions.iter().any(|s| s.note.is_empty()));
        assert!(sessions.iter().any(|s| s.note.contains('\n')));
        assert!(
            sessions
                .iter()
                .any(|s| s.finished_at < now.timestamp() - 365 * 86400)
        );
        assert!(sessions.iter().any(
            |s| Local.timestamp_opt(s.finished_at, 0).unwrap().date_naive() == now.date_naive()
        ));
        assert!(sessions.iter().all(|s| s.id < 0
            && s.profile_id < 0
            && s.started_at < s.finished_at
            && s.finished_at <= now.timestamp()
            && s.finished_at - s.started_at == s.seconds as i64));
        assert!(
            sessions
                .windows(2)
                .all(|pair| pair[0].finished_at >= pair[1].finished_at)
        );
        assert_eq!(sessions, super::sessions(now));
    }
}
