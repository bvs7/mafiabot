use std::collections::{hash_set, HashMap, HashSet};

use rand::{seq::SliceRandom, *};

mod spice;

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
    SURVIVOR,
    GUARD,
    GUARD_Mafia,
    AGENT,
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
    roleset.insert(RoleGen::GUARD);
    roleset.insert(RoleGen::GUARD_Mafia);
    roleset.insert(RoleGen::AGENT);
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
    let mut l = vec![RoleGen::GODFATHER; 2];
    mafia_list.append(&mut l);
    let mut l = vec![RoleGen::STRIPPER; 3];
    mafia_list.append(&mut l);
    let mut l = vec![RoleGen::GOON; 1];
    mafia_list.append(&mut l);
    mafia_list
}

fn create_rogue_list(roleset: &RoleSet) -> Vec<RoleGen> {
    let mut rogue_list = vec![];
    if roleset.contains(&RoleGen::IDIOT) {
        rogue_list.append(&mut vec![RoleGen::IDIOT; 40]);
    }
    if roleset.contains(&RoleGen::GUARD) {
        rogue_list.append(&mut vec![RoleGen::GUARD; 15]);
    }
    if roleset.contains(&RoleGen::GUARD_Mafia) {
        rogue_list.append(&mut vec![RoleGen::GUARD_Mafia; 10]);
    }
    if roleset.contains(&RoleGen::AGENT) {
        rogue_list.append(&mut vec![RoleGen::AGENT; 15]);
    }
    if roleset.contains(&RoleGen::AGENT_Mafia) {
        rogue_list.append(&mut vec![RoleGen::AGENT_Mafia; 10]);
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

fn get_rogue_roles(n_rogue: usize, start_score: i32, roleset: &RoleSet) -> Vec<RoleGen> {
    // TODO: Try 10 generations and take the best one (closest to 0 score)
    let mut rng = rand::thread_rng();
    let maf_score = start_score;
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
                &RoleGen::GUARD => 10,
                &RoleGen::GUARD_Mafia => -10,
                &RoleGen::AGENT => -5,
                &RoleGen::AGENT_Mafia => 10,
                _ => 0,
            }
        }
        if (rogue_score < 0 && maf_score < 0 || rogue_score > 0 && maf_score > 0) && tries < 100 {
            tries += 1;
            continue;
        }
        return roles;
    }
}

fn get_spicy_team_roles(n_spicy_town: usize, n_spicy_mafia: usize) -> Vec<RoleGen> {
    let mut rng = rand::thread_rng();
    let mut roles = Vec::new();
    // Get number of spicy roles
    roles.extend(create_town_list(0).choose_multiple(&mut rng, n_spicy_town));
    roles.extend(create_mafia_list(0).choose_multiple(&mut rng, n_spicy_mafia));

    roles
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

fn get_team_roles_normals(n_town: usize, n_mafia: usize, roleset: &RoleSet) -> (i32, Vec<RoleGen>) {
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

fn get_roles_normals(n: usize, roleset: &RoleSet) -> (i32, Vec<RoleGen>) {
    let n_mafia = get_n_mafia(n);
    let n_rogue = get_n_rogue(n - n_mafia);
    let n_town = n - n_mafia - n_rogue;
    // println!("{}v{},{}", n_town, n_mafia, n_rogue);
    let team_score = (n_town as i32 - 1 - n_mafia as i32 * 2) * 10;
    let rogue_roles = get_rogue_roles(n_rogue, team_score, roleset);
    let (score, team_roles) = get_team_roles_normals(n_town, n_mafia, roleset);

    (score, vec![team_roles, rogue_roles].concat())
}

mod test {}
