#![allow(mixed_script_confusables)]

use std::{collections::HashMap, error::Error, fs, path::Path};

use serde::{Deserialize, Serialize};

pub fn read_data(path: &Path) -> Result<Data, Box<dyn Error>> {
    let data = fs::read_to_string(path)?;

    Ok(toml::from_str(&data)?)
}

pub fn write_data(path: &Path, data: &Data) -> Result<(), Box<dyn Error>> {
    let str = toml::to_string(data)?;

    Ok(fs::write(path, str)?)
}

/// Returns the changes in the ratings, not the final ratings.
pub fn rating_change(α: f64, games: usize, ratings: [f64; 3], scores: [f64; 3]) -> [f64; 3] {
    let average_rating = ratings.iter().sum::<f64>() / 3.0;

    let mut new_ratings = [0.0; 3];

    for i in 0..3 {
        let r_i = ratings[i];
        let r_avg = average_rating;
        let g = games as i32;
        let s_i_avg = scores[i] / games as f64;

        new_ratings[i] = (1.0 - α).powi(g) * r_i + (r_avg + s_i_avg) * (1.0 - (1.0 - α).powi(g));

        assert!(new_ratings[i].is_finite());

        // new_ratings[i] = (1.0 - config.realloc).powi(games as i32) * ratings[i]
        //     + (average_rating + config.score_multiplier / config.realloc * (scores[i] / games as f64))
        //     * (1.0 - (1.0 - config.realloc).powi(games as i32));
    }

    new_ratings
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Data {
    pub config: Config,
    pub history: Vec<Change>,
}

impl Data {
    pub fn evaluate(&self) -> Evaluation {
        let mut α = self.config.starting_alpha;
        let mut ratings: HashMap<String, f64> = HashMap::new();

        for change in &self.history {
            match change {
                Change::AddPlayer(addition) => {
                    ratings.insert(addition.name.clone(), addition.rating);
                }
                Change::Play(play) => {
                    let selected_ratings: [f64; 3] = play
                        .outcomes
                        .clone()
                        .map(|outcome| ratings[&outcome.player]);
                    let scores = play.outcomes.clone().map(|outcome| outcome.points);
                    let new_ratings = rating_change(α, play.game_count, selected_ratings, scores);

                    for i in 0..3 {
                        *ratings.get_mut(&play.outcomes[i].player).unwrap() = new_ratings[i];
                    }
                }
                Change::AdjustAlpha(new) => α = *new,
            }
        }

        Evaluation { α, ratings }
    }

    pub fn add_player(&mut self, name: String, rating: f64) {
        self.history
            .push(Change::AddPlayer(AddPlayer { name, rating }));
    }

    pub fn add_player_display(&mut self, name: String, display: f64) {
        self.add_player(name, self.config.rating_from_display(display));
    }

    pub fn play(&mut self, play: Play) {
        self.history.push(Change::Play(play));
    }

    pub fn adjust_α(&mut self, new: f64) {
        self.history.push(Change::AdjustAlpha(new));
    }

    pub fn adjust_score_multiplier(&mut self, new: f64) {
        self.adjust_α(self.config.α_from_display(new));
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub spread: f64,
    pub base_rating: f64,
    pub starting_alpha: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            spread: 50.0,
            base_rating: 100.0,
            starting_alpha: 0.02,
        }
    }
}

impl Config {
    pub fn rating_from_display(&self, display: f64) -> f64 {
        (display - self.base_rating) / self.spread
    }

    pub fn rating_to_display(&self, rating: f64) -> f64 {
        rating * self.spread + self.base_rating
    }

    pub fn α_from_display(&self, display: f64) -> f64 {
        display / self.spread
    }

    pub fn α_to_display(&self, α: f64) -> f64 {
        α * self.spread
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Deserialize, Serialize)]
#[non_exhaustive]
pub enum Change {
    AddPlayer(AddPlayer),
    Play(Play),
    AdjustAlpha(f64),
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct AddPlayer {
    pub name: String,
    pub rating: f64,
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Play {
    pub game_count: usize,
    #[serde(with = "toml_datetime_compat")]
    pub date: chrono::NaiveDate,
    pub outcomes: [Outcome; 3],
}

impl Play {
    pub fn now(game_count: usize, outcomes: [Outcome; 3]) -> Self {
        Play {
            game_count,
            date: chrono::Local::now().date_naive(),
            outcomes,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Outcome {
    pub player: String,
    pub points: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Evaluation {
    pub α: f64,
    pub ratings: HashMap<String, f64>,
}
