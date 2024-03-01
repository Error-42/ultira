use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
struct Cli {
    file: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(alias = "p")]
    Play(Play),
    #[command(alias = "n")]
    New,
    AddUser(AddUser),
}

#[derive(Debug, Parser)]
struct Play {
    alone_player: String,
    team_player_1: String,
    team_player_2: String,
    #[arg(allow_hyphen_values = true)]
    points: f64,
}

#[derive(Debug, Parser)]
struct AddUser {
    user: String,
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
        println!("{name}: {past} -> {new}");
    }

    *data.ratings.get_mut(&play.alone_player).unwrap() = new_alone;
    *data.ratings.get_mut(&play.team_player_1).unwrap() = new_team[0];
    *data.ratings.get_mut(&play.team_player_2).unwrap() = new_team[1];

    ultira::write_data(path, &data).unwrap();
}

fn new(path: &Path) {
    ultira::write_data(path, &Default::default()).unwrap();
}

fn add_user(path: &Path, param: AddUser) {
    let mut data = ultira::read_data(path).unwrap();
    let rating = param
        .rating
        .or(data.config.default_rating)
        .expect("No default rating provided in file, so a rating must be provided");

    data.ratings.insert(param.user, rating);

    ultira::write_data(path, &data).unwrap();
}

fn main() {
    let args: Cli = Cli::parse();

    match args.command {
        Command::Play(p) => play(&args.file, p),
        Command::New => new(&args.file),
        Command::AddUser(p) => add_user(&args.file, p),
    }
}
