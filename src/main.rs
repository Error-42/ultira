#![allow(confusable_idents, mixed_script_confusables)]

use std::{
    collections::HashMap,
    fs, io, iter,
    path::{Path, PathBuf},
    process,
};

use clap::{Arg, Args, FromArgMatches, Parser, Subcommand, ValueEnum};

/// Ulti rating calculator
///
/// Player naming:
///
/// Names are case sensitive. Using full names is recommended for players. You don't have to write out the full name.
///
/// 1. Given a pattern, if an exact match exists, that will be used.
/// 2. Otherwist a pattern matches the name iff there exists such a subsequence of the words of the name, the words of the pattern are prefixes of the corresponding words of the subsequence.
///
/// Example: "Márton" will match "Németh Márton" but not "Németh Marcell". "Németh M" will match both "Németh Márton" and "Németh Marcell" and therefore will give an error. "Dani" will match "Dániel".
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
    /// TODO
    #[command(visible_alias = "a")]
    Arbitrary(Arbitrary),
    /// TODO
    #[command(visible_alias = "c")]
    Circular(Circular),
    /// TODO
    #[command(visible_alias = "s")]
    Symmetric(Symmetric),
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
    /// TODO! update this when finished developing new capabilities
    ///
    /// These are
    /// - play
    /// - add-player
    /// - adjust realloc
    Undo(Undo),
    /// Renames a player to a new name, also allows merging players
    #[command(visible_alias = "rename")]
    RenamePlayer(RenamePlayer),
    /// TODO
    ExportRatings(ExportRatings),
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
struct Arbitrary {
    /// Specify the date of the play, does not affect the order of the plays. Format: YYYY-MM-DD
    #[arg(short = 'd', long)]
    date: Option<chrono::NaiveDate>,
}

// Hang on... this is basically ultira::Outcome! TODO: maybe refactor so that these two aren't different?
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Parser)]
#[command(no_binary_name = true)]
struct Score {
    /// TODO
    player: String,
    /// TODO
    #[arg(allow_hyphen_values = true)]
    score: i64,
}

#[derive(Debug, Parser)]
#[command(no_binary_name = true)]
struct ArbitraryGameCollection {
    /// TODO
    player_1: String,
    /// TODO
    player_2: String,
    /// TODO
    games: usize,
}

#[derive(Debug, Parser)]
struct Circular {
    /// TODO
    game_count: usize,
    /// Specify the date of the play, does not affect the order of the plays. Format: YYYY-MM-DD
    #[arg(short = 'd', long)]
    date: Option<chrono::NaiveDate>,
    /// TODO
    #[command(flatten)]
    scores: PlayScoreArgs,
}

#[derive(Debug, Parser)]
struct Symmetric {
    /// TODO
    round_count: usize,
    /// Specify the date of the play, does not affect the order of the plays. Format: YYYY-MM-DD
    #[arg(short = 'd', long)]
    date: Option<chrono::NaiveDate>,
    /// TODO
    #[command(flatten)]
    scores: PlayScoreArgs,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct PlayScoreArgs {
    scores: Vec<Score>,
}

// TODO: get these close to official style
impl FromArgMatches for PlayScoreArgs {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        let mut ret = PlayScoreArgs::default();
        ret.update_from_arg_matches(matches)?;
        Ok(ret)
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        let Some(mut scores) = matches.get_many::<String>("scores") else {
            return Err(clap::Error::raw(
                clap::error::ErrorKind::MissingRequiredArgument,
                "missing ...scores",
            ));
        };

        while let Some(name) = scores.next() {
            let Some(score) = scores.next() else {
                return Err(clap::Error::raw(
                    clap::error::ErrorKind::TooFewValues,
                    "no matching score for name",
                ));
            };

            let Ok(score) = score.parse() else {
                return Err(clap::Error::raw(
                    clap::error::ErrorKind::InvalidValue,
                    "score must be a number",
                ));
            };

            self.scores.push(Score {
                player: name.clone(),
                score,
            })
        }

        Ok(())
    }
}

