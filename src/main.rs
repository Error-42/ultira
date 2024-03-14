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
    #[command(visible_alias = "σ")]
    Spread { new_value: f64 },
    /// Assuming equal ratings, the rating points will be adjusted by score multiplier * score.
    #[command(visible_alias = "μ")]
    ScoreMultiplier { new_value: f64 },
    /// Adjusting the base rating will increase ratings by the difference between the new and old one. 
    #[command(visible_alias = "δ")]
    BaseRating { new_value: Option<f64> },
}

fn play(path: &Path, play: Play) {
    let mut data = ultira::read_data(path).unwrap();

    let play = ultira::Play {
        game_count: play.games,
        outcomes: [
            ultira::Outcome {
                player: play.player_1,
                points: play.score_1,
            },
            ultira::Outcome {
                player: play.player_2,
                points: play.score_2,
            },
            ultira::Outcome {
                player: play.player_3,
                points: play.score_3,
            }
        ]
    };

    let eval_before = data.evaluate();

    data.play(play.clone());

    let eval_after = data.evaluate();

    for ultira::Outcome{ player, points: _ } in play.outcomes {
        println!(
            "{}: {:.1} -> {:.1}",
            player,
            data.config.rating_to_display(eval_before.ratings[&player]),
            data.config.rating_to_display(eval_after.ratings[&player]),
        );
    }

    ultira::write_data(path, &data).unwrap();
}

fn new(path: &Path) {
    ultira::write_data(path, &Default::default()).unwrap();
}

fn add_player(path: &Path, param: AddPlayer) {
    let mut data = ultira::read_data(path).unwrap();
    let rating = param.rating.unwrap_or(data.config.base_rating);

    data.add_player_display(param.player, rating);

    ultira::write_data(path, &data).unwrap();
}

fn ratings(path: &Path) {
    let data = ultira::read_data(path).unwrap();

    let mut ratings: Vec<(String, f64)> = data.evaluate().ratings.into_iter().collect();

    // Clone is probably avoidable, but I'm lazy.
    ratings.sort_unstable_by_key(|(player, _rating)| player.clone());

    for (player, rating) in ratings {
        println!(
            "{}: {:.1}",
            player,
            data.config.rating_to_display(rating),
        );
    }
}

fn adjust(path: &Path, param: Param) {
    let mut data = ultira::read_data(path).unwrap();
    
    match param {
        Param::Spread { new_value } => data.config.spread = new_value,
        Param::ScoreMultiplier { new_value } => data.adjust_score_multiplier(new_value),
        Param::BaseRating { new_value: None } => println!("{}", data.config.base_rating),
        Param::BaseRating { new_value: Some(val) } => data.config.base_rating = val,
    }

    ultira::write_data(path, &data).unwrap();
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
