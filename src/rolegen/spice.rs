use crate::core::player::Role;

use super::{get_normal_dist_rand, RoleGen, RoleSet};
use rand::{rngs::ThreadRng, seq::SliceRandom};

fn create_spicy_town_list(roleset: &RoleSet) -> Vec<RoleGen> {
    let mut roles = vec![];
    if roleset.contains(&RoleGen::COP) {
        roles.append(&mut vec![RoleGen::COP; 4]);
    }
    if roleset.contains(&RoleGen::DOCTOR) {
        roles.append(&mut vec![RoleGen::DOCTOR; 3]);
    }
    if roleset.contains(&RoleGen::CELEB) {
        roles.append(&mut vec![RoleGen::CELEB; 2]);
    }
    if roleset.contains(&RoleGen::MILLER) {
        roles.append(&mut vec![RoleGen::MILLER; 3]);
    }
    roles.append(&mut vec![RoleGen::TOWN; 1]);

    roles
}

fn create_spicy_mafia_list(roleset: &RoleSet) -> Vec<RoleGen> {
    let mut roles = vec![];
    if roleset.contains(&RoleGen::STRIPPER) {
        roles.append(&mut vec![RoleGen::STRIPPER; 4]);
    }
    if roleset.contains(&RoleGen::GODFATHER) {
        roles.append(&mut vec![RoleGen::GODFATHER; 3]);
    }
    if roleset.contains(&RoleGen::GOON) {
        roles.append(&mut vec![RoleGen::GOON; 2]);
    }
    roles.append(&mut vec![RoleGen::MAFIA; 1]);

    roles
}
fn create_rogue_list(roleset: &RoleSet) -> Vec<RoleGen> {
    let mut rogue_list = vec![];
    if roleset.contains(&RoleGen::IDIOT) {
        rogue_list.append(&mut vec![RoleGen::IDIOT; 40]);
    }
    if roleset.contains(&RoleGen::SURVIVOR) {
        rogue_list.append(&mut vec![RoleGen::SURVIVOR; 20])
    }
    if roleset.contains(&RoleGen::GUARD) {
        rogue_list.append(&mut vec![RoleGen::GUARD; 10]);
    }
    if roleset.contains(&RoleGen::GUARD_Mafia) {
        rogue_list.append(&mut vec![RoleGen::GUARD_Mafia; 5]);
    }
    if roleset.contains(&RoleGen::AGENT) {
        rogue_list.append(&mut vec![RoleGen::AGENT; 10]);
    }
    if roleset.contains(&RoleGen::AGENT_Mafia) {
        rogue_list.append(&mut vec![RoleGen::AGENT_Mafia; 5]);
    }

    if rogue_list.is_empty() {
        rogue_list.append(&mut vec![RoleGen::TOWN; 20]);
    }

    rogue_list
}
fn get_team_score(n_town: usize, n_mafia: usize) -> i32 {
    let maf_challenge = (n_town as i32 - n_mafia as i32 + 1) / 2 * 10;
    let town_challenge = n_mafia as i32 * 15;
    maf_challenge - town_challenge
}

fn get_n_mafia(n_players: usize, spice: f64) -> usize {
    if n_players <= 5 {
        return 1;
    }
    loop {
        let div = 3.7;
        let r = get_normal_dist_rand(0.0, div / 1.5);
        let n_mafia = ((n_players as f64) + r) / div;
        let n_mafia = n_mafia as usize;
        if n_mafia < 1 {
            continue;
        } else if n_mafia > n_players / 2 - 1 {
            continue;
        }
        return n_mafia;
    }
}

fn get_n_rogue(n_players: usize, spice: f64) -> usize {
    loop {
        let r = get_normal_dist_rand(3.0, 3.0);
        let n_rogue = ((n_players as f64).log2() * r * (0.7 + spice)) / 8.0;
        let n_rogue = n_rogue as usize;
        if n_rogue > n_players / 2 + 1 {
            continue;
        }
        return n_rogue;
    }
}

fn get_n_spice(n: usize, spice: f64) -> usize {
    let s = (n as f64 + 1.0).log2() * spice;
    loop {
        let r = get_normal_dist_rand(spice * n as f64, s) + 0.5;
        let r = r as usize;
        if r > n {
            continue;
        }
        return r;
    }
}

