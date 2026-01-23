#![allow(dead_code)]

use valence::ItemKind;

// Uses 1.8 damage
pub fn item_dmg(item: ItemKind) -> f32 {
    match item {
        ItemKind::WoodenSword => 4.0,
        ItemKind::GoldenSword => 4.0,
        ItemKind::StoneSword => 5.0,
        ItemKind::IronSword => 6.0,
        ItemKind::DiamondSword => 7.0,
        ItemKind::WoodenAxe => 3.0,
        ItemKind::GoldenAxe => 3.0,
        ItemKind::StoneAxe => 4.0,
        ItemKind::IronAxe => 5.0,
        ItemKind::DiamondAxe => 6.0,
        ItemKind::WoodenPickaxe => 2.0,
        ItemKind::GoldenPickaxe => 2.0,
        ItemKind::StonePickaxe => 3.0,
        ItemKind::IronPickaxe => 4.0,
        ItemKind::DiamondPickaxe => 5.0,
        ItemKind::WoodenShovel => 1.0,
        ItemKind::GoldenShovel => 1.0,
        ItemKind::StoneShovel => 2.0,
        ItemKind::IronShovel => 3.0,
        ItemKind::DiamondShovel => 4.0,
        _ => 1.0,
    }
}

pub fn armor_points(item: ItemKind) -> f32 {
    match item {
        ItemKind::LeatherHelmet => 1.0,
        ItemKind::LeatherChestplate => 3.0,
        ItemKind::LeatherLeggings => 2.0,
        ItemKind::LeatherBoots => 1.0,
        ItemKind::GoldenHelmet => 2.0,
        ItemKind::GoldenChestplate => 5.0,
        ItemKind::GoldenLeggings => 3.0,
        ItemKind::GoldenBoots => 1.0,
        ItemKind::ChainmailHelmet => 2.0,
        ItemKind::ChainmailChestplate => 5.0,
        ItemKind::ChainmailLeggings => 4.0,
        ItemKind::ChainmailBoots => 1.0,
        ItemKind::IronHelmet => 2.0,
        ItemKind::IronChestplate => 6.0,
        ItemKind::IronLeggings => 5.0,
        ItemKind::IronBoots => 2.0,
        ItemKind::DiamondHelmet => 3.0,
        ItemKind::DiamondChestplate => 8.0,
        ItemKind::DiamondLeggings => 6.0,
        ItemKind::DiamondBoots => 3.0,
        _ => 0.0,
    }
}

pub fn armor_toughness(item: ItemKind) -> f32 {
    match item {
        ItemKind::DiamondHelmet => 2.0,
        ItemKind::DiamondChestplate => 2.0,
        ItemKind::DiamondLeggings => 2.0,
        ItemKind::DiamondBoots => 2.0,
        _ => 0.0,
    }
}

pub fn calc_dmg(
    dmg: f32,
    helmet: ItemKind,
    chestplate: ItemKind,
    leggings: ItemKind,
    boots: ItemKind,
) -> f32 {
    let armor = armor_points(helmet)
        + armor_points(chestplate)
        + armor_points(leggings)
        + armor_points(boots);
    let toughness = armor_toughness(helmet)
        + armor_toughness(chestplate)
        + armor_toughness(leggings)
        + armor_toughness(boots);
    let reduction =
        20f32.min((armor / 5.0).max(armor - (4.0 * dmg) / (toughness.min(20.0) + 8.0))) / 25.0;
    dmg * (1.0 - reduction)
}

pub fn calc_dmg_with_weapon(
    weapon: ItemKind,
    helmet: ItemKind,
    chestplate: ItemKind,
    leggings: ItemKind,
    boots: ItemKind,
) -> f32 {
    let dmg = item_dmg(weapon);
    calc_dmg(dmg, helmet, chestplate, leggings, boots)
}
