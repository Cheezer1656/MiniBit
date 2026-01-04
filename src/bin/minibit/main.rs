/*
    MiniBit - A Minecraft minigame server network written in Rust.
    Copyright (C) 2026  Cheezer1656 (https://github.com/Cheezer1656/)

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

mod lobby;
mod bedwars;
mod bowfight;
mod boxing;
mod bridge;
mod classic;
mod parkour;
mod spaceshooter;
mod sumo;
mod trainchase;

use std::path::{Path, PathBuf};
use std::thread;
use std::thread::JoinHandle;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = None)] auto: Option<String>,

    #[arg(long, default_value = None)] lobby: Option<String>,
    #[arg(long, default_value = None)] bedwars: Option<String>,
    #[arg(long, default_value = None)] bowfight: Option<String>,
    #[arg(long, default_value = None)] boxing: Option<String>,
    #[arg(long, default_value = None)] bridge: Option<String>,
    #[arg(long, default_value = None)] classic: Option<String>,
    #[arg(long, default_value = None)] parkour: Option<String>,
    #[arg(long, default_value = None)] spaceshooter: Option<String>,
    #[arg(long, default_value = None)] sumo: Option<String>,
    #[arg(long, default_value = None)] trainchase: Option<String>,
}

#[macro_export]
macro_rules! minigames {
    ( $args:expr, $handles:expr, $( $x:ident ),* ) => {
        $(
            if let Some(path) = $args.$x {
                $handles.push(thread::spawn(|| {
                    println!("Starting {} at {}", stringify!($x), path.clone());
                    $x::main(PathBuf::from(path));
                }));
            }
        )*
    };
}

#[macro_export]
macro_rules! auto_minigames {
    ( $dir:expr, $handles:expr, $( $x:ident ),* ) => {
        $(
            let dir = $dir.join(stringify!($x));
            if dir.exists() {
                $handles.push(thread::spawn(|| {
                    println!("Starting {} at {}", stringify!($x), dir.display());
                    $x::main(dir);
                }));
            }
        )*
    };
}

fn main() {
    let args = Args::parse();

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    if let Some(auto) = args.auto {
        let dir = Path::new(&auto);
        auto_minigames!(dir, handles, lobby, bedwars, bowfight, boxing, bridge, classic, parkour, spaceshooter, sumo, trainchase);
    } else {
        minigames!(args, handles, lobby, bedwars, bowfight, boxing, bridge, classic, parkour, spaceshooter, sumo, trainchase);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}
