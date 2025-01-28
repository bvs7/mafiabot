use crate::prelude::*;

use rand::{
    distributions::{Distribution, Open01, Standard},
    rngs::ThreadRng,
    seq::SliceRandom,
    Rng, RngCore,
};
use statrs::distribution::Poisson;

use super::RoleGen;

const MIN_P: f64 = 0.2;
const MAX_P: f64 = 0.5;

const TARGET_P: f64 = 0.35;

const ROGUES: [RoleKind; 4] =
    [RoleKind::IDIOT, RoleKind::SURVIVOR, RoleKind::GUARD, RoleKind::AGENT];
const POWERS: [RoleKind; 5] =
    [RoleKind::COP, RoleKind::DOCTOR, RoleKind::CELEB, RoleKind::MILKY, RoleKind::STRIPPER];
const MISLEAD: [RoleKind; 2] = [RoleKind::MILLER, RoleKind::GODFATHER];

#[derive(Debug, Clone, Copy)]
enum Role_ {
    Role(RoleKind),
    Charged(RoleKind, Team),
}

impl Role_ {
    fn from_kind(kind: RoleKind) -> Self {
        Role_::Role(kind)
    }
    fn from_charge(kind: RoleKind, team: Team) -> Self {
        assert!(kind == RoleKind::GUARD || kind != RoleKind::AGENT);
        Role_::Charged(kind, team)
    }
    fn kind(&self) -> RoleKind {
        match self {
            Role_::Role(kind) => *kind,
            Role_::Charged(kind, _) => *kind,
        }
    }

