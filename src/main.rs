use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

/// Ulti rating calculator
#[derive(Debug, Parser)]
#[clap(version)]
struct Cli {
    /// File containing the data
    file: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Evaluate rating changes after a game
    #[command(visible_alias = "p")]
    Play(Play),
    /// Create or clear the file
    #[command(visible_alias = "n")]
    New,
    /// Add a new player; if the player already exists, their rating will be overriden.
    #[command(visible_alias = "a")]
    AddPlayer(AddPlayer),
    /// Print the ratings of the players
    #[command(visible_alias = "r")]
    Ratings,
}

#[derive(Debug, Parser)]
struct Play {
    games: usize,
    player_1: String,
    #[arg(allow_hyphen_values = true)]
    score_1: f64,
    player_2: String,
    #[arg(allow_hyphen_values = true)]
    score_2: f64,
    player_3: String,
    #[arg(allow_hyphen_values = true)]
    score_3: f64,
}

#[derive(Debug, Parser)]
struct AddPlayer {
    /// The name of the new player
    player: String,
    /// The rating of the new player
    rating: Option<f64>,
}

fn play(path: &Path, play: Play) {
    let mut data = ultira::read_data(path).unwrap();

    let players = [play.player_1, play.player_2, play.player_3];
    let scores = [play.score_1, play.score_2, play.score_3];

    let mut ratings = [0.0; 3];

    for i in 0..3 {
        ratings[i] = data.ratings[&players[i]];
    }

    let new_ratings = ultira::rating_change(&data.config, play.games, ratings, scores);

    for i in 0..3 {
        println!("{}: {:.1} -> {:.1}", players[i], ratings[i], new_ratings[i]);
        *data.ratings.get_mut(&players[i]).unwrap() = new_ratings[i];
    }

    ultira::write_data(path, &data).unwrap();
}

fn new(path: &Path) {
    ultira::write_data(path, &Default::default()).unwrap();
}

fn add_player(path: &Path, param: AddPlayer) {
    let mut data = ultira::read_data(path).unwrap();
    let rating = param
        .rating
        .or(data.config.default_rating)
        .expect("No default rating provided in file, so a rating must be provided");

    data.ratings.insert(param.player, rating);

    ultira::write_data(path, &data).unwrap();
}

fn ratings(path: &Path) {
    let data = ultira::read_data(path).unwrap();

    let mut ratings: Vec<(&String, &f64)> = data
        .ratings
        .iter()
        .collect();
    
    ratings.sort_unstable_by_key(|(player, _rating)| *player);
    
    for (player, rating) in ratings {
        println!("{player}: {rating:.1}");
    }
}

fn main() {
    let args: Cli = Cli::parse();

    match args.command {
        Command::Play(p) => play(&args.file, p),
        Command::New => new(&args.file),
        Command::AddPlayer(p) => add_player(&args.file, p),
        Command::Ratings => ratings(&args.file),
    }
}
