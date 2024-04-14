#![allow(confusable_idents, mixed_script_confusables)]

use std::{
    io,
    path::{Path, PathBuf},
    process,
};

use clap::{Parser, Subcommand};

/// Ulti rating calculator
/// 
/// Player naming:
/// 
/// Names are case sensitive. Using full names is recommended for players. You don't have to write out the full name. Given a pattern, if an exact match exists, that will be used, otherwise the pattern is split up by spaces. Each part must match a part of the original name in order.
/// 
/// - A pattern part consisting of a single letter will match any name part starting with that letter.
/// - Otherwise a pattern part must exactly match a name part.
/// 
/// Example: "Márton" will match "Németh Márton" but not "Németh Marcell". "Németh M" will match both "Németh Márton" and "Németh Marcell", but not "Németh Dominik".
#[derive(Debug, Parser)]
#[clap(version)]
struct Cli {
    /// File containing the data
    #[arg(default_value = "ultira.toml", short, long)]
    file: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Evaluate rating changes after a play.
    ///
    /// A play in defined as a consecutive series of games by the same three people on the same day uninterrupted. We consider a play interrupted iff at least one of the three members plays a rated game with someone other than the other two people.
    ///
    /// Plays are ordered, this will make this play the newest one, no matter the date.
    ///
    /// Each play has a date associated with it. If not specified, the system's date will be used in the proleptic Gregorian calendar. Monotonity is not guaranteed.
    #[command(visible_alias = "p")]
    Play(Play),
    /// Create or clear the file
    New(New),
    /// Add a new player; if the player already exists, their rating will be overriden.
    #[command(visible_alias = "add")]
    AddPlayer(AddPlayer),
    /// Print the ratings of the players
    #[command(visible_alias = "r")]
    Ratings,
    /// Get and set config.
    ///
    /// Not passing any parameters to config will show to current value.
    Config(Config),
    /// Undoes last command which affected history.
    ///
    /// These are
    /// - play
    /// - add-player
    /// - adjust realloc
    Undo(Undo),
    /// Renames a player to a new name, also allows merging players
    #[command(visible_alias = "rename")]
    RenamePlayer(RenamePlayer),
}

#[derive(Debug, Parser)]
struct Play {
    /// Number of games
    game_count: usize,
    /// Name of player 1
    player_1: String,
    /// Total score of player 1
    #[arg(allow_hyphen_values = true)]
    score_1: i64,
    /// Name of player 2
    player_2: String,
    /// Total score of player 2
    #[arg(allow_hyphen_values = true)]
    score_2: i64,
    /// Name of player 3
    player_3: String,
    /// Total score of player 3
    #[arg(allow_hyphen_values = true)]
    score_3: i64,
    /// Specify the date of the play, does not affect the order of the plays. Format: YYYY-MM-DD
    #[arg(short = 'd', long)]
    date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Parser)]
struct New {
    #[arg(short = 'n', long, action)]
    no_confirm: bool,
}

#[derive(Debug, Parser)]
struct AddPlayer {
    /// The name of the new player
    player: String,
    /// The rating of the new player
    #[arg(allow_hyphen_values = true)]
    rating: Option<f64>,
}

#[derive(Debug, Parser)]
#[command(subcommand_help_heading = "Params", subcommand_value_name = "PARAM")]
struct Config {
    #[command(subcommand)]
    param: Param,
}

#[derive(Debug, Subcommand)]
enum Param {
    /// If a player's rating if k * spread higher than the average, it means on average they win k points.
    ///
    /// This is only affects display ratings, not internal ones. Modifications do not get commited to history.
    #[command(visible_alias = "σ")]
    Spread { new_value: Option<f64> },
    /// Assuming equal ratings, the rating points will be adjusted by score multiplier * score.
    ///
    /// This affects both display and internal ratings. Modifications get commited to history, only affects new plays.
    #[command(visible_alias = "μ")]
    ScoreMultiplier { new_value: Option<f64> },
    /// Adjusting the base rating will increase ratings by the difference between the new and old one.
    ///
    /// This affects only display ratings, not internal ones. Modifications do not get commited to history.
    #[command(visible_alias = "δ")]
    BaseRating { new_value: Option<f64> },
}

#[derive(Debug, Parser)]
struct RenamePlayer {
    old_name: String,
    new_name: String,
}

#[derive(Debug, Parser)]
struct Undo {
    #[arg(short = 'n', long, action)]
    no_confirm: bool,
}