fn get_town(n_town: usize, spice: f64, roleset: &RoleSet) -> Vec<RoleGen> {
    let n_spicy_town = get_n_spice(n_town, spice);
    let mut roles: Vec<RoleGen> = create_spicy_town_list(roleset)
        .choose_multiple(&mut ThreadRng::default(), n_spicy_town)
        .map(|r| *r)
        .collect();
    roles.extend(vec![RoleGen::TOWN; n_town - roles.len()]);
    roles
}

fn get_mafia(n_mafia: usize, spice: f64, roleset: &RoleSet) -> Vec<RoleGen> {
    let n_spicy_mafia = get_n_spice(n_mafia, spice);
    let mut roles: Vec<RoleGen> = create_spicy_mafia_list(roleset)
        .choose_multiple(&mut ThreadRng::default(), n_spicy_mafia)
        .map(|r| *r)
        .collect();
    roles.extend(vec![RoleGen::MAFIA; n_mafia - roles.len()]);
    roles
}

fn get_rogue(n_rogue: usize, score: i32, roleset: &RoleSet) -> Vec<RoleGen> {
    if n_rogue == 0 {
        return vec![];
    }
    let mut best_set = create_rogue_list(roleset)
        .choose_multiple(&mut ThreadRng::default(), n_rogue)
        .map(|r| *r)
        .collect();
    let mut best_score = score;
    for _ in 0..10 {
        let mut rogue_score = 0;
        let roles = create_rogue_list(roleset)
            .choose_multiple(&mut ThreadRng::default(), n_rogue)
            .map(|r| *r)
            .collect();
        for role in &roles {
            rogue_score += match role {
                &RoleGen::IDIOT => 0,
                &RoleGen::GUARD => 0,
                &RoleGen::GUARD_Mafia => -10,
                &RoleGen::AGENT => -10,
                &RoleGen::AGENT_Mafia => 10,
                _ => 0,
            }
        }
        let new_score = rogue_score + score;
        if new_score.abs() < best_score.abs() {
            best_score = new_score;
            best_set = roles;
        }
    }
    return best_set;
}

fn get_roles(n_players: usize, spice: f64, roleset: &RoleSet) -> Vec<RoleGen> {
    // Guards for invariants?
    let n_mafia = get_n_mafia(n_players, spice);
    let n_rogue = get_n_rogue(n_players - n_mafia, spice);
    let n_town = n_players - n_mafia - n_rogue;

    // Want spice to be slightly more likely to apply to mafia
    let town_spice = spice * 1.0;
    let mafia_spice = spice * 1.0;

    let mut roles = Vec::new();

    let team_score = (n_town as i32 - 1 - n_mafia as i32 * 2) * 10;
    roles.extend(get_rogue(n_rogue, team_score, roleset));

    roles.extend(get_town(n_town, town_spice, roleset));
    roles.extend(get_mafia(n_mafia, mafia_spice, roleset));

    roles
}

mod test {

    use std::collections::HashMap;

    use super::super::{basic_roleset, full_roleset, minimal_roleset};
    use super::*;
    use rand;

    #[test]
    fn test_spice_rolegen() {
        let roleset = full_roleset();
        for _ in 0..10 {
            let roles = get_roles(7, 0.3, &roleset);
            println!("{:?}", roles);
        }
    }

    #[test]
    #[ignore]
    fn test_spice() {
        for n in 1..=10 {
            let mut counts = HashMap::<usize, usize>::new();
            for _ in 0..10000 {
                let spice = get_n_spice(n, 0.9);
                let v = counts.get(&spice).unwrap_or(&0);
                counts.insert(spice, v + 1);
            }
            let mut counts_entries: Vec<_> = counts.iter().collect();
            counts_entries.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
            println!("{}: {:?}", n, counts_entries)
        }
    }

    #[test]
    fn test_n_rogue() {
        for n in 3..=10 {
            let mut counts = HashMap::<usize, usize>::new();
            for _ in 0..10000 {
                let spice = get_n_rogue(n, 0.3);
                let v = counts.get(&spice).unwrap_or(&0);
                counts.insert(spice, v + 1);
            }
            let mut counts_entries: Vec<_> = counts.iter().collect();
            counts_entries.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
            println!("{}: {:?}", n, counts_entries)
        }
    }
}
