// ABOUTME: Re-exports recipe data models from pierre-core and extends MacroTargets with config-aware methods
// ABOUTME: Provides MacroTargetsExt trait for IntelligenceConfig-dependent calorie/timing calculations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 dravr.ai

use crate::config::intelligence::NutritionConfig;

// Re-export recipe model types from local models
pub use crate::models::recipes::{
    DietaryRestriction, IngredientUnit, MacroTargets, MealTiming, Recipe, RecipeConstraints,
    RecipeIngredient, SkillLevel, ValidatedNutrition,
};

/// Extension trait for `MacroTargets` that adds config-aware construction methods.
///
/// This trait exists because `from_calories_and_timing` requires access to
/// nutrition configuration (which lives in `dravr-cageux`), while the
/// `MacroTargets` type itself is defined alongside the recipe models to avoid
/// a circular dependency.
pub trait MacroTargetsExt {
    /// Create targets from a calorie goal, meal timing, and an explicit
    /// nutrition configuration snapshot.
    ///
    /// Defaults are based on ISSN sports nutrition position stands. Callers
    /// pass the relevant slice of [`crate::config::intelligence::IntelligenceConfig`]
    /// rather than reading from a global so the host process owns the
    /// configuration lifecycle.
    fn from_calories_and_timing(
        calories: f64,
        timing: MealTiming,
        nutrition: &NutritionConfig,
    ) -> MacroTargets;
}

impl MacroTargetsExt for MacroTargets {
    fn from_calories_and_timing(
        calories: f64,
        timing: MealTiming,
        nutrition: &NutritionConfig,
    ) -> MacroTargets {
        let (protein_pct, carbs_pct, fat_pct) =
            nutrition.meal_timing_macros.get_distribution(timing);

        Self::from_calories_and_distribution(calories, protein_pct, carbs_pct, fat_pct)
    }
}
