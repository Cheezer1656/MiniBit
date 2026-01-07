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

use bigdecimal::BigDecimal;
use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = super::schema::players)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Player {
    pub uuid: BigDecimal,
    pub rank_id: Option<i32>,
    pub is_banned: bool,
    pub guild_id: Option<i32>,
    pub first_login: chrono::NaiveDateTime,
    pub last_login: chrono::NaiveDateTime,
    pub coins: i32,
    pub experience_points: BigDecimal,
    pub level: i32,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = super::schema::ranks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Rank {
    pub id: i32,
    pub name: String,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = super::schema::rank_permissions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RankPermission {
    pub rank_id: i32,
    pub permission: String,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = super::schema::friends)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Friend {
    pub player1: BigDecimal,
    pub player2: BigDecimal,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = super::schema::guilds)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Guild {
    pub uuid: i32,
    pub name: String,
    pub experience_points: BigDecimal,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = super::schema::minigame_stats)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MinigameStat {
    pub id: i32,
    pub player_id: BigDecimal,
    pub minigame: String,
    pub stat_key: String,
    pub stat_value: BigDecimal,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = super::schema::minigame_inventories)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MiniGameInventory {
    pub player_id: BigDecimal,
    pub minigame: String,
    pub inventory: serde_json::Value,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = super::schema::achievements)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Achievement {
    pub id: i32,
    pub achievement_name: String,
    pub description: String,
    pub reward: BigDecimal,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = super::schema::player_achievements)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PlayerAchievement {
    pub player_id: BigDecimal,
    pub achievement_id: i32,
}
