/*
    MiniBit - A Minecraft minigame server network written in Rust.
    Copyright (C) 2024  Cheezer1656 (https://github.com/Cheezer1656/)

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

#![allow(dead_code)]

pub mod format {
    pub const DARK_RED: &str = "\u{00A7}4";
    pub const RED: &str = "\u{00A7}c";
    pub const GOLD: &str = "\u{00A7}6";
    pub const YELLOW: &str = "\u{00A7}e";
    pub const DARK_GREEN: &str = "\u{00A7}2";
    pub const GREEN: &str = "\u{00A7}a";
    pub const AQUA: &str = "\u{00A7}b";
    pub const DARK_AQUA: &str = "\u{00A7}3";
    pub const DARK_BLUE: &str = "\u{00A7}1";
    pub const BLUE: &str = "\u{00A7}9";
    pub const LIGHT_PURPLE: &str = "\u{00A7}d";
    pub const DARK_PURPLE: &str = "\u{00A7}5";
    pub const WHITE: &str = "\u{00A7}f";
    pub const GRAY: &str = "\u{00A7}7";
    pub const DARK_GRAY: &str = "\u{00A7}8";
    pub const BLACK: &str = "\u{00A7}0";
    pub const OBFUSCATED: &str = "\u{00A7}k";
    pub const BOLD: &str = "\u{00A7}l";
    pub const STRIKETHROUGH: &str = "\u{00A7}m";
    pub const UNDERLINE: &str = "\u{00A7}n";
    pub const ITALIC: &str = "\u{00A7}o";
    pub const RESET: &str = "\u{00A7}r";
}

pub enum ArmorColors {
    Red = 11546150,
    Orange = 16351261,
    Yellow = 16701501,
    Lime = 8439583,
    Green = 6192150,
    LightBlue = 3847130,
    Cyan = 1481884,
    Blue = 3949738,
    Purple = 8991416,
    Magenta = 13061821,
    Pink = 15961002,
    White = 16383998,
    LightGray = 10329495,
    Gray = 4673362,
    Black = 1908001,
    Brown = 8606770,
}
