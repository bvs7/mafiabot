use std::{arch::x86_64, thread::Thread};

use rand::{
    distributions::Open01, prelude::Distribution, rngs::ThreadRng, seq::SliceRandom, thread_rng,
    Rng,
};
use statrs::distribution::Poisson;

use crate::prelude::*;

use super::RoleGen;

const MIN_P: f64 = 0.2;
const MAX_P: f64 = 0.5;

// The idea behind draw.rs is that it generates a bag of roles to choose from, then
// chooses from the bag one by one. This is represented by a weighted list, and the
// weights are modified after each draw.

const CHARGE_TOWN: Pid = Pid(0);
const CHARGE_MAFIA: Pid = Pid(1);
const CHARGE_ANY: Pid = Pid(2);

const TOWN_BASE_WEIGHTS: [(Role, f64, f64); 5] = [
    (Role::COP, 10.0, 0.4),
    (Role::DOCTOR, 10.0, 0.4),
    (Role::CELEB, 10.0, 0.8),
    (Role::MILKY, 5.0, 0.5),
    (Role::GOON, 3.0, 0.5),
];

const MAFIA_BASE_WEIGHTS: [(Role, f64, f64); 1] = [(Role::STRIPPER, 10.0, 0.5)];

// When to add goon? Definitely when there are above a certain number of players.

const ROGUE_BASE_WEIGHTS: [(Role, f64, f64); 8] = [
    (Role::IDIOT, 25.0, 0.5),
    (Role::SURVIVOR, 2.0, 0.5),
    (Role::GUARD(CHARGE_TOWN), 3.0, 0.5),
    (Role::GUARD(CHARGE_MAFIA), 0.5, 0.5),
    (Role::GUARD(CHARGE_ANY), 0.5, 0.75),
    (Role::AGENT(CHARGE_TOWN), 3.0, 0.5),
    (Role::AGENT(CHARGE_MAFIA), 0.5, 0.5),
    (Role::AGENT(CHARGE_ANY), 0.5, 0.75),
];

// Inputs?

struct DrawRoleGen {
    rules: Rules,
    rng: ThreadRng,
}

impl RoleGen for DrawRoleGen {
    fn generate_roles(&mut self, users: Vec<impl Into<Pid>>, rules: &Rules) -> Vec<(Pid, Role)> {
        let n = users.len();

        let rogue_roles = self.draw_rogue(n);
        let n_rogue = rogue_roles.len();
        // Get maf strength of rogue roles
        let sigma_rogue: f64 = rogue_roles.iter().map(|r| r.sigma_adj()).sum();
        // Generate sigma for remaining n players
        let n_left = n - n_rogue;
        let x_0 = self.target_p() / 2.0 * (n_left as f64 + 1.0) - sigma_rogue;
        let sigma_maf = sigmoid_rv(x_0, &mut self.rng);
        let sigma = sigma_rogue + sigma_maf;
        let mut n_maf = sigma_maf.round() as usize;
        if n_maf == 0 {
            n_maf = 1;
        }
        let n_town = n_left - n_maf;
        debug!("N mafia: {n_maf}");
        let mut town_roles = self.draw_town(n_town, sigma);
        while town_roles.len() < n_town {
            town_roles.push(Role::TOWN);
        }
        let mut mafia_roles = self.draw_mafia(n_maf);
        while mafia_roles.len() < n_maf {
            mafia_roles.push(Role::MAFIA);
        }
        let mut roles = rogue_roles
            .into_iter()
            .chain(town_roles.into_iter())
            .chain(mafia_roles.into_iter())
            .collect::<Vec<_>>();
        self.draw_mislead(n, &mut roles);
        roles.sort();

        self.assign_roles(users, roles)
    }
}

impl DrawRoleGen {
    fn target_p(&self) -> f64 {
        let mid = (MAX_P + MIN_P) / 2.0;

        self.rules.kink as f64 / 100.0 * (MAX_P - MIN_P) + mid
    }
}