impl Args for PlayScoreArgs {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        cmd.arg(
            Arg::new("scores")
                .num_args(1..)
                .required(true)
                .allow_negative_numbers(true),
        )
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        PlayScoreArgs::augment_args(cmd)
    }
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

#[derive(Debug, Parser)]
struct ExportRatings {
    file: PathBuf,
    /// Sets the name
    #[arg(default_value = "datum")]
    datum_name: String,
    /// Use a decimal comma instead of decimal point
    #[arg(short = 'c', long, action)]
    decimal_comma: bool,
    #[arg(short = 'b', long)]
    basis: ExportRatingsBasis,
}

#[derive(Debug, ValueEnum, Clone, Copy)]
enum ExportRatingsBasis {
    Date,
    Play,
    Game,
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
        print_rating_change(
            &player,
            data.config.rating_to_display(eval_before.ratings[&player]),
            data.config.rating_to_display(eval_after.ratings[&player]),
        );
    }

    ultira::write_data(path, &data).unwrap();
}

fn arbitrary(path: &Path, param: Arbitrary) {
    let mut data = read_data(path);

    // TODO: better prompts
    println!("Input the scores of players! One per line: <player> <score>. Write an empty line when complete.");

    let mut scores: HashMap<String, i64> = HashMap::new();

    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input.is_empty() {
            break;
        }

        let param = Score::parse_from(splitty::split_unquoted_whitespace(input));
        let Some(player) = try_find_name(&data, &param.player) else {
            continue;
        };

        *scores.entry(player).or_insert(0) += param.score;
    }

    println!("Input the number of games between players! One per line: <player_1> <player_2> <games>. Write an empty line when complete.");

    let mut game_collections = Vec::new();

    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input.is_empty() {
            break;
        }

        let param = ArbitraryGameCollection::parse_from(splitty::split_unquoted_whitespace(input));
        let players = [param.player_1, param.player_2];
        let players: Option<[String; 2]> = players
            .map(|player| try_find_name(&data, &player))
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|v| v.try_into().ok());
        let Some(players) = players else {
            continue;
        };

        game_collections.push(ultira::GameCollection {
            players,
            game_count: param.games,
        });
    }

    let arbitrary = ultira::Arbitrary {
        date: param
            .date
            .unwrap_or_else(|| chrono::Local::now().date_naive()),
        scores: scores.clone(),
        game_collections,
    };

    let eval_before = data.evaluate();

    data.arbitrary(arbitrary);

    // Maybe don't recalculate the whole thing?
    let eval_after = data.evaluate();

    for player in scores.keys() {
        print_rating_change(
            player,
            data.config.rating_to_display(eval_before.ratings[player]),
            data.config.rating_to_display(eval_after.ratings[player]),
        );
    }

    ultira::write_data(path, &data).unwrap();
}

fn circular(path: &Path, param: Circular) {
    let mut data = read_data(path);

    let Some(outcomes): Option<Vec<ultira::Outcome>> = param
        .scores
        .scores
        .into_iter()
        .map(|score| Some(ultira::Outcome { player: try_find_name(&data, &score.player)?, score: score.score }))
        .collect() else {
        return;
    };

    let eval_before = data.evaluate();

    data.circular(ultira::Circular {
        date: param
            .date
            .unwrap_or_else(|| chrono::Local::now().date_naive()),
        game_count: param.game_count,
        outcomes: outcomes.clone(),
    });

    let eval_after = data.evaluate();

    for ultira::Outcome { player, score: _score } in outcomes {
        print_rating_change(
            &player,
            data.config.rating_to_display(eval_before.ratings[&player]),
            data.config.rating_to_display(eval_after.ratings[&player]),
        )
    }

    ultira::write_data(path, &data).unwrap();
}