    // Difference in power from a common TOWN? which would be -0.5?
    // Positive is pro-mafia, negative is pro-town
    fn sigma_adj(&self, n: usize) -> f64 {
        match self {
            Role_::Role(RoleKind::TOWN) => 0.0,
            Role_::Role(RoleKind::IDIOT) => 0.75,
            Role_::Role(RoleKind::SURVIVOR) => 0.25,
            Role_::Charged(RoleKind::AGENT, Team::Town) => 0.25,
            Role_::Charged(RoleKind::AGENT, Team::Mafia) => -0.75,
            Role_::Charged(RoleKind::GUARD, Team::Town) => -0.25,
            Role_::Charged(RoleKind::GUARD, Team::Mafia) => 0.75,
            Role_::Role(RoleKind::COP) => -0.5,
            Role_::Role(RoleKind::DOCTOR) => -0.5,
            Role_::Role(RoleKind::CELEB) => -0.5,
            Role_::Role(RoleKind::MILKY) => -0.25,
            Role_::Role(RoleKind::STRIPPER) => 1.5,
            Role_::Role(RoleKind::MAFIA) => 1.0, // This means an extra mafia!
            _ => {
                warn!("No sigma adjustment for observed {:?}", self);
                0.0
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Adjustment {
    ReplaceTOWN(Role_),
    ReplaceMAFIA(Role_),
    AddMafia,
}

fn p(n_: usize, m_: usize) -> f64 {
    let m = m_ as f64;
    return expected_p(n_, m);
}

fn expected_p(n_: usize, sigma: f64) -> f64 {
    let n = n_ as f64;
    return 2.0 * sigma / (n + 1.0);
}

fn get_sigma_maf(n_: usize, rng: &mut impl Rng) -> f64 {
    let x_0 = TARGET_P / 2.0 * (n_ as f64 + 1.0);
    let sigma = sigmoid_rv(x_0, rng);
    return sigma;
}
fn sigmoid_rv(x_0: f64, rng: &mut impl Rng) -> f64 {
    let x: f64 = rng.sample(Open01);
    let s = (-f64::ln(1.0 / x - 1.0) / 10.0 + 1.0) * x_0;
    return s;
}

fn get_n_rogue(n: usize, rng: &mut impl Rng) -> isize {
    let rate = n as f64 / 25.0;
    let poisson = Poisson::new(rate).unwrap();
    let n_rogue: u64 = poisson.sample(rng);
    return n_rogue as isize;
}

fn get_n_power(n: usize, rng: &mut impl Rng) -> isize {
    // TODO: add K and V inputs
    let rate = n as f64 / 10.0 * 3.0;
    let poisson = Poisson::new(rate).unwrap();
    let n_rogue: u64 = poisson.sample(rng);
    return n_rogue as isize;
}

// TODO: construct these based on allowed_roles
fn get_rogue_adjust(e_p: f64, rng: &mut impl Rng) -> Adjustment {
    let choices = if e_p > TARGET_P {
        vec![
            (Adjustment::ReplaceMAFIA(Role_::Role(RoleKind::IDIOT)), 0.75),
            (Adjustment::ReplaceTOWN(Role_::Charged(RoleKind::GUARD, Team::Town)), 0.20),
            (Adjustment::ReplaceTOWN(Role_::Charged(RoleKind::AGENT, Team::Mafia)), 0.05),
        ]
    } else {
        vec![
            (Adjustment::ReplaceTOWN(Role_::Role(RoleKind::IDIOT)), 0.70),
            (Adjustment::ReplaceTOWN(Role_::Role(RoleKind::SURVIVOR)), 0.05),
            (Adjustment::ReplaceTOWN(Role_::Charged(RoleKind::GUARD, Team::Town)), 0.05),
            (Adjustment::ReplaceTOWN(Role_::Charged(RoleKind::AGENT, Team::Town)), 0.20),
        ]
    };
    return choices.choose_weighted(rng, |x| x.1).unwrap().0;
}
fn get_power_adjust(e_p: f64, rng: &mut impl Rng) -> Adjustment {
    let choices = if e_p > TARGET_P {
        vec![
            (Adjustment::ReplaceTOWN(Role_::Role(RoleKind::DOCTOR)), 0.5),
            (Adjustment::ReplaceTOWN(Role_::Role(RoleKind::COP)), 0.5),
            (Adjustment::ReplaceTOWN(Role_::Role(RoleKind::CELEB)), 0.25),
            (Adjustment::ReplaceTOWN(Role_::Role(RoleKind::MILKY)), 0.10),
        ]
    } else {
        vec![
            (Adjustment::ReplaceTOWN(Role_::Role(RoleKind::MAFIA)), 1.0),
            (Adjustment::ReplaceMAFIA(Role_::Role(RoleKind::STRIPPER)), 0.5),
        ]
    };
    return choices.choose_weighted(rng, |x| x.1).unwrap().0;
}

pub struct StandardRoleGen<R> {
    n: usize,
    rules: Rules,
    sigma: f64,
    roles: Vec<Role_>,
    rng: R,
}

impl<R> std::fmt::Debug for StandardRoleGen<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StandardRoleGen")
            .field("n", &self.n)
            .field("sigma", &self.sigma)
            .field("roles", &self.roles)
            .finish()
    }
}

impl<R: Rng> StandardRoleGen<R> {
    fn new(n: usize, rules: Rules, mut rng: R) -> Self {
        let sigma = get_sigma_maf(n, &mut rng);
        let roles = vec![Role_::from_kind(RoleKind::TOWN); n];
        StandardRoleGen { n, rules, sigma, roles, rng }
    }

    fn add_rogue(&mut self) {
        let mut n_rogue = get_n_rogue(self.n, &mut self.rng);
        // First check for guaranteed roles
        for role in ROGUES.iter() {
            if let Some(n_role) = self.rules.guaranteed_roles.get(role) {
                for _ in 0..*n_role {
                    self.add_role(Role_::from_kind(*role));
                    self.pop_role();
                    n_rogue -= 1;
                }
            }
        }
        for _ in 0..n_rogue {
            let e_p = self.get_e_p();
            let adjust = get_rogue_adjust(e_p, &mut self.rng);
            self.apply(adjust);
        }
    }

    fn add_power(&mut self) {
        let mut n_power = get_n_power(self.n, &mut self.rng);
        for role in POWERS.iter() {
            if let Some(n_role) = self.rules.guaranteed_roles.get(role) {
                for _ in 0..*n_role {
                    self.add_role(Role_::from_kind(*role));
                    self.pop_role();
                    n_power -= 1;
                }
            }
        }
        for _ in 0..n_power {
            let e_p = self.get_e_p();
            let adjust = get_power_adjust(e_p, &mut self.rng);
            self.apply(adjust);
        }
    }

    fn add_mislead(&mut self) {
        let x: f64 = self.rng.sample(Open01);
        if x < (self.rules.mislead as f64 / 100.0) {
            let mut n_mislead: u64 =
                self.rng.sample::<u64, _>(Poisson::new(self.n as f64 / 10.0).unwrap());
            if n_mislead == 0 {
                n_mislead = 1;
            }
            for role in MISLEAD.iter() {
                for _ in 0..n_mislead {
                    self.add_role(Role_::from_kind(*role));
                    self.pop_role();
                    n_mislead -= 1;
                }
            }
            // Check number of vanilla town, vs number of vanilla mafia?
            let n_town = self.roles.iter().filter(|r| r.kind() == RoleKind::TOWN).count() as f64;
            let choices = [(RoleKind::MILLER, n_town), (RoleKind::GODFATHER, self.sigma)];
            let (role, _) = choices.choose_weighted(&mut self.rng, |x| x.1).unwrap();
            self.add_role(Role_::from_kind(*role));
            self.pop_role();
        }
    }

    fn finish(&mut self) {
        let n_mafia = self.sigma.round() as usize;
        for _ in 0..n_mafia {
            self.add_role(Role_::from_kind(RoleKind::MAFIA));
            self.sigma -= 2.0;
            self.pop_role();
        }
        debug!("After finishing: {:?}", self);
    }

    fn get_adj_sigma(&self) -> f64 {
        let mut sigma = self.sigma;
        for role in self.roles.iter() {
            sigma += role.sigma_adj(self.n);
        }
        return sigma;
    }

    fn get_e_p(&self) -> f64 {
        let sigma = self.get_adj_sigma();
        return expected_p(self.n, sigma);
    }

    fn apply(&mut self, adjust: Adjustment) {
        // debug!("Applying adjustment: {:?} to {:?}", adjust, self);
        match adjust {
            Adjustment::ReplaceTOWN(role) => {
                self.add_role(role);
                self.pop_role();
            }
            Adjustment::ReplaceMAFIA(role) => {
                self.add_role(role);
                self.sigma -= 1.0;
                self.pop_role();
            }
            Adjustment::AddMafia => {
                self.sigma += 1.0;
            }
        }
        // debug!("After applying adjustment: {:?}", self);
    }

    fn add_role(&mut self, role: Role_) {
        self.roles.insert(0, role);
        self.sigma += role.sigma_adj(self.n);
    }

    fn pop_role(&mut self) -> Role_ {
        let role = self.roles.pop().unwrap();
        self.sigma -= role.sigma_adj(self.n);
        return role;
    }
}

impl<R: Rng> RoleGen for StandardRoleGen<R> {
    type RNG = R;
    fn generate_roles(n: usize, rules: &Rules, rng: Self::RNG) -> Vec<crate::Role> {
        let mut gen = StandardRoleGen::new(n, rules.clone(), rng);
        info!("After choosing Sigma: {:?}", gen);
        gen.add_rogue();
        info!("After choosing Rogue: {:?}", gen);
        gen.add_power();
        info!("After choosing Power: {:?}", gen);
        gen.add_mislead();
        info!("After choosing Mislead: {:?}", gen);
        gen.finish();
        let roles = gen.roles.into_iter().map(|r| crate::Role::from(r.kind())).collect();
        return roles;
    }
}

#[cfg(test)]
mod tests {
    use rand::prelude::Distribution;

    use statrs::function::gamma::gamma;

    use super::*;

    fn poisson_pdf(x: f64, lambda: f64) -> f64 {
        return lambda.powf(x) * (-lambda).exp() / gamma(x + 1.0) as f64;
    }

    #[test]
    fn poisson() {
        let n = 11.0;
        let lambda = n / 25.0;
        println!("Lambda = {}", lambda);
        let max = 20;
        let end = 3;
        for i in 0..max {
            let x = i as f64 / max as f64 * end as f64;
            let p = poisson_pdf(x, lambda);
            println!("Poisson({:.2}) = {:.3}", x, p);
        }
    }

    #[test]
    #[tracing_test::traced_test]
    fn try_std_role_gen() {
        let rules = Rules::default();
        let mut rng = rand::thread_rng();
        let n = 11;
        let roles = StandardRoleGen::generate_roles(n, &rules, &mut rng);
        println!("{:#?}", roles);
    }
}
