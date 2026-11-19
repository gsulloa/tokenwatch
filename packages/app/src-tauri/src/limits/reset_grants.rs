//! Free limit-reset grants (`cedar_ember` block of the usage response).
//!
//! Parsing is tolerant: anything unexpected yields an empty list, never an error.

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::db;

pub(crate) const DEFAULT_GRANT_LABEL: &str = "Reinicio de límites";
pub(crate) const EXPIRING_WINDOW_DAYS: i64 = 3;
pub(crate) const NOTIFIED_CAP: usize = 200;
pub(crate) const GRANTS_META_KEY: &str = "reset_grants_notified";

const CLAIM_HINT: &str = "Reclámalo en claude.ai → Configuración → Uso";

// Fields kept to document the API contract.
#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct ApiCedarEmber {
    pub eligible: Option<bool>,
    pub ineligible_reason: Option<String>,
    pub grants: Option<Vec<serde_json::Value>>,
    pub next_grant_id: Option<String>,
    pub cooldown_until: Option<String>,
}

// Fields kept to document the API contract.
#[allow(dead_code)]
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub(crate) struct ApiGrant {
    pub id: Option<String>,
    pub label: Option<String>,
    pub resets_total: Option<i64>,
    pub resets_left: Option<i64>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub clears: Option<Vec<String>>,
    pub paused: Option<bool>,
    pub usable_now: Option<bool>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub unavailable_reason: Option<String>,
    pub gives_back_weekly: Option<bool>,
    pub claimable: Option<bool>,
}

/// Available free reset grant, as sent to the frontend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetGrant {
    pub id: String,
    pub label: String,
    pub resets_left: u32,
    /// RFC 3339 as received (only if it parses); `None` if missing or unparseable.
    pub ends_at: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub usable_now: bool,
}