impl Role {
    // The amounts these roles are worth in number of mafia
    fn sigma_adj(&self) -> f64 {
        match self {
            Role::IDIOT => 0.75,
            Role::SURVIVOR => 0.25,
            Role::GUARD(CHARGE_TOWN) => -0.25,
            Role::GUARD(CHARGE_MAFIA) => 0.75,
            Role::GUARD(CHARGE_ANY) => 0.0,
            Role::AGENT(CHARGE_TOWN) => 0.5,
            Role::AGENT(CHARGE_MAFIA) => -0.75,
            Role::AGENT(CHARGE_ANY) => -0.5,
            _ => 0.0,
        }
    }
}

fn get_n_poisson(rate: f64, rng: &mut impl Rng) -> usize {
    let poisson = Poisson::new(rate).unwrap();
    let x: u64 = poisson.sample(rng);
    return x as usize;
}

fn sq_get_n_poisson(rate: f64, rng: &mut impl Rng) -> usize {
    let mut x = get_n_poisson(rate, rng);
    for _ in 0..100 {
        let x2 = get_n_poisson(rate, rng);
        if x == x2 {
            return x;
        }
        x = x2;
    }
    return x;
}

fn sigmoid_rv(x_0: f64, rng: &mut impl Rng) -> f64 {
    let x: f64 = rng.sample(Open01);
    let s = (-f64::ln(1.0 / x - 1.0) / 10.0 + 1.0) * x_0;
    return s;
}

fn pull_bag(
    bag: &mut Vec<(Role, f64, f64)>,
    dec: fn(f64, f64) -> f64,
    rng: &mut impl Rng,
) -> Option<Role> {
    if bag.is_empty() {
        return None;
    }
    let pick = bag.choose_weighted_mut(rng, |e| e.1).unwrap();
    pick.1 = dec(pick.1, pick.2);
    let result = pick.0.clone();
    if pick.1 <= 0.0 {
        bag.retain(|e| e.1 > 0.0);
    }
    Some(result)
}

impl DrawRoleGen {
    fn draw_rogue(&mut self, n: usize) -> Vec<Role> {
        let n_rogue = get_n_poisson(n as f64 * self.rules.rogue as f64 / 100.0, &mut self.rng);
        // Ensure we don't have too many rogues
        if n_rogue > n / 3 {
            return Vec::new();
        }
        // Construct the bag of roles
        let mut bag = ROGUE_BASE_WEIGHTS.to_vec();

        let dec = |w, g| ((w - 1.0) * g);

        bag.retain(|(role, _, _)| self.rules.allowed_roles.contains(&role.kind()));

        // TODO: guaranteed roles are removed first...
        let mut rogue_roles = Vec::new();
        for _ in 0..n_rogue {
            let Some(role) = pull_bag(&mut bag, dec, &mut self.rng) else {
                break;
            };
            rogue_roles.push(role);
        }
        rogue_roles
    }

    fn draw_town(&mut self, n_town: usize, sigma: f64) -> Vec<Role> {
        let mut bag = TOWN_BASE_WEIGHTS.to_vec();
        bag.retain(|(role, _, _)| self.rules.allowed_roles.contains(&role.kind()));

        let dec = |w, g| ((w - 1.0) * g);

        // Rate we want is... Kink / 100.0 * n_tot around there. sigma is about 1/5 n_tot...

        let rate = (self.rules.kink as f64 / 100.0) * 5.0 * sigma;
        let mut n_pick = sq_get_n_poisson(rate, &mut self.rng);
        if n_pick > n_town - 3 {
            n_pick = n_town - 3;
        }
        debug!("N power roles: {n_pick}");

        let mut town_roles = Vec::new();

        for (r, w, g) in bag.iter_mut() {
            if let Some(k) = self.rules.guaranteed_roles.get(&r.kind()) {
                for _ in 0..*k {
                    town_roles.push(r.clone());
                    *w = dec(*w, *g);
                }
            }
        }
        debug!("Town bag: {:?}", bag);
        let j = town_roles.len();
        for _ in j..n_pick {
            let Some(role) = pull_bag(&mut bag, dec, &mut self.rng) else {
                break;
            };
            town_roles.push(role);
        }
        debug!("Town bag: {:?}", bag);
        town_roles
    }

