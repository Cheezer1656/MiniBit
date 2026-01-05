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

mod subservers {
    automod::dir!(pub "src/bin/minibit/subservers");
}

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::thread;
use std::thread::JoinHandle;
use clap::{arg, command, value_parser, Arg};
use crate::subservers::*;

#[macro_export]
macro_rules! subserver {
    ($server:ident) => {
        (stringify!($server), $server::main as fn(PathBuf))
    };
}

fn main() {
    let subservers: HashMap<&str, fn(PathBuf)> = HashMap::from([
        subserver!(lobby),
        subserver!(bedwars),
        subserver!(bowfight),
        subserver!(boxing),
        subserver!(bridge),
        subserver!(classic),
        subserver!(parkour),
        subserver!(spaceshooter),
        subserver!(sumo),
        subserver!(trainchase),
    ]);

    let mut matches = command!()
        .arg(arg!(--auto <FOLDER> "Automatically launches servers")
            .required(false)
            .value_parser(value_parser!(PathBuf)));

    for subserver in subservers.keys() {
        matches = matches.arg(Arg::new(subserver)
            .long(subserver)
            .required(false)
            .value_name("FOLDER")
            .value_parser(value_parser!(PathBuf))
            .help(format!("Launch {} using configuration at FOLDER", subserver)))
    }

    let matches = matches.get_matches();

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    if let Some(auto) = matches.get_one::<PathBuf>("auto") {
        let dir = Path::new(&auto);

        for (game, run) in subservers {
            let dir = dir.join(game);
            if dir.exists() {
                println!("Starting {} at {}", game, dir.display());
                handles.push(thread::spawn(move || {
                    run(dir);
                }));
            }
        }
    } else {
        for (game, run) in subservers {
            if let Some(dir) = matches.get_one::<PathBuf>(game) {
                println!("Starting {} at {}", game, dir.display());
                let cloned = dir.clone();
                handles.push(thread::spawn(move || {
                    run(cloned);
                }));
            }
        }
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}
