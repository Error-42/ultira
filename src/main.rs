use std::{env, fs, io::Write, path::{Path, PathBuf}};

use clap::{Parser, Subcommand};

/// Ulti rating calculator
///
/// There is a temporary logging functionality built in. Because it is temporary, it is not implemented correctly. Please do not use special characters in arguments.
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
    #[command(visible_alias = "add")]
    AddPlayer(AddPlayer),
    /// Print the ratings of the players
    #[command(visible_alias = "r")]
    Ratings,
    /// Adjust settings with rating corrections
    Adjust(Adjust),
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

#[derive(Debug, Parser)]
#[command(subcommand_help_heading = "Params", subcommand_value_name = "PARAM")]
struct Adjust {
    #[command(subcommand)]
    param: Param,
}

#[derive(Debug, Subcommand)]
enum Param {
    /// Controls how agressively ratings points are redistruted.
    ///
    /// Each game `realloc * (average rating of the group - player rating)` ratings points are redistrobuted to the player.
    Realloc { new_value: f64 },
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
    let rating = param.rating.unwrap_or(data.config.default_rating);

    data.ratings.insert(param.player, rating);

    ultira::write_data(path, &data).unwrap();
}

fn ratings(path: &Path) {
    let data = ultira::read_data(path).unwrap();

    let mut ratings: Vec<(&String, &f64)> = data.ratings.iter().collect();

    ratings.sort_unstable_by_key(|(player, _rating)| *player);

    for (player, rating) in ratings {
        println!("{player}: {rating:.1}");
    }
}

fn adjust(path: &Path, param: Param) {
    let mut data = ultira::read_data(path).unwrap();
    
    match param {
        Param::Realloc { new_value } => adjust_realloc(&mut data, new_value),
    }

    ultira::write_data(path, &data).unwrap();
}

fn adjust_realloc(data: &mut ultira::data::Data, new_value: f64) {
    let factor = new_value / data.config.realloc;
    let default_rating = data.config.default_rating;

    for (_player, rating) in data.ratings.iter_mut() {
        *rating = (*rating - default_rating) / factor + default_rating;

        assert!(rating.is_finite());
    }

    data.config.realloc = new_value;
}

fn log_command(path: &Path) {
    let args = env::args();

    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .unwrap();

    let args: Vec<_> = args.map(|arg| format!("\"{arg}\"")).collect();

    writeln!(file, "{}", args.join(" ")).unwrap();
}

fn main() {
    let args: Cli = Cli::parse();

    // We only log the command if it was valid.
    log_command(&args.file.with_extension(".log"));    

    match args.command {
        Command::Play(p) => play(&args.file, p),
        Command::New => new(&args.file),
        Command::AddPlayer(p) => add_player(&args.file, p),
        Command::Ratings => ratings(&args.file),
        Command::Adjust(a) => adjust(&args.file, a.param),
    }
}
