use super::{RoleGen, RoleSet};
use rand::seq::SliceRandom;

mod normal {
    fn get_normal_boxmullar() -> (f64, f64) {
        let r: f64 = rand::random();
        let p: f64 = rand::random();
        let tmp: f64 = (-2.0 * r.ln()).sqrt();
        (
            tmp * f64::cos(p * 2.0 * std::f64::consts::PI),
            tmp * f64::sin(p * 2.0 * std::f64::consts::PI),
        )
    }

    pub fn get_normal_dist_rand(u: f64, s: f64) -> f64 {
        let (a, _) = get_normal_boxmullar();
        a * s + u
    }

    pub fn get_n_normal<F>(u: f64, s: f64, f: F) -> usize
    where
        F: FnOnce(f64) -> usize,
    {
        let r = get_normal_dist_rand(u, s);
        f(r)
    }
}

fn get_n_mafia(n: usize) -> usize {
    if n <= 5 {
        return 1;
    }
    loop {
        let r = normal::get_normal_dist_rand(0.0, 1.5);
        let m = ((n as f64) + r) / 3.0;
        let i: i32 = unsafe { m.to_int_unchecked() };
        if i < 1 {
            continue;
        }
        if i > n as i32 / 2 - 1 {
            continue;
        }
        return i.try_into().unwrap();
    }
}

fn get_n_rogue(n: usize) -> usize {
    let r = normal::get_normal_dist_rand(3.0, 5.0);
    let m = ((n as f64).ln() * r) / 8.0;
    // print!("({}-{})", r, m);
    let i: i32 = unsafe { m.to_int_unchecked() };
    let i = i.max(0);
    i.try_into().unwrap()
}

fn count_roles<F>(roles: &Vec<RoleGen>, f: F) -> usize
where
    F: Fn(&RoleGen) -> bool,
{
    roles.iter().filter(|&r| f(r)).count()
}

fn create_spice_list(roleset: &RoleSet) -> Vec<RoleGen> {
    let mut spice_list = vec![];
    for role in roleset {
        let mut addition = match role {
            RoleGen::COP => vec![RoleGen::COP; 30],
            RoleGen::DOCTOR => vec![RoleGen::DOCTOR; 30],
            RoleGen::CELEB => vec![RoleGen::CELEB; 20],
            RoleGen::MILLER => vec![RoleGen::MILLER; 10],
            RoleGen::GODFATHER => vec![RoleGen::GODFATHER; 10],
            RoleGen::STRIPPER => vec![RoleGen::STRIPPER; 25],
            RoleGen::GOON => vec![RoleGen::GOON; 20],
            RoleGen::IDIOT => vec![RoleGen::IDIOT; 10],
            RoleGen::GUARD_Town => vec![RoleGen::GUARD_Town; 8],
            RoleGen::GUARD_Mafia => vec![RoleGen::GUARD_Mafia; 5],
            RoleGen::AGENT_Town => vec![RoleGen::AGENT_Town; 8],
            RoleGen::AGENT_Mafia => vec![RoleGen::AGENT_Mafia; 5],
            _ => vec![],
        };
        spice_list.append(&mut addition);
    }
    spice_list
}

mod score {
    use std::collections::HashMap;

    use super::*;

    pub fn get_teams_score(n_town: usize, n_mafia: usize, n_rogue: usize) -> i32 {
        let n = -8 + ((n_town + n_mafia + n_rogue) % 2) as i32 * 8;
        (n_town as i32 - 1 - n_mafia as i32 * 2) * 10 + n
    }

    pub fn get_town_v_maf_score(n_players: usize, n_mafia: usize) -> i32 {
        // Things to consider:
        // Number of successes for town to win
        let kill_maf_needed = n_mafia as i32;
        // Number of successes for mafia to win
        let kill_town_needed = ((n_players - n_mafia) / 2) as i32;
        // How likely is town to hit maf?
        let kill_maf_difficulty = 15;
        let kill_town_difficulty = 10;

        kill_town_needed * kill_town_difficulty - kill_maf_needed * kill_maf_difficulty
    }

