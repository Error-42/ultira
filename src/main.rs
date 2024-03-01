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
    /// The name of the player trying to complete the bet
    alone_player: String,
    /// The name of the first player working against the bet
    team_player_1: String,
    /// The name of the second player working against the bet
    team_player_2: String,
    /// The amount of points the bet is worth; positive if the bet was completed, negative otherwise
    #[arg(allow_hyphen_values = true)]
    points: f64,
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

    let past_alone = data.ratings[&play.alone_player];
    let past_team = [
        data.ratings[&play.team_player_1],
        data.ratings[&play.team_player_2],
    ];

    let (new_alone, new_team) =
        ultira::rating_change(&data.config, past_alone, &past_team, play.points);

    for (name, past, new) in [
        (&play.alone_player, past_alone, new_alone),
        (&play.team_player_1, past_team[0], new_team[0]),
        (&play.team_player_2, past_team[1], new_team[1]),
    ] {
        println!("{name}: {past:.0} -> {new:.0}");
    }

    *data.ratings.get_mut(&play.alone_player).unwrap() = new_alone;
    *data.ratings.get_mut(&play.team_player_1).unwrap() = new_team[0];
    *data.ratings.get_mut(&play.team_player_2).unwrap() = new_team[1];

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
        println!("{player}: {rating:.0}");
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
