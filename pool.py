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

def groupgame_result_point(bet, result):
    point = 0
    def count_orders(xs,ys):
        x_order_set = set((x,y) for x in xs for y in xs  if xs.index(x) < xs.index(y))
        y_order_set = set((x,y) for x in ys for y in ys  if ys.index(x) < ys.index(y))
        return len(x_order_set.intersection(y_order_set))
    return count_orders([tgr.team.short for tgr in bet.get_ranks()],[tgr.team.short for tgr in result.get_ranks()])
