use std::collections::{HashMap, HashSet};

use rand::{seq::SliceRandom, *};

#[allow(non_camel_case_types)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RoleGen {
    TOWN,
    COP,
    DOCTOR,
    CELEB,
    MILLER,
    MAFIA,
    GODFATHER,
    STRIPPER,
    GOON,
    IDIOT,
    GUARD_Town,
    GUARD_Mafia,
    AGENT_Town,
    AGENT_Mafia,
}

#[allow(dead_code)]
type RoleSet = HashSet<RoleGen>;

fn new_roleset() -> RoleSet {
    let mut roleset = HashSet::new();
    roleset.insert(RoleGen::TOWN);
    roleset.insert(RoleGen::MAFIA);
    roleset
}

fn minimal_roleset() -> RoleSet {
    let mut roleset = new_roleset();
    roleset.insert(RoleGen::COP);
    roleset.insert(RoleGen::DOCTOR);
    roleset
}

fn basic_roleset() -> RoleSet {
    let mut roleset = minimal_roleset();
    roleset.insert(RoleGen::CELEB);
    roleset.insert(RoleGen::MILLER);
    roleset.insert(RoleGen::GODFATHER);
    roleset.insert(RoleGen::STRIPPER);
    roleset.insert(RoleGen::IDIOT);
    roleset
}

fn full_roleset() -> RoleSet {
    let mut roleset = basic_roleset();
    roleset.insert(RoleGen::GOON);
    roleset.insert(RoleGen::GUARD_Town);
    roleset.insert(RoleGen::GUARD_Mafia);
    roleset.insert(RoleGen::AGENT_Town);
    roleset.insert(RoleGen::AGENT_Mafia);
    roleset
}

fn create_town_list(vanilla: usize) -> Vec<RoleGen> {
    let mut town_list = vec![RoleGen::TOWN; vanilla * 10];
    let mut l = vec![RoleGen::COP; 15];
    town_list.append(&mut l);
    let mut l = vec![RoleGen::DOCTOR; 15];
    town_list.append(&mut l);
    let mut l = vec![RoleGen::CELEB; 10];
    town_list.append(&mut l);
    let mut l = vec![RoleGen::MILLER; 10];
    town_list.append(&mut l);
    town_list
}

fn create_mafia_list(vanilla: usize) -> Vec<RoleGen> {
    let mut mafia_list = vec![RoleGen::MAFIA; 5 + vanilla * 10];
    let mut l = vec![RoleGen::GODFATHER; 15];
    mafia_list.append(&mut l);
    let mut l = vec![RoleGen::STRIPPER; 15];
    mafia_list.append(&mut l);
    let mut l = vec![RoleGen::GOON; 15];
    mafia_list.append(&mut l);
    mafia_list
}

fn create_rogue_list(roleset: &RoleSet) -> Vec<RoleGen> {
    let mut rogue_list = vec![];
    if roleset.contains(&RoleGen::IDIOT) {
        rogue_list.append(&mut vec![RoleGen::IDIOT; 40]);
    }
    if roleset.contains(&RoleGen::GUARD_Town) {
        rogue_list.append(&mut vec![RoleGen::GUARD_Town; 15]);
    }
    if roleset.contains(&RoleGen::GUARD_Mafia) {
        rogue_list.append(&mut vec![RoleGen::GUARD_Mafia; 15]);
    }
    if roleset.contains(&RoleGen::AGENT_Town) {
        rogue_list.append(&mut vec![RoleGen::AGENT_Town; 15]);
    }
    if roleset.contains(&RoleGen::AGENT_Mafia) {
        rogue_list.append(&mut vec![RoleGen::AGENT_Mafia; 15]);
    }

    if rogue_list.is_empty() {
        rogue_list.append(&mut vec![RoleGen::TOWN; 20]);
    }

    rogue_list
}

fn count_roles<F>(roles: &Vec<RoleGen>, f: F) -> usize
where
    F: Fn(&RoleGen) -> bool,
{
    roles.iter().filter(|&r| f(r)).count()
}

