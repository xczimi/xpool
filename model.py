#!/usr/bin/env python
# -*- coding: UTF-8 -*-
#
# Copyright 2010 Peter Czimmermann  <xczimi@gmail.com>
#
import Cookie
import uuid
import re
from google.appengine.api import users
from google.appengine.api import memcache
from google.appengine.api import mail
from google.appengine.ext import webapp
from google.appengine.ext.webapp import util
from google.appengine.ext.webapp import template
from google.appengine.api import memcache

from django.template import TemplateDoesNotExist

from google.appengine.ext import db
from google.appengine.ext.db import polymodel

from datetime import datetime

MAX_ITEMS = 256

def cached(func):
    def cached_func(self, nocache=False):
        cache_prop = '_'+func.__name__
        if nocache or cache_prop not in self.__dict__:
            self.__dict__[cache_prop] = func(self, nocache=nocache)
        return self.__dict__[cache_prop]
    return cached_func

def perm_cached(func):
    obj_perm_cache = {}
    def perm_cached_func(self, nocache=False):
        cache_key = self.__class__.__name__ + "/" + func.__name__ + "/" + str(self.key())
        if nocache:
            memcache.delete(cache_key)
            obj_perm_cache[cache_key] = None
            return None
        if cache_key not in obj_perm_cache:
            data = memcache.get(cache_key)
            if data is None:
                data = func(self, nocache=nocache)
                memcache.add(cache_key, data)
            obj_perm_cache[cache_key] = data
        return obj_perm_cache[cache_key]
    return perm_cached_func

def perm_cached_class(func, flush=False):
    class_perm_cache = {}
    if flush:
        class_perm_cache = {}
        memcache.flush_all()
    def cached_func(self):
        cache_key = self.__name__ + "/" + func.__name__
        if cache_key in class_perm_cache:
            return class_perm_cache[cache_key]
        data = memcache.get(cache_key)
        if data is None:
            data = func(self)
            memcache.add(cache_key, data)
        class_perm_cache[cache_key] = data
        return class_perm_cache[cache_key]
    return cached_func

class LocalUser(db.Model):
    email = db.EmailProperty(required=True)
    nick = db.StringProperty()
    full_name = db.StringProperty()
    google_user = db.UserProperty()
    password = db.StringProperty()
    authcode = db.StringProperty()
    referrer = db.SelfReferenceProperty(collection_name="referral_set")
    active = db.BooleanProperty()

    def __str__(self):
        if self.nick: return self.nick.encode('utf-8')
        return self.email.encode('utf-8')

    @perm_cached
    def singleresults(self, nocache=False):
        """ Returns a dict of the users Result objects keyed by the singlegame (cached). """
        results = {}
        for result in self.singleresult_set.fetch(MAX_ITEMS):
            results[str(result.singlegame.key())] = result
        return results

    @perm_cached
    def groupresults(self, nocache=False):
        """ Returns a dict of the users Result objects keyed by the singlegame (cached). """
        results = {}
        for result in self.groupresult_set.fetch(MAX_ITEMS):
            results[str(result.groupgame.key())] = result
        return results

    def singlegame_result(self, singlegame):
        if not str(singlegame.key()) in self.singleresults():
            return Result(user=self,singlegame=singlegame)
        return self.singleresults()[str(singlegame.key())]

    def groupgame_result(self, groupgame):
        if not str(groupgame.key()) in self.groupresults():
            return GroupResult(user=self,groupgame=groupgame)
        return self.groupresults()[str(groupgame.key())]

    @classmethod
    def actives(self):
        actives = memcache.get('active_users')
        if actives is None:
            actives = self.all().filter('active =',True).fetch(MAX_ITEMS)
            memcache.add('active_users',actives,300)
        return actives

class LocalUserGroup(db.Model):
    name = db.StringProperty(required=True)
    root = db.ReferenceProperty(LocalUser,required=True)
    password = db.StringProperty()

class FacebookUser(db.Model):
    id = db.StringProperty(required=True)
    created = db.DateTimeProperty(auto_now_add=True)
    updated = db.DateTimeProperty(auto_now=True)
    name = db.StringProperty(required=True)
    profile_url = db.StringProperty(required=True)
    access_token = db.StringProperty(required=True)
    localuser = db.ReferenceProperty(LocalUser)
    email = db.EmailProperty(required=True)

class Team(db.Model):
    name = db.StringProperty(required=True)
    flag = db.StringProperty()
    short = db.StringProperty()
    href = db.StringProperty()
    def __hash__(self):
        id_match = re.match(r'^/worldcup/teams/team=([0-9]+)/index.html$', self.href)
        return int(id_match.group(1))

    def __eq__(self, other):
        return self.__hash__() == other.__hash__()

    @classmethod
    @perm_cached_class
    def everything(self):
        teams = {}
        for team in self.all().fetch(MAX_ITEMS):
            teams[str(team.key())] = team
        return teams

    @classmethod
    def byKey(self, key):
        try:
            return self.everything()[str(key)]
        except KeyError:
            return None

    @classmethod
    def nothing(self):
        memcache.delete(self.__name__ + "/everything")
        return []

