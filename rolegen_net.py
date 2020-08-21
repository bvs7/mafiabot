""" Input neurons:
n_players
n_town
n_maf
n_rogue
n_TOWN
n_COP
n_DOCTOR
n_CELEB
n_MILKY
n_MILLER
n_MAFIA
n_STRIPPER
n_GODFATHER
n_GOON
n_IDIOT
n_SURVIVOR
n_GUARD_Town
n_GUARD_Mafia
n_GUARD_Rogue
n_AGENT_Town
n_AGENT_Mafia
n_AGENT_Rogue

(22 inputs)

assume rules stay the same

The net takes list of inputs and gives a probability of town winning.
The input data is a list of inputs and probabilities, where the number of
games in that category decides the certainty of the results. (e.g. no games in category give 50%? 1 game gives 40%-60%? etc?)

Or, assume every category has at least n games, add tied games if not enough data.

"""

