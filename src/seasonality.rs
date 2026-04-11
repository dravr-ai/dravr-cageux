// ABOUTME: Season detection and sport-season compatibility for location-aware recommendations
// ABOUTME: Maps latitude to hemisphere, determines season, and rates sport suitability per season
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use serde::{Deserialize, Serialize};

use crate::models::SportType;

/// Latitude threshold for tropical zone (degrees from equator)
const TROPICAL_LATITUDE_THRESHOLD: f64 = 23.5;

// ============================================================================
// Core types
// ============================================================================

/// Astronomical season based on hemisphere and month
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Season {
    /// Spring (Mar-May in Northern Hemisphere)
    Spring,
    /// Summer (Jun-Aug in Northern Hemisphere)
    Summer,
    /// Fall / Autumn (Sep-Nov in Northern Hemisphere)
    Fall,
    /// Winter (Dec-Feb in Northern Hemisphere)
    Winter,
}

impl Season {
    /// Human-readable display name
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Spring => "spring",
            Self::Summer => "summer",
            Self::Fall => "fall",
            Self::Winter => "winter",
        }
    }
}

/// Geographic hemisphere derived from latitude
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Hemisphere {
    /// North of the Tropic of Cancer (~23.5°N)
    Northern,
    /// South of the Tropic of Capricorn (~23.5°S)
    Southern,
    /// Between the tropics — minimal seasonal variation
    Tropical,
}

/// How suitable a sport is for a given season at a location
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SeasonalSuitability {
    /// Snow on cycling path, ice on trails — sport is not viable
    Inappropriate = 0,
    /// Possible but conditions are poor (cold rain, short days)
    Marginal = 1,
    /// Can do it, not peak season but reasonable
    Acceptable = 2,
    /// Peak season for this sport
    Ideal = 3,
}

impl SeasonalSuitability {
    /// Whether the sport should be recommended in these conditions
    #[must_use]
    pub const fn is_recommended(&self) -> bool {
        matches!(self, Self::Ideal | Self::Acceptable)
    }
}

/// Seasonal context for a user's location at a given point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalContext {
    /// Current season at this location
    pub season: Season,
    /// Hemisphere derived from latitude
    pub hemisphere: Hemisphere,
    /// Latitude used for determination
    pub latitude: f64,
    /// Month (1-12) used for determination
    pub month: u32,
}

// ============================================================================
// Season detection
// ============================================================================

/// Determine hemisphere from latitude
///
/// Uses the Tropic of Cancer (23.5°N) and Tropic of Capricorn (23.5°S)
/// as boundaries for the tropical zone.
#[must_use]
pub fn hemisphere_from_latitude(latitude: f64) -> Hemisphere {
    if latitude > TROPICAL_LATITUDE_THRESHOLD {
        Hemisphere::Northern
    } else if latitude < -TROPICAL_LATITUDE_THRESHOLD {
        Hemisphere::Southern
    } else {
        Hemisphere::Tropical
    }
}

/// Determine the current season from hemisphere and month
///
/// For tropical latitudes, returns the closest equivalent based on
/// wet/dry patterns (wet season maps to Summer, dry to Winter).
///
/// # Panics
///
/// Never panics — months outside 1-12 are clamped to the nearest valid range.
#[must_use]
pub fn season_from_month(hemisphere: Hemisphere, month: u32) -> Season {
    let clamped_month = month.clamp(1, 12);

    match hemisphere {
        Hemisphere::Northern => match clamped_month {
            3..=5 => Season::Spring,
            6..=8 => Season::Summer,
            9..=11 => Season::Fall,
            _ => Season::Winter, // 12, 1, 2
        },
        Hemisphere::Southern => match clamped_month {
            // Seasons are inverted in the Southern Hemisphere
            3..=5 => Season::Fall,
            6..=8 => Season::Winter,
            9..=11 => Season::Spring,
            _ => Season::Summer, // 12, 1, 2
        },
        Hemisphere::Tropical => {
            // Tropical regions have minimal seasonal variation.
            // All seasons are warm enough for most outdoor sports.
            // Map to the same calendar seasons as Northern Hemisphere
            // for consistency, but sport suitability treats all as warm.
            match clamped_month {
                3..=5 => Season::Spring,
                6..=8 => Season::Summer,
                9..=11 => Season::Fall,
                _ => Season::Winter, // 12, 1, 2 — calendar winter but still warm
            }
        }
    }
}

/// Build a complete seasonal context from latitude and month
#[must_use]
pub fn build_seasonal_context(latitude: f64, month: u32) -> SeasonalContext {
    let hemisphere = hemisphere_from_latitude(latitude);
    let season = season_from_month(hemisphere, month);

    SeasonalContext {
        season,
        hemisphere,
        latitude,
        month,
    }
}