class TeamGroupRank:
    team = None
    tie = None
    draw = []
    def __init__(self, team, w=0, d=0, l=0, gf=0, ga=0):
        self.team, self.w, self.d, self.l, self.gf, self.ga = team, w, d, l, gf, ga
        self.pt = 3 * w + d

    def __add__(self,other):
        return TeamGroupRank(self.team,
            self.w+other.w, self.d+other.d, self.l+other.l,
            self.gf+other.gf, self.ga+other.ga)

    def __hash__(self):
        """ Helps to create a set to find ties. """
        return self.gf + 1000 * (
                (self.gf - self.ga + 500) + 1000 * (
                    self.pt ))

    def __cmp__(self,other):
        if self.pt < other.pt: return -1
        if self.pt > other.pt: return 1
        if self.gf - self.ga < other.gf - other.ga: return -1
        if self.gf - self.ga > other.gf - other.ga: return 1
        if self.gf < other.gf: return -1
        if self.gf > other.gf: return 1
        return 0

class GroupGame(db.Model):
    name = db.StringProperty(required=True)
    upgroup_ref = db.SelfReferenceProperty(collection_name="game_set")

    def upgroup_key(self):
        return GroupGame.upgroup_ref.get_value_for_datastore(self)

    def upgroup(self):
        try:
            return self.everything()[str(self.upgroup_key())]
        except KeyError:
            return None

    def level(self):
        if self.upgroup() is None: return 1
        return self.upgroup().level() + 1

    @cached
    def singlegames(self, nocache=False):
        singlegames = [singlegame for singlegame in SingleGame.everything().itervalues() if self.key() == singlegame.group_key()]
        singlegames.sort(cmp=lambda x,y: cmp(x.time, y.time))
        return singlegames

    @cached
    def groupgames(self, nocache=False):
        groupgames = self.subgames()
        groupgames.sort(cmp=lambda x,y: cmp(x.name, y.name))
        return groupgames

    @cached
    def subgames(self, nocache=False):
        return [game for game in GroupGame.everything().itervalues() if self.key() == game.upgroup_key()]

    @cached
    def widewalk(self, nocache=False):
        """ List groupgames as a wide tree full walkthrough. """
        games = [self]
        subgames = self.subgames()
        subgames.sort(key=GroupGame.groupstart)
        for game in subgames:
            games.extend(game.widewalk())
        return games

    @cached
    def teams(self, nocache=False):
        return set(
            [singlegame.homeTeam() for singlegame in self.singlegames()] +
            [singlegame.awayTeam() for singlegame in self.singlegames()])

    @cached
    def groupstart(self, nocache=False):
        return reduce(min,[group.groupstart() for group in self.groupgames()] + [single.time for single in self.singlegames()], datetime.max)

    @classmethod
    @perm_cached_class
    def everything(self):
        games = {}
        for game in self.all().fetch(MAX_ITEMS):
            games[str(game.key())] = game
        return games

    @classmethod
    def byKey(self, key):
        try:
            return self.everything()[str(key)]
        except KeyError:
            return None

    @classmethod
    def nothing(self):
        memcache.delete(self.__name__ + "/everything")
        return []

class SingleGame(db.Model):
    time = db.DateTimeProperty()
    fifaId = db.IntegerProperty()
    homeTeam_ref = db.ReferenceProperty(Team,collection_name="homegame_set")
    awayTeam_ref = db.ReferenceProperty(Team,collection_name="awaygame_set")
    location = db.StringProperty()
    group_ref = db.ReferenceProperty(GroupGame,collection_name="singlegame_set")

    def group_key(self):
        return SingleGame.group_ref.get_value_for_datastore(self)

    def homeTeam_key(self):
        return SingleGame.homeTeam_ref.get_value_for_datastore(self)

    def awayTeam_key(self):
        return SingleGame.awayTeam_ref.get_value_for_datastore(self)

    def homeTeam(self):
        try:
            return Team.everything()[str(self.homeTeam_key())]
        except KeyError:
            return None

    def awayTeam(self):
        try:
            return Team.everything()[str(self.awayTeam_key())]
        except KeyError:
            return None

    def group(self):
        try:
            return GroupGame.everything()[str(self.group_key())]
        except KeyError:
            return None

    @classmethod
    @perm_cached_class
    def everything(self):
        games = {}
        for game in self.all().fetch(MAX_ITEMS):
            games[str(game.key())] = game
        return games

    @classmethod
    def byKey(self, key):
        try:
            return self.everything()[str(key)]
        except KeyError:
            return None

    @classmethod
    def nothing(self):
        memcache.delete(self.__name__ + "/everything")
        return []

    @perm_cached
    def results(self, nocache=False):
        results = {}
        for result in self.result_set.fetch(MAX_ITEMS):
            results[str(result.user.key())] = result
        return results

    def get_ranks(self, user):
        result = user.singlegame_result(self)
        return result.get_ranks()