fn parse_ends_at(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

/// Parse the raw `cedar_ember` value into the available grants, sorted by
/// `ends_at` ascending (None last). Returns empty on: None, JSON null, wrong
/// type, `eligible != Some(true)`, `grants` null/missing.
pub(crate) fn available_grants(
    raw: Option<serde_json::Value>,
    now: DateTime<Utc>,
) -> Vec<ResetGrant> {
    let Some(raw) = raw else {
        return vec![];
    };
    let block = match serde_json::from_value::<ApiCedarEmber>(raw) {
        Ok(b) => b,
        Err(_) => {
            tracing::debug!("limits: cedar_ember block not parseable; ignoring");
            return vec![];
        }
    };
    if block.eligible != Some(true) {
        return vec![];
    }

    let mut out: Vec<(Option<DateTime<Utc>>, ResetGrant)> = Vec::new();
    for v in block.grants.unwrap_or_default() {
        let Ok(g) = serde_json::from_value::<ApiGrant>(v) else {
            continue;
        };
        let Some(id) = g.id.filter(|s| !s.is_empty()) else {
            continue;
        };
        let resets_left = g.resets_left.unwrap_or(0);
        if g.paused == Some(true) || resets_left <= 0 {
            continue;
        }
        let ends = g.ends_at.as_deref().and_then(parse_ends_at);
        if let Some(e) = ends {
            if e <= now {
                continue;
            }
        }
        let label = g
            .label
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_GRANT_LABEL.to_owned());
        out.push((
            ends,
            ResetGrant {
                id,
                label,
                resets_left: u32::try_from(resets_left).unwrap_or(u32::MAX),
                ends_at: ends.and(g.ends_at),
                kind: g.kind,
                usable_now: g.usable_now.unwrap_or(false),
            },
        ));
    }
    // Stable sort: dated grants ascending, undated last.
    out.sort_by(|a, b| match (a.0, b.0) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    out.into_iter().map(|(_, g)| g).collect()
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(crate) struct GrantNotifyState {
    pub available: Vec<String>,
    pub expiring: Vec<String>,
}

fn body_available(label: &str, date: Option<&str>) -> String {
    match date {
        Some(d) => format!("Reinicio de límites disponible: {label} · vence el {d}. {CLAIM_HINT}"),
        None => format!("Reinicio de límites disponible: {label}. {CLAIM_HINT}"),
    }
}

fn body_expiring(label: &str, date: &str) -> String {
    format!("Reinicio de límites por vencer: {label} · vence el {date}. {CLAIM_HINT}")
}

/// "30 nov". Months: ene feb mar abr may jun jul ago sept oct nov dic.
pub(crate) fn format_short_date_es(date: NaiveDate) -> String {
    const MONTHS: [&str; 12] = [
        "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
    ];
    let m = MONTHS[(date.month0() as usize) % 12];
    format!("{} {m}", date.day())
}

fn local_date_label(ends: DateTime<Utc>) -> String {
    format_short_date_es(ends.with_timezone(&Local).date_naive())
}

fn cap(list: &mut Vec<String>) {
    if list.len() > NOTIFIED_CAP {
        let excess = list.len() - NOTIFIED_CAP;
        list.drain(..excess);
    }
}

/// Pure state machine. Mutates `state` and returns the notification bodies to send
/// (in grant order). Does not know about mute.
pub(crate) fn next_grant_notifications(
    state: &mut GrantNotifyState,
    grants: &[ResetGrant],
    now: DateTime<Utc>,
) -> Vec<String> {
    let mut bodies = Vec::new();
    for g in grants {
        let ends = g.ends_at.as_deref().and_then(parse_ends_at);
        let expiring_now = ends.is_some_and(|e| {
            let left = e - now;
            left > Duration::zero() && left <= Duration::days(EXPIRING_WINDOW_DAYS)
        });
        let date = ends.map(local_date_label);
        let is_new = !state.available.contains(&g.id);
        let in_expiring = state.expiring.contains(&g.id);
        if is_new {
            bodies.push(body_available(&g.label, date.as_deref()));
            state.available.push(g.id.clone());
            if expiring_now && !in_expiring {
                state.expiring.push(g.id.clone());
            }
        } else if expiring_now && !in_expiring {
            if let Some(d) = date.as_deref() {
                bodies.push(body_expiring(&g.label, d));
            }
            state.expiring.push(g.id.clone());
        }
    }
    cap(&mut state.available);
    cap(&mut state.expiring);
    bodies
}

/// Load state from meta, run the state machine, persist if changed, and return
/// the bodies to send — or an empty vec when `muted` (state still advances).
pub(crate) fn evaluate_grant_alerts(
    conn: &rusqlite::Connection,
    grants: &[ResetGrant],
    now: DateTime<Utc>,
    muted: bool,
) -> Vec<String> {
    if grants.is_empty() {
        return vec![];
    }
    let mut state: GrantNotifyState = db::meta_get(conn, GRANTS_META_KEY)
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let before = state.clone();
    let bodies = next_grant_notifications(&mut state, grants, now);
    if state != before {
        if let Ok(json) = serde_json::to_string(&state) {
            let _ = db::meta_set(conn, GRANTS_META_KEY, &json);
        }
    }
    if muted {
        vec![]
    } else {
        bodies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-07T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    fn valid_grant() -> serde_json::Value {
        json!({"id":"g1","label":"Semanal","resets_total":1,"resets_left":1,
               "ends_at":"2026-11-30T12:00:00Z","type":"weekly","usable_now":true})
    }

    fn grant(id: &str, ends: Option<&str>) -> ResetGrant {
        ResetGrant {
            id: id.to_owned(),
            label: format!("L-{id}"),
            resets_left: 1,
            ends_at: ends.map(str::to_owned),
            kind: None,
            usable_now: true,
        }
    }

    fn ids(v: &[ResetGrant]) -> Vec<&str> {
        v.iter().map(|g| g.id.as_str()).collect()
    }

    #[test]
    fn cedar_none_returns_empty() {
        assert!(available_grants(None, now()).is_empty());
    }

    #[test]
    fn cedar_null_returns_empty() {
        assert!(available_grants(Some(serde_json::Value::Null), now()).is_empty());
    }

    #[test]
    fn cedar_wrong_type_returns_empty() {
        assert!(available_grants(Some(json!("x")), now()).is_empty());
        assert!(available_grants(Some(json!([1])), now()).is_empty());
    }

    #[test]
    fn cedar_ineligible_returns_empty() {
        let raw = json!({"eligible":false,"ineligible_reason":"surface","grants":[valid_grant()]});
        assert!(available_grants(Some(raw), now()).is_empty());
    }

    #[test]
    fn cedar_eligible_missing_flag_returns_empty() {
        let raw = json!({"grants":[valid_grant()]});
        assert!(available_grants(Some(raw), now()).is_empty());
    }

    #[test]
    fn cedar_grants_null_returns_empty() {
        let raw = json!({"eligible":true,"grants":null});
        assert!(available_grants(Some(raw), now()).is_empty());
    }

    #[test]
    fn eligible_mixed_grants_selects_available() {
        let raw = json!({"eligible":true,"grants":[
            {"id":"A","resets_left":1,"ends_at":"2026-12-01T00:00:00Z","type":"weekly","usable_now":true},
            {"id":"B","resets_left":1,"paused":true},
            {"id":"C","resets_left":0},
            {"id":"D","resets_left":1,"ends_at":"2026-11-01T00:00:00Z"},
            {"resets_left":1},
            {"id":"F","resets_left":"uno"},
            {"id":"G","resets_left":2}
        ]});
        let out = available_grants(Some(raw), now());
        assert_eq!(ids(&out), vec!["A", "G"]);
        assert_eq!(out[0].kind.as_deref(), Some("weekly"));
        assert!(out[0].usable_now);
        assert_eq!(out[0].ends_at.as_deref(), Some("2026-12-01T00:00:00Z"));
        assert_eq!(out[1].resets_left, 2);
        assert!(!out[1].usable_now);
        assert_eq!(out[1].ends_at, None);
    }

    #[test]
    fn available_sorted_by_ends_at_none_last() {
        let raw = json!({"eligible":true,"grants":[
            {"id":"n","resets_left":1},
            {"id":"c","resets_left":1,"ends_at":"2026-12-20T00:00:00Z"},
            {"id":"a","resets_left":1,"ends_at":"2026-11-10T00:00:00Z"},
            {"id":"b","resets_left":1,"ends_at":"2026-12-01T00:00:00Z"}
        ]});
        assert_eq!(
            ids(&available_grants(Some(raw), now())),
            vec!["a", "b", "c", "n"]
        );
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let raw = json!({"eligible":true,"at_limit":true,"foo":{"bar":1},"grants":[
            {"id":"x","resets_left":1,"foo":{"bar":1},"weekly_resets_at":"2026-11-09T00:00:00Z"}
        ]});
        assert_eq!(available_grants(Some(raw), now()).len(), 1);
    }

    #[test]
    fn missing_label_uses_default() {
        let raw = json!({"eligible":true,"grants":[
            {"id":"x","resets_left":1},
            {"id":"y","resets_left":1,"label":""}
        ]});
        let out = available_grants(Some(raw), now());
        assert_eq!(out[0].label, "Reinicio de límites");
        assert_eq!(out[1].label, "Reinicio de límites");
    }

    #[test]
    fn unparseable_ends_at_becomes_none() {
        let raw = json!({"eligible":true,"grants":[
            {"id":"x","resets_left":1,"ends_at":"mañana"}
        ]});
        let out = available_grants(Some(raw), now());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].ends_at, None);
    }

    #[test]
    fn first_run_notifies_all_available() {
        let mut st = GrantNotifyState::default();
        let gs = [grant("a", None), grant("b", None)];
        let out = next_grant_notifications(&mut st, &gs, now());
        assert_eq!(out.len(), 2);
        assert!(out
            .iter()
            .all(|b| b.starts_with("Reinicio de límites disponible: ")));
        assert_eq!(st.available, vec!["a", "b"]);
    }

    #[test]
    fn does_not_repeat_available() {
        let mut st = GrantNotifyState::default();
        let gs = [grant("a", None)];
        next_grant_notifications(&mut st, &gs, now());
        assert!(next_grant_notifications(&mut st, &gs, now()).is_empty());
    }

    #[test]
    fn new_grant_after_first_run_is_notified() {
        let mut st = GrantNotifyState::default();
        next_grant_notifications(&mut st, &[grant("a", None)], now());
        let out = next_grant_notifications(&mut st, &[grant("a", None), grant("b", None)], now());
        assert_eq!(out.len(), 1);
        assert!(out[0].contains("L-b"));
    }

    #[test]
    fn expiring_notified_once() {
        let mut st = GrantNotifyState {
            available: vec!["a".into()],
            expiring: vec![],
        };
        let gs = [grant("a", Some("2026-11-09T12:00:00Z"))];
        let out = next_grant_notifications(&mut st, &gs, now());
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with("Reinicio de límites por vencer: "));
        assert!(next_grant_notifications(&mut st, &gs, now()).is_empty());
    }

    #[test]
    fn expiring_boundary() {
        let mk = || GrantNotifyState {
            available: vec!["a".into()],
            expiring: vec![],
        };
        let mut st = mk();
        let at = [grant("a", Some("2026-11-10T12:00:00Z"))];
        assert_eq!(next_grant_notifications(&mut st, &at, now()).len(), 1);
        let mut st = mk();
        let past = [grant("a", Some("2026-11-10T12:00:01Z"))];
        assert!(next_grant_notifications(&mut st, &past, now()).is_empty());
    }

    #[test]
    fn new_and_expiring_sends_single_notification() {
        let mut st = GrantNotifyState::default();
        let gs = [grant("a", Some("2026-11-08T12:00:00Z"))];
        let out = next_grant_notifications(&mut st, &gs, now());
        assert_eq!(out.len(), 1);
        assert!(out[0].starts_with("Reinicio de límites disponible: "));
        assert_eq!(st.available, vec!["a"]);
        assert_eq!(st.expiring, vec!["a"]);
        assert!(next_grant_notifications(&mut st, &gs, now()).is_empty());
    }

    #[test]
    fn body_without_ends_at_omits_date() {
        let mut st = GrantNotifyState::default();
        let out = next_grant_notifications(&mut st, &[grant("a", None)], now());
        assert_eq!(
            out[0],
            "Reinicio de límites disponible: L-a. Reclámalo en claude.ai → Configuración → Uso"
        );
    }

    #[test]
    fn body_with_date_exact() {
        assert_eq!(
            body_available("X", Some("30 nov")),
            "Reinicio de límites disponible: X · vence el 30 nov. Reclámalo en claude.ai → Configuración → Uso"
        );
        assert_eq!(
            body_expiring("X", "30 nov"),
            "Reinicio de límites por vencer: X · vence el 30 nov. Reclámalo en claude.ai → Configuración → Uso"
        );
    }

    #[test]
    fn notified_lists_are_capped() {
        let mut st = GrantNotifyState {
            available: (0..NOTIFIED_CAP).map(|i| format!("old{i}")).collect(),
            expiring: vec![],
        };
        next_grant_notifications(&mut st, &[grant("new", None)], now());
        assert_eq!(st.available.len(), NOTIFIED_CAP);
        assert!(!st.available.contains(&"old0".to_owned()));
        assert!(st.available.contains(&"new".to_owned()));
    }

    #[test]
    fn format_short_date_es_months() {
        let d = |y, m, dd| NaiveDate::from_ymd_opt(y, m, dd).unwrap();
        assert_eq!(format_short_date_es(d(2026, 1, 5)), "5 ene");
        assert_eq!(format_short_date_es(d(2026, 9, 30)), "30 sept");
        assert_eq!(format_short_date_es(d(2026, 11, 30)), "30 nov");
    }

    #[test]
    fn evaluate_persists_and_does_not_repeat() {
        let conn = db::open_at(":memory:").unwrap();
        let gs = [grant("a", None)];
        assert_eq!(evaluate_grant_alerts(&conn, &gs, now(), false).len(), 1);
        assert!(evaluate_grant_alerts(&conn, &gs, now(), false).is_empty());
        let stored = db::meta_get(&conn, GRANTS_META_KEY).unwrap().unwrap();
        assert!(stored.contains("\"a\""));
    }

    #[test]
    fn evaluate_muted_returns_empty_but_advances_state() {
        let conn = db::open_at(":memory:").unwrap();
        let gs = [grant("a", None)];
        assert!(evaluate_grant_alerts(&conn, &gs, now(), true).is_empty());
        assert!(evaluate_grant_alerts(&conn, &gs, now(), false).is_empty());
    }

    #[test]
    fn evaluate_corrupt_meta_resets_state() {
        let conn = db::open_at(":memory:").unwrap();
        db::meta_set(&conn, GRANTS_META_KEY, "{not json").unwrap();
        let gs = [grant("a", None)];
        assert_eq!(evaluate_grant_alerts(&conn, &gs, now(), false).len(), 1);
    }

    #[test]
    fn evaluate_empty_grants_does_not_write_meta() {
        let conn = db::open_at(":memory:").unwrap();
        assert!(evaluate_grant_alerts(&conn, &[], now(), false).is_empty());
        assert_eq!(db::meta_get(&conn, GRANTS_META_KEY).unwrap(), None);
    }
}
