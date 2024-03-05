#![allow(mixed_script_confusables)]

use std::{error::Error, fs, path::Path};

pub mod data;

use data::*;

pub fn read_data(path: &Path) -> Result<Data, Box<dyn Error>> {
    let data = fs::read_to_string(path)?;

    Ok(toml::from_str(&data)?)
}

pub fn write_data(path: &Path, data: &Data) -> Result<(), Box<dyn Error>> {
    let str = toml::to_string(data)?;

    Ok(fs::write(path, str)?)
}

/// Returns the changes in the ratings, not the final ratings.
pub fn rating_change(
    config: &Config,
    games: usize,
    ratings: [f64; 3],
    scores: [f64; 3],
) -> [f64; 3] {
    let average_rating = ratings.iter().sum::<f64>() / 3.0;

    let mut new_ratings = [0.0; 3];

    for i in 0..3 {
        let r_i = ratings[i];
        let r_avg = average_rating;
        let g = games as i32;
        let s_i_avg = scores[i] / games as f64;
        let ρ = config.realloc;
        let σ = config.score_multiplier;

        new_ratings[i] =
            (1.0 - ρ).powi(g) * r_i + (r_avg + σ / ρ * s_i_avg) * (1.0 - (1.0 - ρ).powi(g));

        assert!(new_ratings[i].is_finite());

        // new_ratings[i] = (1.0 - config.realloc).powi(games as i32) * ratings[i]
        //     + (average_rating + config.score_multiplier / config.realloc * (scores[i] / games as f64))
        //     * (1.0 - (1.0 - config.realloc).powi(games as i32));
    }

    new_ratings
}
