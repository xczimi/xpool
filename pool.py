from google.appengine.api import memcache
from uefa2012 import Uefa2012
from model import GroupGame

def singlegame_result_point(bet, result):
    point = 0
    if not result.locked: return 0
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
    if not result.locked: return 0
    def count_orders(xs,ys):
        x_order_set = set((x,y) for x in xs for y in xs  if xs.index(x) < xs.index(y))
        y_order_set = set((x,y) for x in ys for y in ys  if ys.index(x) < ys.index(y))
        return len(x_order_set.intersection(y_order_set))
    return count_orders([tgr.team.short for tgr in bet.get_ranks()],[tgr.team.short for tgr in result.get_ranks()])

def group_multiplier(game):
    multiplier = 1
    kolevels = Uefa2012().kostage.subgames()
    kolevels.sort(key=GroupGame.groupstart)
    for kolevel in kolevels:
        multiplier = multiplier + 1
        if str(game.key()) == str(kolevel.key()):
            return multiplier
    return 1

def score_group(user, resultuser, game):
    point = 0
    for groupgame in game.groupgames():
        point = point + score_group(user, resultuser, groupgame)
    for singlegame in game.singlegames():
        point = point + singlegame_result_point(user.singlegame_result(singlegame), resultuser.singlegame_result(singlegame))
    point = point + groupgame_result_point(user.groupgame_result(game), resultuser.groupgame_result(game))
    return group_multiplier(game) * point

def perfects_single(users, resultuser, game, perfect = 4):
    perfects = []
    for user in users:
        if user.key() != resultuser.key():
            bet = user.singlegame_result(game)
            if perfect == singlegame_result_point(bet, resultuser.singlegame_result(game)):
                perfects.append({'user':user,'game':game,'bet':bet})
    return perfects

def perfects_group(users, resultuser, game, perfect = 4):
    cache_key = 'perfect/' + str(game.key())
    perfects = memcache.get(cache_key)
    if perfects is None:
        perfects = []
        for group in game.groupgames():
            perfects.extend(perfects_group(users, resultuser, group, perfect))
        for single in game.singlegames():
            perfects.extend(perfects_single(users, resultuser, single, perfect))
        memcache.add(cache_key, perfects)
    return perfects

def scoreboard(users, resultuser, game):
    cache_key = 'scoreboard/' + str(game.key())
    scoreboard = memcache.get(cache_key)
    if scoreboard is None:
        scoreboard = [{'user':user,'score':score_group(user,resultuser,game)} for user in users if str(user.key()) != str(resultuser.key())]
        scoreboard.sort(reverse=True,key=lambda x: x['score'])
        memcache.add(cache_key, scoreboard)
    return scoreboard

def flush_singlegame(singlegame):
    memcache.delete('perfect/'+str(singlegame.key()))
    memcache.delete('scoreboard/'+str(singlegame.key()))
    flush_groupgame(singlegame.group())

def flush_groupgame(groupgame):
    memcache.delete('perfect/'+str(groupgame.key()))
    memcache.delete('scoreboard/'+str(groupgame.key()))
    if not groupgame.upgroup() is None:
        flush_groupgame(groupgame.upgroup())
