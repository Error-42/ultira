#![allow(mixed_script_confusables)]
//! Only the binary may be stable, the library cannot!
use std::{collections::HashMap, error::Error, fs, path::Path};

use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};

pub fn read_data(path: &Path) -> Result<Data, Box<dyn Error>> {
    let data = fs::read_to_string(path)?;

    Ok(toml::from_str(&data)?)
}

pub fn write_data(path: &Path, data: &Data) -> Result<(), Box<dyn Error>> {
    let str = toml::to_string(data)?;

    Ok(fs::write(path, str)?)
}

pub fn update3(α: f64, games: f64, rating: f64, avg_rating: f64, avg_score: f64) -> f64 {
    (rating - avg_rating - avg_score) * f64::exp(-α * games) + avg_rating + avg_score
}

pub fn update_n(
    α: f64,
    games: f64,
    rating: f64,
    avg_rating: f64,
    total_score: f64,
    player_count: f64,
) -> f64 {
    let offset = avg_rating + 3.0 / player_count / (player_count - 2.0) * total_score;

    (rating - offset) * (-player_count * (player_count - 2.0) / 3.0 * α * games).exp() + offset
}

pub fn update_generic(
    α: f64,
    laplace_matrix: DMatrix<f64>,
    starting_ratings: DVector<f64>,
    scores: DVector<f64>,
    eps: f64,
) -> Result<DVector<f64>, &'static str> {
    let steady_state = -3.0 * laplace_matrix.clone().pseudo_inverse(eps)? * scores;

    Ok((laplace_matrix * α / 3.0).exp() * (starting_ratings - steady_state.clone()) + steady_state)
}

/// Returns the final ratings
///
/// ```
/// // TODO: fix this not working!
///
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
        let s_i_avg = scores[i] as f64 / games as f64;

        new_ratings[i] = update3(α, games as f64, r_i, r_avg, s_i_avg);

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
    pub fn starting_evaluation(&self) -> Evaluation {
        Evaluation::new(self.config.starting_alpha)
    }

    pub fn evaluate(&self) -> Evaluation {
        let mut evaluation = self.starting_evaluation();

        for change in &self.history {
            evaluation.change(change);
        }

        evaluation
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

    pub fn rating_period(&mut self, period: RatingPeriod) {
        self.history.push(Change::RatingPeriod(period))
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
                Change::RatingPeriod(_) => todo!(),
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum Change {
    AddPlayer(AddPlayer),
    // TODO: remove?
    Play(Play),
    RatingPeriod(RatingPeriod),
    AdjustAlpha(f64),
}

impl Change {
    pub fn date(&self) -> Option<&chrono::NaiveDate> {
        match self {
            Change::AddPlayer(_) => None,
            Change::Play(play) => Some(&play.date),
            Change::RatingPeriod(period) => Some(&period.date),
            Change::AdjustAlpha(_) => None,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RatingPeriod {
    pub date: chrono::NaiveDate,
    pub outcomes: Outcomes,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Outcomes {
    Arbitrary(ArbitraryOutcomes),
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ArbitraryOutcomes {
    pub scores: HashMap<String, i64>,
    pub game_collections: Vec<GameCollection>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct GameCollection {
    pub players: [String; 2],
    pub game_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Evaluation {
    pub α: f64,
    pub ratings: HashMap<String, f64>,
    pub last_date: Option<chrono::NaiveDate>,
}

impl Evaluation {
    pub fn new(α: f64) -> Self {
        Evaluation {
            α,
            ..Default::default()
        }
    }

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

    pub fn change(&mut self, change: &Change) {
        match change {
            Change::AddPlayer(addition) => {
                self.ratings.insert(addition.name.clone(), addition.rating);
            }
            Change::Play(play) => {
                let selected_ratings: [f64; 3] = play
                    .outcomes
                    .clone()
                    .map(|outcome| self.ratings[&outcome.player]);
                let scores = play.outcomes.clone().map(|outcome| outcome.score);
                let new_ratings = rating_change(self.α, play.game_count, selected_ratings, scores);

                self.last_date = Some(match self.last_date {
                    None => play.date,
                    Some(last_date) => play.date.max(last_date),
                });

                for i in 0..3 {
                    *self.ratings.get_mut(&play.outcomes[i].player).unwrap() = new_ratings[i];
                }
            }
            Change::RatingPeriod(period) => {
                match &period.outcomes {
                    Outcomes::Arbitrary(outcomes) => self.apply_arbtrarity_outcomes(outcomes),
                }

                self.last_date = Some(match self.last_date {
                    None => period.date,
                    Some(last_date) => period.date.max(last_date),
                });
            }
            Change::AdjustAlpha(new) => self.α = *new,
        }
    }

    pub fn apply_arbtrarity_outcomes(&mut self, outcomes: &ArbitraryOutcomes) {
        let player_index_pairs: Vec<(&String, &i64)> = outcomes.scores.iter().collect();
        let player_to_index: HashMap<&String, usize> = player_index_pairs
            .iter()
            .enumerate()
            .map(|(i, (player, _score))| (*player, i))
            .collect();

        let initial_ratings: DVector<f64> = DVector::from_iterator(
            player_index_pairs.len(),
            player_index_pairs
                .iter()
                .map(|(player, _score)| self.ratings[*player]),
        );

        let matrix: DMatrix<f64> = {
            let mut matrix = DMatrix::zeros(player_index_pairs.len(), player_index_pairs.len());

            for game_collection in &outcomes.game_collections {
                let i0 = player_to_index[&game_collection.players[0]];
                let i1 = player_to_index[&game_collection.players[1]];

                matrix[(i0, i1)] += game_collection.game_count as f64;
                matrix[(i1, i0)] += game_collection.game_count as f64;

                matrix[(i0, i0)] -= game_collection.game_count as f64;
                matrix[(i1, i1)] -= game_collection.game_count as f64;
            }

            matrix
        };

        let scores: DVector<f64> = DVector::from_iterator(
            player_index_pairs.len(),
            player_index_pairs
                .iter()
                .map(|(_player, score)| **score as f64),
        );

        let new_ratings = update_generic(self.α, matrix, initial_ratings, scores, 1e-6).unwrap();

        for i in 0..player_index_pairs.len() {
            *self.ratings.get_mut(player_index_pairs[i].0).unwrap() = new_ratings[i];
        }
    }
}

fn match_names(matched: &str, pattern: &str) -> bool {
    let mut split_name = matched.split(' ').filter(|x| !x.is_empty());
    let split_pattern = pattern.split(' ').filter(|x| !x.is_empty());

    for pattern_word in split_pattern {
        loop {
            match split_name.next() {
                Some(word) if word.starts_with(pattern_word) => break,
                Some(_word) => {}
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