// ============================================================================
// Sport-season compatibility matrix
// ============================================================================

/// Get the seasonal suitability for a sport at a specific location
///
/// Tropical locations override winter restrictions since they never get snow.
#[must_use]
pub fn sport_suitability_at_location(
    sport: &SportType,
    context: &SeasonalContext,
) -> SeasonalSuitability {
    // Tropical regions don't have winter weather restrictions
    if context.hemisphere == Hemisphere::Tropical {
        return SeasonalSuitability::Ideal;
    }
    sport_seasonal_suitability(sport, &context.season)
}

/// Get the seasonal suitability for a given sport and season
///
/// Indoor and virtual sports are always Ideal regardless of season.
/// Outdoor sports are rated based on typical conditions for the season.
/// For location-aware checks that account for tropical climate, use
/// `sport_suitability_at_location` instead.
#[must_use]
pub fn sport_seasonal_suitability(sport: &SportType, season: &Season) -> SeasonalSuitability {
    use Season::{Fall, Spring, Summer, Winter};
    use SeasonalSuitability::{Acceptable, Ideal, Inappropriate, Marginal};

    match sport {
        // Indoor / virtual sports — season-independent
        SportType::VirtualRide
        | SportType::VirtualRun
        | SportType::Workout
        | SportType::Yoga
        | SportType::StrengthTraining
        | SportType::Crossfit
        | SportType::Pilates
        | SportType::Rowing // Indoor rowing is common
        | SportType::RockClimbing // Indoor climbing gyms
        | SportType::Basketball
        | SportType::Soccer => Ideal,

        // Outdoor cycling — needs dry, snow-free roads
        SportType::Ride | SportType::EbikeRide => match season {
            Summer => Ideal,
            Spring | Fall => Acceptable,
            Winter => Inappropriate,
        },
        SportType::MountainBike | SportType::GravelRide => match season {
            Summer => Ideal,
            Spring | Fall => Acceptable,
            Winter => Inappropriate,
        },

        // Running — possible year-round with gear, but winter is harder
        SportType::Run => match season {
            Spring | Summer | Fall => Ideal,
            Winter => Acceptable,
        },
        SportType::TrailRunning => match season {
            Summer | Fall => Ideal,
            Spring => Acceptable,
            Winter => Marginal,
        },

        // Walking and hiking
        SportType::Walk => match season {
            Spring | Summer | Fall => Ideal,
            Winter => Acceptable,
        },
        SportType::Hike => match season {
            Summer | Fall => Ideal,
            Spring => Acceptable,
            Winter => Marginal,
        },

        // Winter sports — need snow and cold
        SportType::CrossCountrySkiing | SportType::BackcountrySkiing => match season {
            Winter => Ideal,
            Fall | Spring => Marginal,
            Summer => Inappropriate,
        },
        SportType::AlpineSkiing | SportType::Snowboarding => match season {
            Winter => Ideal,
            Spring => Marginal,
            Summer | Fall => Inappropriate,
        },
        SportType::Snowshoe => match season {
            Winter => Ideal,
            Fall | Spring => Marginal,
            Summer => Inappropriate,
        },
        SportType::IceSkating => match season {
            Winter => Ideal,
            Fall | Spring => Marginal,
            Summer => Inappropriate,
        },

        // Water sports — need warm water and weather
        SportType::Swim => match season {
            Summer => Ideal,
            Spring | Fall => Acceptable,
            Winter => Marginal, // Indoor pools exist
        },
        SportType::Kayaking | SportType::Canoeing | SportType::Paddleboarding => match season {
            Summer => Ideal,
            Spring | Fall => Acceptable,
            Winter => Inappropriate,
        },
        SportType::Surfing | SportType::Kitesurfing => match season {
            Summer => Ideal,
            Spring | Fall => Acceptable,
            Winter => Marginal,
        },

        // Skating — needs dry pavement
        SportType::Skateboarding | SportType::InlineSkating => match season {
            Summer => Ideal,
            Spring | Fall => Acceptable,
            Winter => Inappropriate,
        },

        // Golf and tennis
        SportType::Tennis => match season {
            Summer => Ideal,
            Spring | Fall => Acceptable,
            Winter => Marginal, // Indoor courts
        },
        SportType::Golf => match season {
            Summer => Ideal,
            Spring | Fall => Acceptable,
            Winter => Inappropriate,
        },

        // Unknown/other — assume outdoor, moderate sensitivity
        SportType::Other(_) => match season {
            Summer => Ideal,
            Spring | Fall => Acceptable,
            Winter => Marginal,
        },
    }
}

