def singlegame_result_point(bet, result):
    point = 0
    if not result.locked or not bet.locked: return 0
    if bet.homeScore >=0 and bet.homeScore == result.homeScore: point = point + 1
    elif bet.homeScore > 4 and result.homeScore > 4: point = point + 1
    if bet.awayScore >=0 and bet.awayScore == result.awayScore: point = point + 1
    elif bet.homeScore > 4 and result.homeScore > 4: point = point + 1
    if bet.home_w() == 1 and result.home_w() == 1: point = point + 2
    if bet.home_l() == 1 and result.home_l() == 1: point = point + 2
    if bet.home_d() == 1 and result.home_d() == 1: point = point + 2
    return point

def groupgame_result_point(bet, result):
    point = 0
    if not result.locked or not bet.locked: return 0
    def count_orders(xs,ys):
        x_order_set = set((x,y) for x in xs for y in xs  if xs.index(x) < xs.index(y))
        y_order_set = set((x,y) for x in ys for y in ys  if ys.index(x) < ys.index(y))
        return len(x_order_set.intersection(y_order_set))
    return count_orders([tgr.team.short for tgr in bet.get_ranks()],[tgr.team.short for tgr in result.get_ranks()])

def score_group(user, result, game):
    point = 0
    for groupgame in game.groupgames():
        point = point + score_group(user, result, groupgame)
    for singlegame in game.singlegames():
        point = point + singlegame_result_point(user.singlegame_result(singlegame), result.singlegame_result(singlegame))
    point = point + groupgame_result_point(user.groupgame_result(game), result.groupgame_result(game))
    return point