fn get_normal_boxmullar() -> (f64, f64) {
    let r: f64 = rand::random();
    let p: f64 = rand::random();
    let tmp: f64 = (-2.0 * r.ln()).sqrt();
    (
        tmp * f64::cos(p * 2.0 * std::f64::consts::PI),
        tmp * f64::sin(p * 2.0 * std::f64::consts::PI),
    )
}

fn get_normal_dist_rand(u: f64, s: f64) -> f64 {
    let (a, _) = get_normal_boxmullar();
    a * s + u
}

fn get_n_mafia(n: usize) -> usize {
    if n <= 5 {
        return 1;
    }
    loop {
        let r = get_normal_dist_rand(0.0, 1.5);
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
    let r = get_normal_dist_rand(3.0, 5.0);
    let m = ((n as f64).ln() * r) / 8.0;
    // print!("({}-{})", r, m);
    let i: i32 = unsafe { m.to_int_unchecked() };
    let i = i.max(0);
    i.try_into().unwrap()
}

fn get_rogue_roles(
    n_town: usize,
    n_mafia: usize,
    n_rogue: usize,
    roleset: &RoleSet,
) -> (i32, Vec<RoleGen>) {
    // TODO: Try 10 generations and take the best one (closest to 0 score)
    let mut rng = rand::thread_rng();
    let maf_score = (n_town as i32 - 1 - n_mafia as i32 * 2) * 10;
    let mut tries = 0;
    loop {
        let mut rogue_score = 0;
        let roles = create_rogue_list(roleset)
            .choose_multiple(&mut rng, n_rogue)
            .map(|r| *r)
            .collect();
        for role in &roles {
            rogue_score += match role {
                &RoleGen::IDIOT => -5,
                &RoleGen::GUARD_Town => 10,
                &RoleGen::GUARD_Mafia => -10,
                &RoleGen::AGENT_Town => -5,
                &RoleGen::AGENT_Mafia => 10,
                _ => 0,
            }
        }
        if (rogue_score < 0 && maf_score < 0 || rogue_score > 0 && maf_score > 0) && tries < 100 {
            tries += 1;
            continue;
        }
        return (rogue_score, roles);
    }
}

fn get_set_town_roles(n_town: usize, roleset: &RoleSet) -> Vec<RoleGen> {
    let mut town_roles = vec![
        RoleGen::COP,    // 1
        RoleGen::DOCTOR, // 2
        RoleGen::TOWN,   // 3
        RoleGen::MILLER, // 4
        RoleGen::TOWN,   // 5
        RoleGen::CELEB,  // 6
        RoleGen::MILLER, // 7
        RoleGen::COP,    // 8
        RoleGen::TOWN,   // 9
        RoleGen::DOCTOR, // 10
        RoleGen::TOWN,   // 11
        RoleGen::CELEB,  // 12
        RoleGen::TOWN,   // 13
        RoleGen::TOWN,   // 14
        RoleGen::MILLER, // 15
    ];
    if n_town > town_roles.len() {
        town_roles.extend(vec![RoleGen::TOWN; n_town - town_roles.len()]);
    } else {
        town_roles = town_roles[0..n_town].iter().map(|r| *r).collect();
    }
    for role in town_roles.iter_mut() {
        if !roleset.contains(role) {
            *role = RoleGen::TOWN;
        }
    }
    town_roles
}

fn get_set_mafia_roles(n_mafia: usize, roleset: &RoleSet) -> Vec<RoleGen> {
    let mut mafia_roles = vec![
        RoleGen::MAFIA,
        RoleGen::GODFATHER,
        RoleGen::STRIPPER,
        RoleGen::GOON,
    ];
    if n_mafia > mafia_roles.len() {
        mafia_roles.extend(vec![RoleGen::MAFIA; n_mafia - mafia_roles.len()]);
    } else {
        mafia_roles = mafia_roles[0..n_mafia].iter().map(|r| *r).collect();
    }
    for role in mafia_roles.iter_mut() {
        if !roleset.contains(role) {
            *role = RoleGen::MAFIA;
        }
    }
    mafia_roles
}

fn get_teams_score(n_town: usize, n_mafia: usize, n_rogue: usize) -> i32 {
    let n = -10 + ((n_town + n_mafia + n_rogue) % 2) as i32 * 10;
    (n_town as i32 - 1 - n_mafia as i32 * 2) * 10 + n
}

fn get_n_score(n_town: usize, n_mafia: usize, rogue_score: i32) -> i32 {
    (n_town as i32 - 1 - n_mafia as i32 * 2) * 10 + rogue_score
}

fn get_score_of_roles(roles: &Vec<RoleGen>) -> i32 {
    let mut score_mod = 0;

    for role in roles {
        score_mod += match role {
            RoleGen::TOWN | RoleGen::MAFIA => 0,
            RoleGen::COP | RoleGen::DOCTOR => 20,
            RoleGen::CELEB => 10,
            RoleGen::MILLER => 0,
            RoleGen::GODFATHER => 0,
            RoleGen::STRIPPER => -20,
            RoleGen::GOON => 25,
            _ => 0,
        }
    }
    score_mod
}

fn create_score_map() -> HashMap<RoleGen, i32> {
    vec![
        (RoleGen::TOWN, 0),
        (RoleGen::COP, 20),
        (RoleGen::DOCTOR, 20),
        (RoleGen::CELEB, 15),
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

fn get_score(roles: &Vec<RoleGen>) -> i32 {
    let mut score_map = create_score_map();
    let n_deceive = count_roles(roles, |r| matches!(r, RoleGen::GODFATHER | RoleGen::MILLER));
    if n_deceive > 0 {
        let cop_score = score_map.get(&RoleGen::COP).unwrap_or(&20);
        score_map.insert(
            RoleGen::COP,
            ((cop_score - 5) * 10) / 2i32.pow(n_deceive as u32) / 10 + 5,
        );
    }
    let n_strippers = count_roles(roles, |r| matches!(r, RoleGen::STRIPPER));
    if n_strippers > 0 {
        let cop_score = score_map.get(&RoleGen::COP).unwrap_or(&20);
        score_map.insert(
            RoleGen::COP,
            ((cop_score - 5) * 10) / 2i32.pow(n_deceive as u32) / 10 + 5,
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

fn get_score_mod(roles: &Vec<RoleGen>, new_roles: &Vec<RoleGen>) -> i32 {
    // Find Synergies in roles/new_roles for each new_roles.
    let mut score_mod = 0;
    for new_role in new_roles {
        score_mod += match new_role {
            RoleGen::COP => {
                let role_mod: i32 = 20;
                // Count MILLER/GF
                let bads: u32 = roles
                    .iter()
                    .chain(new_roles.iter())
                    .filter(|r| {
                        matches!(r, RoleGen::GODFATHER | RoleGen::MILLER | RoleGen::STRIPPER)
                    })
                    .count() as u32;
                // 0 -> 25, 1 -> 15, 2 -> 10, 3 -> 7, inf -> 5
                ((role_mod * 10) / (2i32.pow(bads))) / 10 + 5
            }
            RoleGen::DOCTOR => {
                let role_mod: i32 = 20;
                let bads: u32 = roles
                    .iter()
                    .chain(new_roles.iter())
                    .filter(|r| matches!(r, RoleGen::STRIPPER))
                    .count() as u32;
                // 0 -> 25, 1 -> 15, 2 -> 10, 3 -> 7, inf -> 5
                ((role_mod * 10) / (2i32.pow(bads))) / 10 + 5
            }
            RoleGen::CELEB => {
                let bads: u32 = roles
                    .iter()
                    .chain(new_roles.iter())
                    .filter(|r| matches!(r, RoleGen::STRIPPER))
                    .count() as u32;
                // 0 -> 10, 1 -> 6, 2 -> 4, 3 -> 3, inf -> 2
                ((80 / (2u32.pow(bads))) / 10 + 2) as i32
            }
            _ => {
                let r = vec![*new_role];
                get_score_of_roles(&r)
            }
        } as i32;
    }

    let synergy: i32 = roles
        .iter()
        .chain(new_roles.iter())
        .filter(|r| matches!(r, RoleGen::COP | RoleGen::DOCTOR | RoleGen::CELEB))
        .count()
        .try_into()
        .unwrap_or(0);
    score_mod += synergy * 5;

    return score_mod;
}

fn get_team_roles(n_town: usize, n_mafia: usize, rogue_score: i32) -> (i32, Vec<RoleGen>) {
    let mut rng = rand::thread_rng();
    let mut roles = Vec::new();
    // Get the ratio bewteen town and maf.
    let maf_score = get_n_score(n_town, n_mafia, rogue_score);

    let mut score = maf_score + rogue_score;

    let mut n_town_chosen = 0;
    let mut n_maf_chosen = 0;

    loop {
        let mut temp_roles = Vec::new();
        let n_town_to_choose = n_town - n_town_chosen;
        let n_mafia_to_choose = n_mafia - n_maf_chosen;
        let t: usize;
        let m: usize;
        if true {
            t = n_town_to_choose;
            m = n_mafia_to_choose;
        } else if n_town_to_choose >= 3 && n_mafia_to_choose >= 2 {
            t = 3;
            m = 2;
        } else if n_town_to_choose >= 2 && n_mafia_to_choose >= 1 {
            t = 2;
            m = 1;
        } else if n_mafia_to_choose >= 1 {
            t = 0;
            m = 1;
        } else if n_town_to_choose >= 1 {
            t = 1;
            m = 0;
        } else {
            break;
        }

        temp_roles.extend(create_town_list(8).choose_multiple(&mut rng, t));
        temp_roles.extend(create_mafia_list(15).choose_multiple(&mut rng, m));

        let score_mod = get_score_mod(&roles, &temp_roles);
        let r = get_normal_dist_rand(0.0, 10.0).round() as i32;
        if (score_mod + score + r).abs() < 10 {
            // println!("{} += {}+{}:  +{:?}", score, score_mod, r, temp_roles);
            score += score_mod;
            roles.extend(temp_roles);
            n_town_chosen += t;
            n_maf_chosen += m;
            break;
        } else {
            // println!("XXX: {} += {}+{}: +{:?}", score, score_mod, r, temp_roles);
        }
    }

    (score, roles)
}

/*
let r = get_normal_dist_rand(3.0, 6.0);
    let m = ((n as f64).ln() * r) / 8.0;
    // print!("({}-{})", r, m);
    let i: i32 = unsafe { m.to_int_unchecked() };
    let i = i.max(0);
    i.try_into().unwrap()
*/

fn get_n_normal<F>(u: f64, s: f64, f: F) -> usize
where
    F: FnOnce(f64) -> usize,
{
    let r = get_normal_dist_rand(u, s);
    f(r)
}

fn get_team_roles_normals(
    n_town: usize,
    n_mafia: usize,
    _: i32,
    roleset: &RoleSet,
) -> (i32, Vec<RoleGen>) {
    let mut rng = rand::thread_rng();

    let mut n_cop = 0;
    let mut n_doctor = 0;
    if roleset.contains(&RoleGen::COP) && roleset.contains(&RoleGen::DOCTOR) {
        let (c, d) = get_n_cop_doc(n_town);
        n_cop += c;
        n_doctor += d;
    } else {
        if roleset.contains(&RoleGen::COP) {
            n_cop += get_n_cop(n_town);
        }
        if roleset.contains(&RoleGen::DOCTOR) {
            n_doctor += get_n_doctor(n_town);
        }
    }
    let mut n_celeb = 0;
    if roleset.contains(&RoleGen::CELEB) {
        n_celeb += get_n_celeb(n_town);
    }
    let mut n_miller = 0;
    if roleset.contains(&RoleGen::MILLER) {
        n_miller += get_n_miller(n_town, n_cop);
    }

    let mut roles = Vec::new();
    for _ in 0..n_cop {
        roles.push(RoleGen::COP);
    }
    for _ in 0..n_doctor {
        roles.push(RoleGen::DOCTOR);
    }
    for _ in 0..n_celeb {
        roles.push(RoleGen::CELEB);
    }
    for _ in 0..n_miller {
        roles.push(RoleGen::MILLER);
    }
    if roles.len() > n_town {
        roles = roles
            .choose_multiple(&mut rng, n_town)
            .map(|r| *r)
            .collect();
    } else {
        let n_more = n_town - roles.len();
        for _ in 0..n_more {
            roles.push(RoleGen::TOWN);
        }
    }
    for _ in 0..n_mafia {
        roles.push(RoleGen::MAFIA);
    }
    (0, roles)
}

fn get_n_cop_doc(n_town: usize) -> (usize, usize) {
    let n = get_n_normal(2.0, 1.6, |r| ((n_town as f64 + 3.0) * r / 12.0) as usize);
    // choose dice
    // 7 -> 4,3
    if n == 0 {
        return (0, 0);
    }
    let d1 = n / 2 + n % 2;
    let d2 = n / 2;
    let mut rng = rand::thread_rng();
    let c = rng.gen_range(0..=d1) + rng.gen_range(0..=d2);
    let d = n - c;
    (c, d)
}
fn get_n_cop(n_town: usize) -> usize {
    let k = 1.0;
    get_n_normal(k * 1.5, 0.8, |r| {
        ((n_town as f64 + 4.0) * r / 11.0) as usize
    })
}
fn get_n_doctor(n_town: usize) -> usize {
    get_n_normal(1.5, 0.8, |r| ((n_town as f64 + 4.0) * r / 11.0) as usize)
}
fn get_n_celeb(n_town: usize) -> usize {
    get_n_normal(2.5, 1.8, |r| {
        (((n_town as f64).ln() + 1.0) * r / 9.0) as usize
    })
}
fn get_n_miller(n_town: usize, n_cop: usize) -> usize {
    get_n_normal(2.8, 2.2, |r| {
        (((n_town as f64) / 6.0 + (n_cop as f64) / 1.5 + 0.6) * r / 9.0) as usize
    })
}

fn create_spice_list(roleset: &RoleSet) -> Vec<RoleGen> {
    let mut spice_list = vec![];
    for role in roleset {
        let mut addition = match role {
            RoleGen::COP => vec![RoleGen::COP; 20],
            RoleGen::DOCTOR => vec![RoleGen::DOCTOR; 20],
            RoleGen::CELEB => vec![RoleGen::CELEB; 15],
            RoleGen::MILLER => vec![RoleGen::MILLER; 15],
            RoleGen::GODFATHER => vec![RoleGen::GODFATHER; 15],
            RoleGen::STRIPPER => vec![RoleGen::STRIPPER; 20],
            RoleGen::GOON => vec![RoleGen::GOON; 15],
            RoleGen::IDIOT => vec![RoleGen::IDIOT; 10],
            RoleGen::GUARD_Town => vec![RoleGen::GUARD_Town; 15],
            RoleGen::GUARD_Mafia => vec![RoleGen::GUARD_Mafia; 10],
            RoleGen::AGENT_Town => vec![RoleGen::AGENT_Town; 15],
            RoleGen::AGENT_Mafia => vec![RoleGen::AGENT_Mafia; 10],
            _ => vec![],
        };
        spice_list.append(&mut addition);
    }
    spice_list
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
            if tries >= 100 {
                return Err(());
            }
            continue;
        }
        return Ok(roles);
    }
}

fn get_n_spice(n_players: usize, spice: f64) -> usize {
    get_n_normal(1.0, 0.25, |r| {
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

    let team_score = get_teams_score(n_town, n_mafia, n_rogue);

    let mut best_set = gen_role_set(n_town, n_mafia, n_rogue, n_spice, roleset, rng)?;
    let mut best_score = get_score(&best_set) + team_score;

    for _ in 0..(2 * n) {
        // Generate a set
        let set = gen_role_set(n_town, n_mafia, n_rogue, n_spice, roleset, rng)?;
        // Check score
        let score = get_score(&set) + team_score;
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

fn get_roles_normals(n: usize, roleset: &RoleSet) -> (i32, Vec<RoleGen>) {
    let n_mafia = get_n_mafia(n);
    let n_rogue = get_n_rogue(n - n_mafia);
    let n_town = n - n_mafia - n_rogue;
    // println!("{}v{},{}", n_town, n_mafia, n_rogue);
    let (rogue_score, rogue_roles) = get_rogue_roles(n_town, n_mafia, n_rogue, roleset);
    let (score, team_roles) = get_team_roles_normals(n_town, n_mafia, rogue_score, roleset);

    (score, vec![team_roles, rogue_roles].concat())
}

fn get_roles(n: usize) -> (i32, Vec<RoleGen>) {
    // For even, add 1 TOWN?
    let n_mafia = get_n_mafia(n);
    let n_rogue = get_n_rogue(n - n_mafia);
    let n_town = n - n_mafia - n_rogue;
    println!("{}v{},{}", n_town, n_mafia, n_rogue);
    let roleset = new_roleset();
    let (rogue_score, rogue_roles) = get_rogue_roles(n_town, n_mafia, n_rogue, &roleset);
    let (score, team_roles) = get_team_roles(n_town, n_mafia, rogue_score);
    (score, vec![team_roles, rogue_roles].concat())
}

fn get_roles_set(n: usize, roleset: &RoleSet) -> Vec<RoleGen> {
    let n_mafia = get_n_mafia(n);
    let n_rogue = get_n_rogue(n - n_mafia);
    let n_town = n - n_mafia - n_rogue;
    // println!("{}v{},{}", n_town, n_mafia, n_rogue);
    let (_rogue_score, rogue_roles) = get_rogue_roles(n_town, n_mafia, n_rogue, roleset);
    let town_roles = get_set_town_roles(n_town, roleset);
    // println!("..");
    let mafia_roles = get_set_mafia_roles(n_mafia, roleset);
    // println!("...");
    vec![town_roles, mafia_roles, rogue_roles].concat()
}

mod test {
    #[allow(unused_imports)]
    use super::*;
    use std::collections::HashMap;

    #[test]
    #[ignore]
    fn basic() {
        for n in 5..=15 {
            if n % 2 == 1 {
                for _ in 0..3 {
                    print!("{}: ", n);
                    println!("{}: {:?}", n, get_roles(n))
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn basic_normal() {
        let mut roleset = new_roleset();
        roleset.extend(vec![RoleGen::COP, RoleGen::DOCTOR]);
        for n in 5..=15 {
            for _ in 0..3 {
                println!("{}: {:?}", n, get_roles_normals(n, &roleset))
            }
        }
    }

    #[test]
    #[ignore]
    fn basic_set() {
        let roleset = full_roleset();
        for n in 5..=15 {
            for _ in 0..3 {
                println!("{}: {:?}", n, get_roles_set(n, &roleset))
            }
        }
    }

    #[test]
    #[ignore]
    fn gen_example_n_cop() {
        for n_town in 2..=15 {
            let mut counts = HashMap::new();
            // let mut counts_doc = HashMap::new();
            for _ in 0..10000 {
                let n_cop_doc = get_n_cop_doc(n_town);
                // let n_doc = get_n_doctor(n_town);
                let v = counts.get(&n_cop_doc).unwrap_or(&0);
                counts.insert(n_cop_doc, v + 1);
                // let v = counts_doc.get(&n_doc).unwrap_or(&0);
                // counts_doc.insert(n_doc, v + 1);
            }
            let mut counts_sorted: Vec<_> = counts.into_iter().collect();
            counts_sorted.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
            // let mut counts_doc_sorted: Vec<_> = counts_doc.into_iter().collect();
            // counts_doc_sorted.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
            println!("{}: {:?}\n", n_town, counts_sorted);
            // println!("{}: {:?}", n_town, counts_doc_sorted);
        }
    }

    #[test]
    #[ignore]
    fn gen_example_n_miller() {
        for n_cop in 0..=3 {
            for n_town in 3..=15 {
                let mut counts = HashMap::new();
                for _ in 0..1000 {
                    let n_miller = get_n_miller(n_town, n_cop);
                    let v = counts.get(&n_miller).unwrap_or(&0);
                    counts.insert(n_miller, v + 1);
                }

                let mut counts_sorted: Vec<_> = counts.into_iter().collect();
                counts_sorted.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
                println!("{}, {}: {:?}", n_town, n_cop, counts_sorted);
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
        for n_players in 3..15 {
            let (score, roles) =
                get_roles_spice(n_players, spice, &roleset, &mut rng).expect("Should not fail?");
            println!("{}: ({}){:?}", n_players, score, roles);
        }
    }
}