    pub fn get_roles_score(roles: &Vec<RoleGen>) -> i32 {
        // Certain roles have certain effectiveness based on what others exist?
        // Score map? adjust down various roles based on what other roles exist.
        // A Stripper counteracts 1.5 town roles but only if they exist!
        // Goon helps town less and less with each maf
        // When generating spicy roles...
        // Choose one role at a time. Certain roles already chosen change the value of future roles?
        // Weights for choices based on..?
        //   Current roles+team score and how new role would affect.

        // Choose maf and rogue first
        // Three components to score:
        // - Team (# of town and maf)
        // - Role (Town and Maf Roles).
        // - Rogue (Rogue Roles)
        // These can kinda be chosen separately? But should be decided together...
        // - Team - Can be positive or negative, but shouldn't be too far
        // - Role - Depends on available roles!!! Usually only positive.
        // - Rogue - Depends on available roles. Can swing either way.
        // Bifurcate score three ways, then try to close in?

        // Inputs. number of players. spiciness. fairness?
        // Fairness affects how many attempts we try.
        // Spiciness is the number of spicy roles

        // Choose # of rogue first...
        // Using spiciness, generate

        0
    }

    fn create_score_map() -> HashMap<RoleGen, i32> {
        vec![
            (RoleGen::TOWN, 0),
            (RoleGen::COP, 16),
            (RoleGen::DOCTOR, 16),
            (RoleGen::CELEB, 12),
            (RoleGen::MILLER, 0),
            (RoleGen::MAFIA, 0),
            (RoleGen::GODFATHER, 0),
            (RoleGen::STRIPPER, -20),
            (RoleGen::GOON, 20),
            (RoleGen::IDIOT, -5),
            (RoleGen::GUARD_Town, 10),
            (RoleGen::GUARD_Mafia, -10),
            (RoleGen::AGENT_Town, -10),
            (RoleGen::AGENT_Mafia, 10),
        ]
        .into_iter()
        .collect()
    }

    pub fn get_score(roles: &Vec<RoleGen>) -> i32 {
        let mut score_map = create_score_map();
        let n_deceive = count_roles(roles, |r| matches!(r, RoleGen::GODFATHER | RoleGen::MILLER));
        if n_deceive > 0 {
            let cop_score = score_map.get(&RoleGen::COP).unwrap_or(&20);
            score_map.insert(
                RoleGen::COP,
                ((cop_score - 8) * 10) / 2i32.pow(n_deceive as u32) / 10 + 8,
            );
        }
        let n_strippers = count_roles(roles, |r| matches!(r, RoleGen::STRIPPER));
        if n_strippers > 0 {
            let cop_score = score_map.get(&RoleGen::COP).unwrap_or(&20);
            score_map.insert(
                RoleGen::COP,
                ((cop_score - 8) * 10) / 2i32.pow(n_deceive as u32) / 10 + 8,
            );
            let doc_score = score_map.get(&RoleGen::DOCTOR).unwrap_or(&20);
            score_map.insert(
                RoleGen::DOCTOR,
                ((doc_score - 5) * 10) / 2i32.pow(n_deceive as u32) / 10 + 5,
            );
            let celeb_score = score_map.get(&RoleGen::CELEB).unwrap_or(&20);
            score_map.insert(
                RoleGen::CELEB,
                ((celeb_score - 8) * 10) / 2i32.pow(n_deceive as u32) / 10 + 8,
            );
        }
        roles.iter().map(|r| score_map[r]).sum()
    }
}

fn gen_role_set(
    max_town: usize,
    max_mafia: usize,
    max_rogue: usize,
    n: usize,
    roleset: &RoleSet,
    rng: &mut rand::rngs::ThreadRng,
) -> Result<Vec<RoleGen>, ()> {
    let mut tries = 0;
    loop {
        let roles: Vec<RoleGen> = create_spice_list(roleset)
            .choose_multiple(rng, n)
            .map(|r| *r)
            .collect();
        // Check role counts first
        let n_town = roles
            .iter()
            .filter(|r| {
                matches!(
                    r,
                    RoleGen::COP | RoleGen::DOCTOR | RoleGen::CELEB | RoleGen::MILLER
                )
            })
            .count();

        let n_mafia = roles
            .iter()
            .filter(|r| matches!(r, RoleGen::GODFATHER | RoleGen::STRIPPER | RoleGen::GOON))
            .count();

        let n_rogue = roles
            .iter()
            .filter(|r| {
                matches!(
                    r,
                    RoleGen::IDIOT
                        | RoleGen::GUARD_Town
                        | RoleGen::GUARD_Mafia
                        | RoleGen::AGENT_Town
                        | RoleGen::AGENT_Mafia
                )
            })
            .count();

        if n_town > max_town || n_mafia > max_mafia || n_rogue > max_rogue {
            tries += 1;
            if tries >= 1000 {
                // TODO: Have fallback??
                return Err(());
            }
            continue;
        }
        return Ok(roles);
    }
}

