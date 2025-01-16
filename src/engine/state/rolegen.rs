// Sigmoid fn to gen number of mafia

use rand::{self, distributions::Open01, rngs::ThreadRng, thread_rng, Rng};
use tracing::debug;

const min_p: f64 = 0.2;
const max_p: f64 = 0.5;

const target_p: f64 = 0.35;

fn geo_p() -> f64 {
    (min_p * max_p).sqrt()
}

fn p_from_v(n_: usize, v: f64) -> f64 {
    let n = n_ as f64;
    return 2.0 * v / (n + 1.0);
}

fn P_from_v(n_: usize, v: f64) -> f64 {
    let p = p_from_v(n_, v);
    return (p - min_p) / (max_p - min_p);
}

fn p(n_: usize, m_: usize) -> f64 {
    let n = n_ as f64;
    let m = m_ as f64;
    return 2.0 * m / (n + 1.0);
}

// Assume valid p range is 0.2 to 0.5
fn P(n_: usize, m_: usize) -> f64 {
    let p = p(n_, m_);
    return (p - min_p) / (max_p - min_p);
}

fn get_n_maf(n_: usize, mut rng: &mut ThreadRng) -> usize {
    let v = get_v(n_, rng);
    let mut m = v.round() as usize;
    if m == 0 {
        m = 1;
    }
    for _ in 0..5 {
        if p(n_, m) > max_p {
            m -= 1;
        }
    }
    for _ in 0..5 {
        if p(n_, m) < min_p {
            m += 1;
        }
    }
    return m;
}

fn get_v(n_: usize, mut rng: &mut ThreadRng) -> f64 {
    let n = n_ as f64;
    let x_0 = target_p / 2.0 * (n + 1.0);
    let k = 10.0 / x_0;
    let x: f64 = rng.sample(Open01);
    let v = -f64::ln(1.0 / x - 1.0) / k + x_0;
    return v;
}

