known_roles = "known_roles"
reveal_on_death = "reveal_on_death"
know_if_saved = "know_if_saved"
know_if_saved_doc = "know_if_saved_doc"
know_if_saved_self = "know_if_saved_self"
start_night = "start_night"
charge_refocus_guard = "charge_refocus_guard"
charge_refocus_agent = "charge_refocus_agent"
idiot_vengeance = "idiot_vengeance"
know_if_stripped = "know_if_stripped"
no_milk_self = "no_milk_self"
cop_strength = "cop_strength"


TOWN_ROLES = [
  'TOWN',
  'COP',
  'DOCTOR',
  'CELEB',
  'MILLER',
  'MILKY',
  'MASON',
]

MAFIA_ROLES = [
  'MAFIA',
  'GODFATHER',
  'STRIPPER',
  'GOON',
]

ROGUE_ROLES = [
  'IDIOT',
  'SURVIVOR',
  'GUARD',
  'AGENT',
]

ALL_ROLES = TOWN_ROLES + MAFIA_ROLES + ROGUE_ROLES

TARGETING_ROLES = {
  'COP',
  'DOCTOR',
  'MILKY',
  'STRIPPER',
}

CONTRACT_ROLES = {
  'IDIOT',
  'SURVIVOR',
  'GUARD',
  'AGENT',
}