class UndecidedTeam(db.Model):
    group = db.ReferenceProperty(GroupGame)

from datetime import datetime

class Result(db.Model):
    user = db.ReferenceProperty(LocalUser, collection_name="singleresult_set", required=True)
    singlegame = db.ReferenceProperty(SingleGame, collection_name="result_set", required=True)
    homeScore = db.IntegerProperty()
    awayScore = db.IntegerProperty()
    locked = db.BooleanProperty(default=False)

    @classmethod
    def score_list(self):
        return range(10)

    def home_w(self):
        if self.homeScore >=0 and self.homeScore > self.awayScore: return 1
        else: return 0

    def home_d(self):
        if self.homeScore >=0 and self.homeScore == self.awayScore: return 1
        else: return 0

    def home_l(self):
        if self.homeScore >=0 and self.homeScore < self.awayScore: return 1
        else: return 0

    def home_gf(self):
        if self.homeScore >=0: return self.homeScore
        return 0

    def home_ga(self):
        if self.awayScore >=0: return self.awayScore
        return 0

    def get_home_rank(self):
        return TeamGroupRank(team=self.singlegame.homeTeam(),
                    w=self.home_w(),d=self.home_d(),l=self.home_l(),
                    gf=self.home_gf(),ga=self.home_ga()
                )
    def get_away_rank(self):
        return TeamGroupRank(team=self.singlegame.awayTeam(),
                    w=self.home_l(),d=self.home_d(),l=self.home_w(),
                    gf=self.home_ga(),ga=self.home_gf()
                )
    def get_ranks(self):
        return [self.get_home_rank(),self.get_away_rank()]

class GroupResult(db.Model):
    user = db.ReferenceProperty(LocalUser, collection_name="groupresult_set", required=True)
    groupgame = db.ReferenceProperty(GroupGame, collection_name="result_set", required=True)
    draw_order = db.StringListProperty() # list of the team names
    locked = db.BooleanProperty()

    def get_draw_order(self):
        if self.draw_order is None or len(self.draw_order) == 0:
            self.draw_order = [team.name for team in self.groupgame.teams()]
        return self.draw_order

    @cached
    def get_ranks(self, nocache=False):
        """ Calculate the group standings. 
        
        This turned into one ugly beast.
        """
        # team ordering rules 1,2,3
        ranks = [
            reduce(TeamGroupRank.__add__,                                       # sum up ranks
                filter(lambda x: x.team == team,                                # for the team
                    reduce(lambda x,y: x + y,                                   # "join" all ranks into a single list
                        (singlegame.get_ranks(self.user)
                            for singlegame in self.groupgame.singlegames() )))) # from the singlegames by the user
            for team in self.groupgame.teams()]  # the set of teams
        ranks.sort(reverse=True) # sort the ranks

        # check for ties
        ranks_set = set(ranks)
        if len(ranks_set) == len(ranks): return ranks

        # team ordering rules 4,5,6

        # create a list from the set of ranks
        ranks_set_list = [rank for rank in ranks_set]
        ranks_set_list.sort(reverse=True)
        ranks_no_tie = []

        for rank_tie in ranks_set_list:
            tie_teams = [rank.team for rank in ranks if rank == rank_tie]
            if len(tie_teams) == 1:
                # not a tied rank
                for rank in ranks:
                    if rank.team == tie_teams[0]:
                        ranks_no_tie.append(rank)
            else:
                # apply team ordering rules 1,2,3 to the games played among the tied teams
                tie_ranks = [
                    reduce(TeamGroupRank.__add__,                                      # sum up ranks
                        filter(lambda x: x.team == team,                               # for the team
                            reduce(lambda x,y: x + y,                                  # "join" all ranks into a single list
                                (singlegame.get_ranks(self.user)
                                    for singlegame in self.groupgame.singlegames()
                                        if singlegame.homeTeam() in tie_teams            # both team must be from tied group
                                            and singlegame.awayTeam() in tie_teams))))   # both team must be from tied group
                    for team in tie_teams]
                tie_ranks.sort(reverse=True)
                if len(set(tie_ranks)) != len(tie_ranks):
                    tie_ranks_set_list = [rank for rank in set(tie_ranks)]
                    tie_ranks_set_list.sort(reverse=True)
                    for tie_rank in tie_ranks_set_list:
                        # draw teams are   teams who are in the tie_ranks with the same rank as the tie_rank
                        draw_teams = [rank.team for rank in tie_ranks if rank == tie_rank]
                        for draw in self.get_draw_order():
                            for draw_rank in tie_ranks:
                                for rank in ranks:
                                    if draw_rank.team.name == draw and draw_rank == tie_rank and rank.team == draw_rank.team:
                                        rank.tie = tie_rank
                                        # yes tie, yes draw
                                        rank.draw = draw_teams
                                        ranks_no_tie.append(rank)

                else:
                    for tie_rank in tie_ranks:
                        for rank in ranks:
                            if rank.team == tie_rank.team:
                                rank.tie = tie_rank
                                ranks_no_tie.append(rank)
        return ranks_no_tie
