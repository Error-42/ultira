use std::{path::{Path, PathBuf}, str::FromStr};

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
struct Cli {
    data: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Play(Play),
}

#[derive(Debug, Parser)]
struct Play {
    alone_player: String,
    team_player_1: String,
    team_player_2: String,
    points: f64,
}

fn play(path: &Path, play: Play) {
    let mut data = ultira::read_data(path).unwrap();

    let past_alone = data.ratings[&play.alone_player];
    let past_team = [data.ratings[&play.team_player_1], data.ratings[&play.team_player_2]];

    let (new_alone, new_team) = ultira::rating_change(&data.config, past_alone, &past_team, play.points);

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

fn main() {
    let args: Cli = Cli::parse();

    match args.command {
        Command::Play(p) => play(&args.data, p),
    }

    // let data = ultira::read_data(&PathBuf::from_str("test/a.toml").unwrap()).unwrap();
    // ultira::write_data(&PathBuf::from_str("test/b.toml").unwrap(), data).unwrap();
}
