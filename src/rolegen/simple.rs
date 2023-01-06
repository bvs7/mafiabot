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