fn get_n_spice(n_players: usize, spice: f64) -> usize {
    normal::get_n_normal(1.0, 0.25, |r| {
        let u = (r * spice * (n_players as f64) + 0.5) as usize;
        u.max(0).min(n_players)
    })
}

fn get_roles_spice(
    n: usize,
    spice: f64,
    roleset: &RoleSet,
    rng: &mut rand::rngs::ThreadRng,
) -> Result<(i32, Vec<RoleGen>), ()> {
    let mut n_rogue = get_n_rogue(n);
    let n_mafia = get_n_mafia(n);
    let mut n_town = n - n_mafia - n_rogue;
    if n_town == 0 {
        n_town = n_rogue;
        n_rogue = 0;
    }
    let n_spice = get_n_spice(n, spice);

    let team_score = score::get_town_v_maf_score(n, n_mafia);

    let mut best_set = gen_role_set(n_town, n_mafia, n_rogue, n_spice, roleset, rng)?;
    let mut best_score = score::get_score(&best_set) + team_score;

    for _ in 0..(2 * n) {
        // Generate a set
        let set = gen_role_set(n_town, n_mafia, n_rogue, n_spice, roleset, rng)?;
        // Check score
        let score = score::get_score(&set) + team_score;
        // Compare score
        if best_score.abs() > score.abs() {
            best_score = score;
            best_set = set;
        }
    }

    let current_town = count_roles(&best_set, |r| {
        matches!(
            r,
            RoleGen::TOWN | RoleGen::COP | RoleGen::DOCTOR | RoleGen::CELEB | RoleGen::MILLER
        )
    });
    let current_mafia = count_roles(&best_set, |r| {
        matches!(
            r,
            RoleGen::MAFIA | RoleGen::GODFATHER | RoleGen::STRIPPER | RoleGen::GOON
        )
    });
    let current_rogue = count_roles(&best_set, |r| {
        matches!(
            r,
            RoleGen::IDIOT
                | RoleGen::GUARD_Town
                | RoleGen::GUARD_Mafia
                | RoleGen::AGENT_Town
                | RoleGen::AGENT_Mafia
        )
    });

    let mut extra_town = vec![RoleGen::TOWN; n_town + n_rogue - current_town - current_rogue];
    let mut extra_mafia = vec![RoleGen::MAFIA; n_mafia - current_mafia];

    best_set.append(&mut extra_town);
    best_set.append(&mut extra_mafia);

    Ok((best_score, best_set))
}

mod test {
    use super::super::*;
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_team_score() {
        for i in 3..=15 {
            for j in 1..=i / 3 {
                println!("{},{}: {}", i, j, score::get_town_v_maf_score(i, j));
            }
        }
    }

    #[test]
    fn gen_example_n_spice() {
        let spice = 0.33;
        for n_players in 3..15 {
            let mut counts = HashMap::new();
            for _ in 0..10000 {
                let n_special = get_n_spice(n_players, spice);
                let v = counts.get(&n_special).unwrap_or(&0);
                counts.insert(n_special, v + 1);
            }

            let mut counts_sorted: Vec<_> = counts.into_iter().collect();
            counts_sorted.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
            println!("{}: {:?}", n_players, counts_sorted);
        }
    }

    #[test]
    fn gen_sets_n_spice() {
        let spice = 0.5;
        let roleset = full_roleset();
        let mut rng = rand::thread_rng();
        for n_players in 7..=7 {
            for _ in 0..10 {
                let (score, roles) = get_roles_spice(n_players, spice, &roleset, &mut rng)
                    .expect("Should not fail?");
                println!("{}: ({}){:?}", n_players, score, roles);
            }
        }
    }
}
