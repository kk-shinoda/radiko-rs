use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

use crate::config::Weekday;

/// タイムフリー対象エピソード
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Episode {
    /// 開始日時 (JST)
    pub start: NaiveDateTime,
    /// 終了日時 (JST)
    pub end: NaiveDateTime,
    /// 放送日 (YYYYMMDD)
    pub date: NaiveDate,
}

/// 過去7日間（タイムフリー窓）を走査し、指定曜日に該当するエピソードを返す。
/// now は JST の現在日時。
pub fn find_episodes_in_timefree_window(
    _weekday: Weekday,
    _start_time: NaiveTime,
    _duration_minutes: u32,
    _now: NaiveDateTime,
) -> Vec<Episode> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn time(s: &str) -> NaiveTime {
        NaiveTime::parse_from_str(s, "%H:%M").unwrap()
    }

    #[test]
    fn test_finds_saturday_episode_in_past_week() {
        // 2026-02-17 (火) に実行 → 直近の土曜は 2026-02-14
        let now = dt("2026-02-17 12:00");
        let episodes =
            find_episodes_in_timefree_window(Weekday::Sat, time("00:00"), 30, now);
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].start, dt("2026-02-14 00:00"));
        assert_eq!(episodes[0].end, dt("2026-02-14 00:30"));
        assert_eq!(episodes[0].date, NaiveDate::from_ymd_opt(2026, 2, 14).unwrap());
    }

    #[test]
    fn test_finds_multiple_episodes_spanning_two_weeks() {
        // 2026-02-21 (土) 12:00 に実行 → 土曜は 2/14 と 2/21 の2回
        let now = dt("2026-02-21 12:00");
        let episodes =
            find_episodes_in_timefree_window(Weekday::Sat, time("10:00"), 60, now);
        assert_eq!(episodes.len(), 2);
        assert_eq!(episodes[0].date, NaiveDate::from_ymd_opt(2026, 2, 14).unwrap());
        assert_eq!(episodes[1].date, NaiveDate::from_ymd_opt(2026, 2, 21).unwrap());
    }

    #[test]
    fn test_excludes_episode_if_not_yet_aired_today() {
        // 2026-02-21 (土) 09:00 に実行 → 10:00開始の番組はまだ放送されていない
        let now = dt("2026-02-21 09:00");
        let episodes =
            find_episodes_in_timefree_window(Weekday::Sat, time("10:00"), 60, now);
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].date, NaiveDate::from_ymd_opt(2026, 2, 14).unwrap());
    }

    #[test]
    fn test_timefree_window_boundary_exactly_7_days() {
        // 2026-02-21 (土) 12:00 に実行
        // 7日前 = 2/14 00:00 以降が窓内
        // 2/14 00:00 開始の番組は窓の境界ちょうどなので含まれる
        let now = dt("2026-02-21 12:00");
        let episodes =
            find_episodes_in_timefree_window(Weekday::Sat, time("00:00"), 30, now);
        assert_eq!(episodes.len(), 2); // 2/14 と 2/21
    }

    #[test]
    fn test_midnight_crossing_program() {
        // 水曜01:00開始（火曜深夜25時）、120分
        // 2026-02-18 (水) 12:00 に実行 → 2/18 (水) 01:00 が該当
        let now = dt("2026-02-18 12:00");
        let episodes =
            find_episodes_in_timefree_window(Weekday::Wed, time("01:00"), 120, now);
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].start, dt("2026-02-18 01:00"));
        assert_eq!(episodes[0].end, dt("2026-02-18 03:00"));
    }

    #[test]
    fn test_no_matching_day_in_window() {
        // 2026-02-17 (火) → 7日前=2/10(火)、窓内に木曜は 2/12 のみ
        let now = dt("2026-02-17 12:00");
        let episodes =
            find_episodes_in_timefree_window(Weekday::Thu, time("20:00"), 60, now);
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].date, NaiveDate::from_ymd_opt(2026, 2, 12).unwrap());
    }
}
