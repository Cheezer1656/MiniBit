use diesel::prelude::*;

table! {
    players (uuid) {
        uuid -> Numeric,
        rank_id -> Nullable<Int4>,
        is_banned -> Bool,
        guild_id -> Nullable<Int4>,
        first_login -> Timestamp,
        last_login -> Timestamp,
        coins -> Int4,
        experience_points -> Numeric,
        level -> Int4,
    }
}

table! {
    ranks (id) {
        id -> Int4,
        name -> Text,
    }
}

table! {
    rank_permissions (rank_id, permission) {
        rank_id -> Int4,
        permission -> Text,
    }
}

table! {
    friends (player1, player2) {
        player1 -> Numeric,
        player2 -> Numeric,
    }
}

table! {
    guilds (uuid) {
        uuid -> Int4,
        name -> Text,
        experience_points -> Numeric,
    }
}

table! {
    minigame_stats (id) {
        id -> Int4,
        player_id -> Numeric,
        minigame -> Text,
        stat_key -> Text,
        stat_value -> Numeric,
    }
}

table! {
    minigame_inventories (player_id, minigame) {
        player_id -> Numeric,
        minigame -> Text,
        inventory -> Jsonb,
    }
}

table! {
    achievements (id) {
        id -> Int4,
        achievement_name -> Text,
        description -> Text,
        reward -> Numeric,
    }
}

table! {
    player_achievements (player_id, achievement_id) {
        player_id -> Numeric,
        achievement_id -> Int4,
        earned_at -> Timestamp,
    }
}

joinable!(players -> ranks (rank_id));
joinable!(players -> guilds (guild_id));
joinable!(rank_permissions -> ranks (rank_id));
joinable!(friends -> players (player1));
// joinable!(friends -> players (player2));
joinable!(minigame_stats -> players (player_id));
joinable!(minigame_inventories -> players (player_id));
joinable!(player_achievements -> players (player_id));
joinable!(player_achievements -> achievements (achievement_id));

allow_tables_to_appear_in_same_query!(
    players,
    ranks,
    rank_permissions,
    friends,
    guilds,
    minigame_stats,
    minigame_inventories,
    achievements,
    player_achievements,
);
