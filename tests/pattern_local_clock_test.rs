// ABOUTME: Pins that the weekly-schedule histograms count on the athlete's clock, not the server's
// ABOUTME: A 21:00 local session belongs to the day the athlete trained, at the hour they trained it
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! `detect_weekly_schedule` reports which days an athlete trains and at what
//! hour. Both histograms read `Activity::start_date()`, which is UTC.
//!
//! Without a conversion, every session starting at or after 20:00
//! `America/Toronto` was counted on the **following** weekday, and the hour
//! histogram was shifted by the whole offset — so an athlete who trains at
//! 18:00 was described as training at 22:00, on the wrong day.
//!
//! Context: `dravr-platform` fixed three sibling surfaces after a live athlete
//! spent the back half of a fifteen-turn conversation on 2026-09-02 correcting
//! weekday claims and left over it. This was the fourth surface, and the only
//! one outside that repo (registre#252).

use chrono::{Duration, TimeZone, Utc, Weekday};
use chrono_tz::America::Toronto;
use chrono_tz::Asia::Tokyo;
use chrono_tz::UTC;
use dravr_cageux::models::activity::{Activity, ActivityBuilder};
use dravr_cageux::models::sport::SportType;
use dravr_cageux::pattern_detection::PatternDetector;

/// Six evening rides, each 2026-09-02 01:30 UTC minus whole weeks — so every
/// one is a Tuesday 21:30 in Toronto and a Wednesday in UTC.
///
/// Six is `MIN_ACTIVITIES_FOR_PATTERN`, the floor below which detection
/// abstains.
fn tuesday_evening_rides() -> Vec<Activity> {
    (0..6)
        .map(|w| {
            let start = Utc
                .with_ymd_and_hms(2026, 9, 2, 1, 30, 0)
                .unwrap()
                .checked_sub_signed(Duration::weeks(w))
                .unwrap();
            ActivityBuilder::new(
                format!("ride-{w}"),
                "Road 2 AUS",
                SportType::Ride,
                start,
                7_200,
                "strava",
            )
            .build()
        })
        .collect()
}

#[test]
fn the_most_common_day_is_the_athletes_day_not_the_utc_one() {
    let pattern = PatternDetector::detect_weekly_schedule(&tuesday_evening_rides(), Toronto);

    assert_eq!(
        pattern.most_common_days.first(),
        Some(&Weekday::Tue),
        "21:30 Tuesday in Toronto is Tuesday training; got {:?}",
        pattern.most_common_days
    );
    assert_eq!(
        pattern.day_frequencies.get("Tue"),
        Some(&6),
        "all six rides land on the same local day: {:?}",
        pattern.day_frequencies
    );
    assert!(
        !pattern.day_frequencies.contains_key("Wed"),
        "Wednesday is the UTC reading and must not appear: {:?}",
        pattern.day_frequencies
    );
}

#[test]
fn the_most_common_hour_is_the_athletes_hour() {
    let pattern = PatternDetector::detect_weekly_schedule(&tuesday_evening_rides(), Toronto);

    assert_eq!(
        pattern.most_common_times.first(),
        Some(&21),
        "the athlete rides at 21:30 local; reporting 01:00 describes somebody \
         else's day: {:?}",
        pattern.most_common_times
    );
}

/// UTC is still available, explicitly, for an athlete with no zone on file —
/// the previous behaviour, now chosen rather than inherited.
#[test]
fn utc_reproduces_the_server_clock_reading() {
    let pattern = PatternDetector::detect_weekly_schedule(&tuesday_evening_rides(), UTC);

    assert_eq!(pattern.most_common_days.first(), Some(&Weekday::Wed));
    assert_eq!(pattern.most_common_times.first(), Some(&1));
}

/// A zone east of UTC shifts the other way, so a single hard-coded offset
/// cannot pass this file.
#[test]
fn a_zone_ahead_of_utc_moves_the_day_forward() {
    let pattern = PatternDetector::detect_weekly_schedule(&tuesday_evening_rides(), Tokyo);

    assert_eq!(
        pattern.most_common_days.first(),
        Some(&Weekday::Wed),
        "01:30 UTC is Wednesday morning in Tokyo"
    );
    assert_eq!(pattern.most_common_times.first(), Some(&10));
}
