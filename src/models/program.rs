use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::{Asia::Tokyo, Tz};
use serde_derive::{Deserialize, Serialize};

use crate::dto::program_xml::{ProgramXml, RadikoProgramXml};

// ```json
// "data": [
//   {
//     "start_time": "2025-06-29 00:00:00",
//     "end_time": "2025-06-29 01:30:00",
//     "start_time_s": "2400",
//     "end_time_s": "2530",
//     "program_date": "20250628",
//     "program_url": "https://www.mbs1179.com/yaru/",
//     "station_id": "MBS",
//     "performer": "極楽とんぼ、河合郁人、さらば青春の光（週替わり）、トム・ブラウン（週替わり）、小沢一敬（週替わり）（スピードワゴン）、大谷映美里（=LOVE）、池田裕子",
//     "title": "アッパレやってまーす！～土曜日です～",
//     "info": "",
//     "description": "メールアドレス：\u003ca href=mailto:yarudo@mbs1179.com target=_blank\u003eyarudo@mbs1179.com\u003c/a\u003e\u003cbr /\u003e\u003cbr /\u003e\u003cbr /\u003e◆アッパレやってまーす！&#65374;土曜日です&#65374;番組サイト◆\u003cbr /\u003e☆番組ホームページ：\u003ca href='https://www.mbs1179.com/yaru/' target=_blank\u003eこちらをクリック\u003c/a\u003e\u003cbr /\u003e☆X（旧Twitter）：\u003ca href='https://twitter.com/mbs_yarudo/' target=_blank\u003e@mbs_yarudo\u003c/a\u003e",
//     "status": "past",
//     "img": "https://program-static.cf.radiko.jp/6ff0b838-2453-4734-ad79-0be88a84b425.jpeg",
//     "genre": {
//       "personality": {
//         "id": "C010",
//         "name": "タレント"
//       },
//       "program": {
//         "id": "P006",
//         "name": "バラエティ"
//       }
//     },
//     "ts_in_ng": 0,
//     "ts_out_ng": 0,
//     "tsplus_in_ng": 0,
//     "tsplus_out_ng": 0,
//     "metas": [
//       {
//         "name": "twitter",
//         "value": "#radiko"
//       }
//     ]
//   },
//   {
//     "start_time": "2025-07-06 00:00:00",
//     "end_time": "2025-07-06 01:30:00",
//     "start_time_s": "2400",
//     "end_time_s": "2530",
//     "program_date": "20250705",
//     "program_url": "https://www.mbs1179.com/yaru/",
//     "station_id": "MBS",
//     "performer": "極楽とんぼ、河合郁人、さらば青春の光（週替わり）、トム・ブラウン（週替わり）、小沢一敬（週替わり）（スピードワゴン）、大谷映美里（=LOVE）、池田裕子",
//     "title": "アッパレやってまーす！～土曜日です～",
//     "info": "",
//     "description": "メールアドレス：\u003ca href=mailto:yarudo@mbs1179.com target=_blank\u003eyarudo@mbs1179.com\u003c/a\u003e\u003cbr /\u003e\u003cbr /\u003e\u003cbr /\u003e◆アッパレやってまーす！&#65374;土曜日です&#65374;番組サイト◆\u003cbr /\u003e☆番組ホームページ：\u003ca href='https://www.mbs1179.com/yaru/' target=_blank\u003eこちらをクリック\u003c/a\u003e\u003cbr /\u003e☆X（旧Twitter）：\u003ca href='https://twitter.com/mbs_yarudo/' target=_blank\u003e@mbs_yarudo\u003c/a\u003e",
//     "status": "future",
//     "img": "https://program-static.cf.radiko.jp/6ff0b838-2453-4734-ad79-0be88a84b425.jpeg",
//     "genre": {
//       "personality": {
//         "id": "C010",
//         "name": "タレント"
//       },
//       "program": {
//         "id": "P006",
//         "name": "バラエティ"
//       }
//     },
//     "ts_in_ng": 0,
//     "ts_out_ng": 0,
//     "tsplus_in_ng": 0,
//     "tsplus_out_ng": 0,
//     "metas": [
//       {
//         "name": "twitter",
//         "value": "#radiko"
//       }
//     ]
//   },
// ]
// ```

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    #[serde(with = "jst_datetime")]
    pub start_time: DateTime<Tz>,
    #[serde(with = "jst_datetime")]
    pub end_time: DateTime<Tz>,
    pub start_time_s: String,
    pub end_time_s: String,
    pub station_id: String,
    pub performer: String,
    pub title: String,
    pub info: String,
    pub description: String,
    pub img: String,
    #[serde(default, rename = "program_url")]
    pub program_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Programs {
    pub data: Vec<Program>,
}

impl Program {
    pub fn now_to_start_duration(&self, now: Option<DateTime<Tz>>) -> Option<u64> {
        let now = match now {
            Some(now) => now,
            None => Utc::now().with_timezone(&Tokyo),
        };
        let duration = self.start_time.signed_duration_since(now).num_seconds();
        if duration <= 0 {
            return None;
        }
        Some(duration as u64)
    }

    pub fn start_to_end_duration(&self) -> u64 {
        self.end_time
            .signed_duration_since(self.start_time)
            .num_seconds() as u64
    }

    pub fn now_to_end_duration(&self, now: Option<DateTime<Tz>>) -> Option<u64> {
        let now = match now {
            Some(now) => now,
            None => Utc::now().with_timezone(&Tokyo),
        };
        let duration = self.end_time.signed_duration_since(now).num_seconds();
        if duration <= 0 {
            return None;
        }
        Some(duration as u64)
    }
}

impl From<ProgramXml> for Program {
    fn from(value: ProgramXml) -> Self {
        let ft = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str(&value.ft, "%Y%m%d%H%M%S")
                    .expect("time parse error"),
            )
            .unwrap();
        let to = Tokyo
            .from_local_datetime(
                &NaiveDateTime::parse_from_str(&value.to, "%Y%m%d%H%M%S")
                    .expect("time parse error"),
            )
            .unwrap();
        Program {
            start_time: ft,
            end_time: to,
            start_time_s: value.ftl.clone(),
            end_time_s: value.tol.clone(),
            station_id: "".to_string(),
            performer: value.pfm.unwrap_or_default(),
            title: value.title.clone(),
            info: value.info.unwrap_or_default(),
            description: value.desc.unwrap_or_default(),
            img: value.img.unwrap_or_default(),
            program_url: value.url.unwrap_or_default(),
        }
    }
}

impl From<RadikoProgramXml> for Programs {
    fn from(value: RadikoProgramXml) -> Self {
        let mut programs = Vec::new();
        for station in value.stations.station {
            for programs_xml in station.programs {
                for program_xml in programs_xml.program {
                    let mut program = Program::from(program_xml);
                    program.station_id = station.id.clone();
                    programs.push(program);
                }
            }
        }
        Programs { data: programs }
    }
}

/// https://serde.rs/custom-date-format.html
mod jst_datetime {
    use chrono::{DateTime, NaiveDateTime, TimeZone};
    use chrono_tz::{Asia::Tokyo, Tz};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &DateTime<Tz>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Tz>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = NaiveDateTime::parse_from_str(&s, FORMAT).unwrap();
        Ok(Tokyo.from_local_datetime(&dt).unwrap())
    }
}
