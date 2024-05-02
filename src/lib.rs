#![allow(mixed_script_confusables)]
//! Only the binary may be stable, the library cannot!
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

/// Returns the final ratings
///
/// ```
/// use ultira::rating_change;
///
/// let α = 0.1;
/// let game_count = 1;
/// let ratings = [0.0, 0.0, 0.0];
/// let scores = [4, -2, -2];
///
/// let new_ratings = rating_change(α, game_count, ratings, scores);
/// let expected_ratings = [0.4, -0.2, -0.2];
///
/// new_ratings
///     .iter()
///     .zip(expected_ratings.iter())
///     .for_each(|(a, b)| assert!((a - b).abs() < 0.0001));
/// ```
pub fn rating_change(α: f64, games: usize, ratings: [f64; 3], scores: [i64; 3]) -> [f64; 3] {
    let average_rating = ratings.iter().sum::<f64>() / 3.0;

    let mut new_ratings = [0.0; 3];

    for i in 0..3 {
        let r_i = ratings[i];
        let r_avg = average_rating;
        let g = games as i32;
        let s_i_avg = scores[i] as f64 / games as f64;

        new_ratings[i] = (1.0 - α).powi(g) * r_i + (r_avg + s_i_avg) * (1.0 - (1.0 - α).powi(g));

        assert!(new_ratings[i].is_finite());
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
                    let scores = play.outcomes.clone().map(|outcome| outcome.score);
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

    pub fn rename(&mut self, old_name: &str, new_name: &str) {
        for elem in &mut self.history {
            match elem {
                Change::AddPlayer(p) => {
                    if p.name == old_name {
                        p.name = new_name.to_owned();
                    }
                }
                Change::Play(p) => {
                    for outcome in &mut p.outcomes {
                        if outcome.player == old_name {
                            outcome.player = new_name.to_owned();
                        }
                    }
                }
                Change::AdjustAlpha(_) => {}
            }
        }
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
#[serde(rename_all = "snake_case")]
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
    pub score: i64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Evaluation {
    pub α: f64,
    pub ratings: HashMap<String, f64>,
}

impl Evaluation {
    pub fn matching_names<'s>(&'s self, pattern: &'s str) -> Vec<&'s str> {
        if self.ratings.keys().any(|name| name == pattern) {
            return vec![pattern];
        }

        self.ratings
            .keys()
            .filter(|name| match_names(name, pattern))
            .map(|name| name.as_str())
            .collect()
    }
}

fn match_names(matched: &str, pattern: &str) -> bool {
    let mut split_name = matched.split(' ').filter(|x| !x.is_empty());
    let split_pattern = pattern.split(' ').filter(|x| !x.is_empty());

    for pattern_word in split_pattern {
        loop {
            match split_name.next() {
                Some(word) if word.starts_with(pattern_word) => break,
                Some(_word) => {},
                None => return false,
            }
        }
    }

    true
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn name_matching() {
        assert!(match_names("Németh Marcell", "Németh M"));
        assert!(match_names("Németh Márton", "Németh M"));
        assert!(!match_names("Németh Dominik", "Németh M"));
        assert!(match_names("Németh Marcell", "Ma"));
        assert!(!match_names("Németh Márton", "Ma"));
    }
}
