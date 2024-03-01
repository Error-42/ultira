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

pub fn rating_change(
    config: &Config,
    alone_rating: f64,
    team_ratings: &[f64; 2],
    points: f64,
) -> (f64, [f64; 2]) {
    let team_rating = (team_ratings[0] + team_ratings[1]) / 2.0;

    let alone_expected = (alone_rating - team_rating) / config.spread;
    let delta = config.k * (points - alone_expected);

    (
        alone_rating + delta,
        [team_ratings[0] - delta, team_ratings[1] - delta],
    )
}

pub fn add_user(path: &Path, name: &str, rating: f64) -> Result<(), Box<dyn Error>> {
    todo!()
}

pub fn play(path: &Path, alone_player: &str, team_players: [&str; 2], points: f64) {
    todo!()
}
