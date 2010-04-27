def singlegame_result_point(bet, result):
    point = 0
    if bet.homeScore >=0 and bet.homeScore == result.homeScore: point = point + 1
    elif bet.homeScore > 4 and result.homeScore > 4: point = point + 1
    if bet.awayScore >=0 and bet.awayScore == result.awayScore: point = point + 1
    elif bet.homeScore > 4 and result.homeScore > 4: point = point + 1
    if bet.home_w() == result.home_w(): point = point + 1
    if bet.home_l() == result.home_l(): point = point + 1
    if bet.home_d() == result.home_d(): point = point + 2
    return point