fn play(path: &Path, play: Play) {
    let mut data = read_data(path);

    let Some(player_1) = try_find_name(&data, &play.player_1) else {
        return;
    };

    let Some(player_2) = try_find_name(&data, &play.player_2) else {
        return;
    };

    let Some(player_3) = try_find_name(&data, &play.player_3) else {
        return;
    };

    let outcomes = [
        ultira::Outcome {
            player: player_1,
            score: play.score_1,
        },
        ultira::Outcome {
            player: player_2,
            score: play.score_2,
        },
        ultira::Outcome {
            player: player_3,
            score: play.score_3,
        },
    ];

    if outcomes.iter().map(|o| o.score).sum::<i64>() != 0 {
        eprintln!("Points don't sum to 0.");
        return;
    }

    let play = match play.date {
        Some(date) => ultira::Play {
            game_count: play.game_count,
            date,
            outcomes,
        },
        None => ultira::Play::now(play.game_count, outcomes),
    };

    let eval_before = data.evaluate();

    data.play(play.clone());

    let eval_after = data.evaluate();

    for ultira::Outcome { player, score: _ } in play.outcomes {
        println!(
            "{}: {:.1} -> {:.1}",
            player,
            data.config.rating_to_display(eval_before.ratings[&player]),
            data.config.rating_to_display(eval_after.ratings[&player]),
        );
    }

    ultira::write_data(path, &data).unwrap();
}

fn new(path: &Path, param: New) {
    if !param.no_confirm && path.exists() {
        println!(
            "Are you sure you want to override {} (y/N)?",
            path.to_string_lossy()
        );

        if !confirm() {
            return;
        }
    }

    ultira::write_data(path, &Default::default()).unwrap();
}

fn add_player(path: &Path, param: AddPlayer) {
    let mut data = read_data(path);
    let rating = param.rating.unwrap_or(data.config.base_rating);

    data.add_player_display(param.player, rating);

    ultira::write_data(path, &data).unwrap();
}

fn ratings(path: &Path) {
    let data = read_data(path);

    let mut ratings: Vec<(String, f64)> = data.evaluate().ratings.into_iter().collect();

    // Clone is probably avoidable, but I'm lazy.
    ratings.sort_unstable_by_key(|(player, _rating)| player.clone());

    for (player, rating) in ratings {
        println!("{}: {:.1}", player, data.config.rating_to_display(rating),);
    }
}

fn adjust(path: &Path, param: Param) {
    let mut data = read_data(path);

    match param {
        Param::Spread { new_value: None } => println!("{}", data.config.spread),
        Param::Spread {
            new_value: Some(val),
        } => data.config.spread = val,
        Param::ScoreMultiplier { new_value: None } => {
            println!("{}", data.config.α_to_display(data.evaluate().α))
        }
        Param::ScoreMultiplier {
            new_value: Some(val),
        } => data.adjust_score_multiplier(val),
        Param::BaseRating { new_value: None } => println!("{}", data.config.base_rating),
        Param::BaseRating {
            new_value: Some(val),
        } => data.config.base_rating = val,
    }

    ultira::write_data(path, &data).unwrap();
}

fn undo(path: &Path, undo: Undo) {
    let mut data = read_data(path);

    let Some(last) = data.history.last() else {
        eprintln!("Nothing to undo (undo only affects history)");
        process::exit(1);
    };

    if !undo.no_confirm {
        println!("Last element of history: {:#?}", last);

        println!(
            "Are you sure you want to undo last action affecting history (see above) inside {}? (y/N)",
            path.to_string_lossy()
        );

        if !confirm() {
            return;
        }
    }

    data.history.pop();

    ultira::write_data(path, &data).unwrap();
}

fn rename_player(path: &Path, rename: RenamePlayer) {
    let mut data = read_data(path);

    let Some(old_name) = try_find_name(&data, &rename.old_name) else {
        return;
    };

    if data.evaluate().ratings.keys().any(|name| *name == rename.new_name) {
        println!(
            "Name '{}' already exists. YOU CANNOT UNDO THIS OPERATION. Are you sure you want to MERGE these two players into one? (y/N)",
            rename.new_name
        );

        if !confirm() {
            return;
        }
    }

    data.rename(&old_name, &rename.new_name);

    ultira::write_data(path, &data).unwrap();

    println!("Renamed {old_name} to {}", rename.new_name);
}

fn main() {
    let args: Cli = Cli::parse();

    match args.command {
        Command::Play(p) => play(&args.file, p),
        Command::New(p) => new(&args.file, p),
        Command::AddPlayer(p) => add_player(&args.file, p),
        Command::Ratings => ratings(&args.file),
        Command::Config(a) => adjust(&args.file, a.param),
        Command::Undo(p) => undo(&args.file, p),
        Command::RenamePlayer(p) => rename_player(&args.file, p),
    }
}

fn read_data(path: &Path) -> ultira::Data {
    match ultira::read_data(path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("{err}");
            process::exit(1);
        }
    }
}

fn try_find_name(data: &ultira::Data, name: &str) -> Option<String> {
    let eval = data.evaluate();
    let matches = eval.matching_names(name);

    match matches.len() {
        0 => {
            println!("Name '{name}' didn't match any names, aborting...");
            None
        }
        1 => Some(matches[0].to_owned()),
        _ => {
            println!("Name '{name}' match multiple names, aborting. Matched names are:");
            for name in matches {
                println!("{name}");
            }
            None
        }
    }
}

fn confirm() -> bool {
    let mut ans = String::new();
    io::stdin().read_line(&mut ans).unwrap();

    if ans.trim() == "y" || ans.trim() == "Y" {
        true
    } else {
        println!("Confirmation didn't match 'y' or 'Y', aborting...");
        false
    }
}
