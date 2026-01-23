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