/*
First, get v, which is the floating point number of mafia.
Get P value from n and v (0% to 100%). This is the "difficulty" for town of the game

## Generating Rogue players
Generate a number of Rogue players using an exponential equation based on number of players.
- We want a rate of about 1 rogue every, say, 3 games of 7. So that would mean 1/21 players is Rogue
- So in a game of 21 players, there is a good chance to have at least one rogue.
- Use a Poisson Distribution with the rate being... N/20.
- Given a number of Rogue players, do the following
If P > 50% pick one of...
- Turn a mafia into an IDIOT (75%) (v := v-1.0?)
- Create a LOVER that protects TOWN (20%)
- Create an AGENT that targets MAFIA (5%)
Else if P < 50%, pick one of...
- Turn a town into an IDIOT (70%)
- Turn a town into a SURVIVOR (5%)
- Create a LOVER that protects MAFIA (5%)
- Create an AGENT that targets TOWN (20%)

Maybe an IDIOT is more of a .25 maf than .5. Or maybe we will say 0.33
This keeps the odds of a Rogue targeting a mafia at around 1/5 (the approx number of mafia)
How does the number of Rogue players effect the number of mafia?
Say we have a v of 1.45 and a P of ~55% (7 players). We decide to add one rogue, that will replace a mafia
Now the calc v is 0.95 or a P of ~12%. But given that we can't have a game with no mafia... We would end up with
m = 1, i = 0.33, so 1.33 v which is P of 45%... Not bad really. We have to be careful we don't get into a situaton
where we can't get a valid game

I think we can do something similar for Power roles.
Use a Poisson distribution to pick a number of power roles. We can make the ratio a settings option.
For now, let's assume 0.3. So in a game of, 11 players, we expect between 2 and 5, even up to 6.
Maybe we just role rogue into this? Nah. Let's just say the n_rogue subtracts from this
Generate the number, then subtract any guaranteed roles (COP and DOC) and subtract any rogues

Now, do things
Things that benefit mafia:
- Change a town to Mafia?
- Upgrade a mafia to a power role (STRIPPER, GODFATHER)
- Change a Town to a MILLER
Things that benefit town:
- Upgrade a Town to CELEB
- Change a mafia to a GOON
- Upgrade a Town to COP or DOCTOR


List of things to try doing, in order of helping mafia most to helping town most
- Change a town to mafia
- Upgrade a mafia to STRIPPER
- Upgrade a mafia to GODFATHER
- Change a town to MILLER
Neutral
- Upgrade a town to CELEB
- Upgrade two town to MASONs
- Upgrade a town to COP
- Upgrade a town to DOCTOR


So let's say that K is the number of kinky roles to pick... depending on where P is at,
let's pick some k_1, k_2, ... k_k such that they come out to about 0.5 - P.
For example, if P is 0.75, we want our total k to be about 0.25. Then each of those k's gets fuzzed
by a bit and slotted into a possible outcome seen above, based on their weights.

Let's choose our lambda rate of kinkiness to be the setting, minus half the guaranteed roles. Then,
Whatever Poisson value we get, subtract the other half of the guaranteed roles. This means guaranteed
roles allow less variance later on.

K is the kinkiness of the game and is a value from 0 to 1
V is the variability of the game and is a value from 0.00001 to ~100 or so. 1 is standard
r is the number of roles already assigned

Lambda = K * V * N
Offset = K * (V-1) * N

The number of kinky roles is:
let lambda = K * V * N;
let offset = K * (V-1) * N;

TODO: This is too difficult as it gets bigger bc factorial. Figure out something better.

Anyways, we get a number of kinky, then we get the change values, then we apply them.


Experimental values:
Using mean p (min_p + max_p) / 2.0
3-1: 100.00%, p = 0.5000,(P = 1.0000)
E[p] = 0.5000
4-1: 100.00%, p = 0.4000,(P = 0.6667)
E[p] = 0.4000
5-1: 100.00%, p = 0.3333,(P = 0.4444)
E[p] = 0.3333
6-1: 100.00%, p = 0.2857,(P = 0.2857)
E[p] = 0.2857
7-1: 67.13%, p = 0.2500,(P = 0.1667)
7-2: 32.87%, p = 0.5000,(P = 1.0000)
E[p] = 0.3322
8-1: 38.58%, p = 0.2222,(P = 0.0741)
8-2: 61.42%, p = 0.4444,(P = 0.8148)
E[p] = 0.3587
9-1: 19.42%, p = 0.2000,(P = 0.0000)
9-2: 80.58%, p = 0.4000,(P = 0.6667)
E[p] = 0.3612
10-2: 100.00%, p = 0.3636,(P = 0.5455)
E[p] = 0.3636
11-2: 86.85%, p = 0.3333,(P = 0.4444)
11-3: 13.15%, p = 0.5000,(P = 1.0000)
E[p] = 0.3552
12-2: 72.95%, p = 0.3077,(P = 0.3590)
12-3: 27.05%, p = 0.4615,(P = 0.8718)
E[p] = 0.3493
13-2: 54.27%, p = 0.2857,(P = 0.2857)
13-3: 45.73%, p = 0.4286,(P = 0.7619)
E[p] = 0.3510
14-2: 38.74%, p = 0.2667,(P = 0.2222)
14-3: 61.26%, p = 0.4000,(P = 0.6667)
E[p] = 0.3483
15-2: 26.14%, p = 0.2500,(P = 0.1667)
15-3: 66.49%, p = 0.3750,(P = 0.5833)
15-4: 7.37%, p = 0.5000,(P = 1.0000)
E[p] = 0.3515
16-2: 17.12%, p = 0.2353,(P = 0.1176)
16-3: 68.75%, p = 0.3529,(P = 0.5098)
16-4: 14.13%, p = 0.4706,(P = 0.9020)
E[p] = 0.3494
17-2: 10.89%, p = 0.2222,(P = 0.0741)
17-3: 64.57%, p = 0.3333,(P = 0.4444)
17-4: 24.54%, p = 0.4444,(P = 0.8148)
E[p] = 0.3485
18-2: 7.59%, p = 0.2105,(P = 0.0351)
18-3: 55.88%, p = 0.3158,(P = 0.3860)
18-4: 36.53%, p = 0.4211,(P = 0.7368)
E[p] = 0.3463
19-2: 5.28%, p = 0.2000,(P = 0.0000)
19-3: 44.76%, p = 0.3000,(P = 0.3333)
19-4: 44.40%, p = 0.4000,(P = 0.6667)
19-5: 5.56%, p = 0.5000,(P = 1.0000)
E[p] = 0.3502
20-3: 38.25%, p = 0.2857,(P = 0.2857)
20-4: 52.30%, p = 0.3810,(P = 0.6032)
20-5: 9.45%, p = 0.4762,(P = 0.9206)
E[p] = 0.3535
21-3: 29.09%, p = 0.2727,(P = 0.2424)
21-4: 54.97%, p = 0.3636,(P = 0.5455)
21-5: 15.94%, p = 0.4545,(P = 0.8485)
E[p] = 0.3517

*/

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use rand::thread_rng;
    use tracing_test::traced_test;

    use super::*;

    #[test]
    fn try_10000() {
        let tot = 10000;
        let mut rng = thread_rng();

        for n in 3..22 {
            let mut map = HashMap::new();

            // let mut m = [0; 1000];
            for i in 0..tot {
                let m = get_n_maf(n, &mut rng);
                let hist: &mut u32 = map.entry(m).or_default();
                *hist += 1;
            }

            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            let mut expected_p = 0.0;
            for key in keys {
                let m = *key;
                let p = p(n, m);
                let P = P(n, m);
                let e = map[key] as f64 / tot as f64;
                println!(
                    "{}-{}: {:.2}%, p = {:.4},(P = {:.4})",
                    n,
                    key,
                    e * 100.0,
                    p,
                    P
                );
                expected_p += p * e
            }
            println!("E[p] = {:.4}", expected_p);
        }
    }

    #[test]
    fn try_1() {
        let n = 11;
        let mut rng = thread_rng();

        for _ in 0..10 {
            let v = get_v(n, &mut rng);
            let P = P_from_v(n, v);
            println!("{}-{:.3}, {:.3}", n, v, P);
        }
    }
}