    fn draw_mafia(&mut self, n_maf: usize) -> Vec<Role> {
        let mut bag = MAFIA_BASE_WEIGHTS.to_vec();
        bag.retain(|(role, _, _)| self.rules.allowed_roles.contains(&role.kind()));

        let dec = |w, g| ((w - 1.0) * g);

        let rate = (self.rules.kink as f64 / 100.0) * n_maf as f64;

        let n_pick = get_n_poisson(rate, &mut self.rng);

        let mut mafia_roles = Vec::new();

        for (r, w, g) in bag.iter_mut() {
            if let Some(k) = self.rules.guaranteed_roles.get(&r.kind()) {
                for _ in 0..*k {
                    mafia_roles.push(r.clone());
                    *w = dec(*w, *g);
                }
            }
        }

        for _ in mafia_roles.len()..n_pick {
            let Some(role) = pull_bag(&mut bag, dec, &mut self.rng) else {
                break;
            };
            mafia_roles.push(role);
        }
        mafia_roles
    }

    fn add_mislead(&mut self, roles: &mut Vec<Role>) {
        for _ in 0..10 {
            roles.shuffle(&mut self.rng);
            let Some(role) = roles.first_mut() else {
                return;
            };
            match role {
                Role::TOWN => *role = Role::MILLER,
                Role::MAFIA => *role = Role::GODFATHER,
                _ => continue,
            }
            break;
        }
    }

    fn draw_mislead(&mut self, n: usize, roles: &mut Vec<Role>) {
        let mut x: f64;
        for _ in 0..n / 3 {
            x = self.rng.sample(Open01);
            if x < self.rules.mislead as f64 / 100.0 {
                self.add_mislead(roles);
            } else {
                break;
            }
        }
    }

    fn assign_charge(&mut self, role: &mut Role, scratch: &Vec<(Pid, Role)>) {
        for _ in 0..2 {
            let team = match role {
                Role::GUARD(CHARGE_TOWN) | Role::AGENT(CHARGE_TOWN) => Some(Team::Town),
                Role::GUARD(CHARGE_MAFIA) | Role::AGENT(CHARGE_MAFIA) => Some(Team::Mafia),
                Role::GUARD(CHARGE_ANY) | Role::AGENT(CHARGE_ANY) => None,
                _ => panic!("Not a charge role"),
            };
            let f = |(pid, role): &(Pid, Role)| {
                team.as_ref().map(|t| (role.team() == *t).then(|| *pid)).flatten()
            };
            let pids = scratch.iter().filter_map(|x| f(x)).collect::<Vec<_>>();
            if pids.is_empty() {
                match role {
                    Role::GUARD(_) => {
                        *role = Role::GUARD(CHARGE_ANY);
                    }
                    Role::AGENT(_) => {
                        *role = Role::AGENT(CHARGE_ANY);
                    }
                    _ => panic!("Not a charge role"),
                }
                continue;
            }
            match role {
                Role::GUARD(_) => {
                    let pid = pids.choose(&mut self.rng).unwrap();
                    *role = Role::GUARD(*pid);
                }
                Role::AGENT(_) => {
                    let pid = pids.choose(&mut self.rng).unwrap();
                    *role = Role::AGENT(*pid);
                }
                _ => panic!("Not a charge role"),
            }
            break;
        }
    }

    fn assign_roles(&mut self, users: Vec<impl Into<Pid>>, roles: Vec<Role>) -> Vec<(Pid, Role)> {
        let mut users = users.into_iter().map(Into::into).collect::<Vec<_>>();
        users.shuffle(&mut self.rng);
        let mut registry: Vec<(Pid, Role)> = users.into_iter().zip(roles.into_iter()).collect();
        let scratch = registry.clone();
        for (pid, role) in registry.iter_mut() {
            match role {
                Role::GUARD(_) | Role::AGENT(_) => self.assign_charge(role, &scratch),
                _ => {}
            }
        }
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rolegen::RoleGen;

    #[test]
    #[tracing_test::traced_test]
    fn test_draw() {
        let mut rng = thread_rng();
        let mut rules = Rules::default();
        rules.guaranteed_roles.drain();
        let mut rg = DrawRoleGen { rules: Rules::default(), rng: rng.clone() };
        let users = (0..=7).map(Pid).collect::<Vec<_>>();
        for _ in 0..10 {
            let mut roles = rg
                .generate_roles(users.clone(), &Rules::default())
                .into_iter()
                .map(|(_, r)| r)
                .collect::<Vec<_>>();
            roles.sort();
            println!("{:?}", roles);
        }
    }
}