/// Suggest alternative sports when the requested sport is inappropriate for the season
///
/// Returns a list of seasonally appropriate alternatives that provide
/// similar training benefits to the original sport.
#[must_use]
pub fn seasonal_alternatives(sport: &SportType, season: &Season) -> Vec<SportType> {
    let mut alternatives = Vec::with_capacity(4);

    // Only suggest alternatives when the sport is not recommended
    let suitability = sport_seasonal_suitability(sport, season);
    if suitability.is_recommended() {
        return alternatives;
    }

    match season {
        Season::Winter => {
            match sport {
                // Outdoor cycling → indoor cycling + winter sports
                SportType::Ride
                | SportType::EbikeRide
                | SportType::MountainBike
                | SportType::GravelRide => {
                    alternatives.push(SportType::VirtualRide);
                    alternatives.push(SportType::CrossCountrySkiing);
                    alternatives.push(SportType::Snowshoe);
                }
                // Trail running → XC skiing or snowshoeing
                SportType::TrailRunning | SportType::Hike => {
                    alternatives.push(SportType::Snowshoe);
                    alternatives.push(SportType::CrossCountrySkiing);
                    alternatives.push(SportType::BackcountrySkiing);
                }
                // Water sports → indoor options
                SportType::Kayaking
                | SportType::Canoeing
                | SportType::Paddleboarding
                | SportType::Swim => {
                    alternatives.push(SportType::Rowing); // Indoor rowing
                    alternatives.push(SportType::Swim); // Indoor pool
                    alternatives.push(SportType::CrossCountrySkiing);
                }
                // Skating → ice skating
                SportType::Skateboarding | SportType::InlineSkating => {
                    alternatives.push(SportType::IceSkating);
                    alternatives.push(SportType::CrossCountrySkiing);
                }
                // Golf → indoor options
                SportType::Golf => {
                    alternatives.push(SportType::Workout);
                    alternatives.push(SportType::CrossCountrySkiing);
                }
                _ => {}
            }
        }
        Season::Summer => {
            match sport {
                // Winter sports → summer equivalents
                SportType::CrossCountrySkiing | SportType::BackcountrySkiing => {
                    alternatives.push(SportType::TrailRunning);
                    alternatives.push(SportType::Hike);
                    alternatives.push(SportType::Ride);
                }
                SportType::AlpineSkiing | SportType::Snowboarding => {
                    alternatives.push(SportType::MountainBike);
                    alternatives.push(SportType::TrailRunning);
                    alternatives.push(SportType::Hike);
                }
                SportType::Snowshoe => {
                    alternatives.push(SportType::Hike);
                    alternatives.push(SportType::TrailRunning);
                }
                SportType::IceSkating => {
                    alternatives.push(SportType::InlineSkating);
                    alternatives.push(SportType::Skateboarding);
                    alternatives.push(SportType::Ride);
                }
                _ => {}
            }
        }
        Season::Spring | Season::Fall => {
            // Shoulder seasons — most sports work, only extreme cases need alternatives
            match sport {
                SportType::AlpineSkiing | SportType::Snowboarding => {
                    alternatives.push(SportType::MountainBike);
                    alternatives.push(SportType::Hike);
                    alternatives.push(SportType::TrailRunning);
                }
                SportType::IceSkating => {
                    alternatives.push(SportType::InlineSkating);
                    alternatives.push(SportType::Ride);
                }
                _ => {}
            }
        }
    }

    // Filter to only seasonally appropriate alternatives
    alternatives.retain(|alt| sport_seasonal_suitability(alt, season).is_recommended());

    alternatives
}

/// Get all sports that are ideal or acceptable for a given season
///
/// Useful for suggesting what to do rather than what not to do.
#[must_use]
pub fn recommended_sports_for_season(season: &Season) -> Vec<SportType> {
    let all_sports = [
        SportType::Run,
        SportType::Ride,
        SportType::Swim,
        SportType::Walk,
        SportType::Hike,
        SportType::VirtualRide,
        SportType::VirtualRun,
        SportType::Workout,
        SportType::Yoga,
        SportType::EbikeRide,
        SportType::MountainBike,
        SportType::GravelRide,
        SportType::CrossCountrySkiing,
        SportType::AlpineSkiing,
        SportType::Snowboarding,
        SportType::Snowshoe,
        SportType::IceSkating,
        SportType::BackcountrySkiing,
        SportType::Kayaking,
        SportType::Canoeing,
        SportType::Rowing,
        SportType::Paddleboarding,
        SportType::Surfing,
        SportType::Kitesurfing,
        SportType::StrengthTraining,
        SportType::Crossfit,
        SportType::Pilates,
        SportType::RockClimbing,
        SportType::TrailRunning,
        SportType::Soccer,
        SportType::Basketball,
        SportType::Tennis,
        SportType::Golf,
        SportType::Skateboarding,
        SportType::InlineSkating,
    ];

    all_sports
        .into_iter()
        .filter(|sport| sport_seasonal_suitability(sport, season).is_recommended())
        .collect()
}