fn symmetric(path: &Path, param: Symmetric) {
    let mut data = read_data(path);

    let Some(scores): Option<HashMap<String, i64>> = param
        .scores
        .scores
        .into_iter()
        .map(|score| Some((try_find_name(&data, &score.player)?, score.score)))
        .collect()
    else {
        return;
    };

    let eval_before = data.evaluate();

    data.symmetric(ultira::Symmetric {
        date: param
            .date
            .unwrap_or_else(|| chrono::Local::now().date_naive()),
        scores: scores.clone(),
        round_count: param.round_count,
    });

    let eval_after = data.evaluate();

    for player in scores.keys() {
        print_rating_change(
            player,
            data.config.rating_to_display(eval_before.ratings[player]),
            data.config.rating_to_display(eval_after.ratings[player]),
        )
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

    ratings.sort_unstable_by(|(_player_a, rating_a), (_player_b, rating_b)| {
        rating_a.partial_cmp(rating_b).unwrap().reverse()
    });

    for (player, rating) in ratings {
        println!("{:6.1} {}", data.config.rating_to_display(rating), player);
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

    if data
        .evaluate()
        .ratings
        .keys()
        .any(|name| *name == rename.new_name)
    {
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

fn export_ratings(path: &Path, export: ExportRatings) {
    let data = read_data(path);

    let mut names: Vec<String> = data.evaluate().ratings.into_keys().collect();

    names.sort();

    let mut rows: Vec<Vec<String>> = Vec::new();

    // header
    rows.push(
        iter::once(export.datum_name)
            .chain(names.iter().cloned())
            .collect(),
    );

    let mut evaluation = data.starting_evaluation();

    let add_row_by_evaluation =
        |rows: &mut Vec<Vec<String>>,
         header_column: String,
         evaluation: &mut ultira::Evaluation| {
            rows.push(
                iter::once(header_column)
                    .chain(names.iter().map(|name| match evaluation.ratings.get(name) {
                        Some(rating) => {
                            let rating = data.config.rating_to_display(*rating).to_string();

                            if export.decimal_comma {
                                rating.replacen('.', ",", 1)
                            } else {
                                rating
                            }
                        }
                        None => "".to_string(),
                    }))
                    .collect(),
            );
        };

    match export.basis {
        ExportRatingsBasis::Date => {
            for change in data.history {
                if let Some(date) = change.date() {
                    if let Some(last_date) = evaluation.last_date {
                        if *date > last_date {
                            add_row_by_evaluation(
                                &mut rows,
                                last_date.to_string(),
                                &mut evaluation,
                            );
                        }
                    }
                }

                evaluation.change(&change);
            }

            if let Some(last_date) = evaluation.last_date {
                add_row_by_evaluation(&mut rows, last_date.to_string(), &mut evaluation);
            }
        }
        ExportRatingsBasis::Play => {
            for change in data.history {
                evaluation.change(&change);

                if let ultira::Change::Play(play) = change {
                    add_row_by_evaluation(&mut rows, play.date.to_string(), &mut evaluation);
                }
            }
        }
        ExportRatingsBasis::Game => {
            for change in data.history {
                match change {
                    ultira::Change::Play(_play) => {
                        todo!()
                    }
                    _ => evaluation.change(&change),
                }
            }
        }
    }

    let rows: Vec<_> = rows.into_iter().map(|row| row.join("\t")).collect();
    let output = rows.join("\n");

    fs::write(export.file, output).unwrap();
}

fn main() {
    let args: Cli = Cli::parse();

    match args.command {
        Command::Play(p) => play(&args.file, p),
        Command::Arbitrary(p) => arbitrary(&args.file, p),
        Command::Circular(p) => circular(&args.file, p),
        Command::Symmetric(p) => symmetric(&args.file, p),
        Command::New(p) => new(&args.file, p),
        Command::AddPlayer(p) => add_player(&args.file, p),
        Command::Ratings => ratings(&args.file),
        Command::Config(a) => adjust(&args.file, a.param),
        Command::Undo(p) => undo(&args.file, p),
        Command::RenamePlayer(p) => rename_player(&args.file, p),
        Command::ExportRatings(p) => export_ratings(&args.file, p),
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
            println!("Name '{name}' matched multiple names, aborting. Matched names are:");
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

fn print_rating_change(player: &str, rating_before: f64, rating_after: f64) {
    println!("{}: {:.1} -> {:.1}", player, rating_before, rating_after,);
}
