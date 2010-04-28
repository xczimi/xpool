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

from django.template import TemplateDoesNotExist

from google.appengine.ext import db
from google.appengine.ext.db import polymodel

def cached(func):
    def cached_func(self):
        cache_prop = '_'+func.__name__
        if cache_prop not in self.__dict__:
            self.__dict__[cache_prop] = func(self)
        return self.__dict__[cache_prop]
    return cached_func

class LocalUser(db.Model):
    email = db.EmailProperty(required=True)
    nick = db.StringProperty()
    full_name = db.StringProperty()
    google_user = db.UserProperty()
    password = db.StringProperty()
    authcode = db.StringProperty()
    referrer = db.SelfReferenceProperty(collection_name="referral_set")
    def __str__(self):
        if self.nick: return self.nick.encode('utf-8')
        return self.email.encode('utf-8')

    @cached
    def results(self):
        """ Returns a dict of the users Result objects keyed by the singlegame (cached). """
        results = {}
        for result in self.result_set.fetch(SingleGame.all().count()):
            results[str(result.singlegame.key())] = result
        return results

    def singlegame_result(self, singlegame):
        if not str(singlegame.key()) in self.results():
            self.results()[str(singlegame.key())] = Result(user=self,singlegame=singlegame)
        return self.results()[str(singlegame.key())]


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

class TeamGroupRank:
    team = None
    tie = None
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


class Game(polymodel.PolyModel):
    time = db.DateTimeProperty()

class GroupGame(Game):
    name = db.StringProperty(required=True)
    upgroup = db.SelfReferenceProperty(collection_name="game_set")

    def level(self):
        if self.upgroup is None: return 1
        return self.upgroup.level() + 1

    @cached
    def singlegames(self):
        return self.singlegame_set.order('time').fetch(SingleGame.all().count())

    @cached
    def groupgames(self):
        return self.game_set.order('name').fetch(GroupGame.all().count())

    @cached
    def widewalk(self):
        """ List groupgames as a wide tree full walkthrough. """
        games = [self]
        for game in self.game_set.order('name'):
            games.extend(game.widewalk())
        return games

    def teams(self):
        return set(
            [singlegame.homeTeam for singlegame in self.singlegames()] +
            [singlegame.awayTeam for singlegame in self.singlegames()])

    def get_ranks(self, user):
        ranks = [
            reduce(TeamGroupRank.__add__,                                              # sum up ranks
                filter(lambda x: x.team == team,                                       # for the team
                    reduce(lambda x,y: x + y,                                          # "join" all ranks into a single list
                        (singlegame.get_ranks(user) for singlegame in self.singlegames() )))) # from the singlegames by the user
            for team in self.teams()]  # the set of teams
        ranks.sort(reverse=True) # sort the ranks

        ranks_set = set(ranks)
        if len(ranks_set) == len(ranks): return ranks,
        #print "Tie breaking rules 4,5,6"

        ranks_set_list = [rank for rank in ranks_set]
        ranks_set_list.sort(reverse=True)
        ranks_no_tie = []

        tie_draws = []
        #print ranks_set_list
        for rank_tie in ranks_set_list:
            tie_teams = [rank.team for rank in ranks if rank == rank_tie]
            if len(tie_teams) == 1:
                for rank in ranks:
                    if rank.team == tie_teams[0]:
                        ranks_no_tie.append(rank)
            else:
                tie_ranks = [
                    reduce(TeamGroupRank.__add__,                                              # sum up ranks
                        filter(lambda x: x.team == team,                                       # for the team
                            reduce(lambda x,y: x + y,                                          # "join" all ranks into a single list
                                (singlegame.get_ranks(user)
                                    for singlegame in self.singlegames()
                                        if singlegame.homeTeam in tie_teams and singlegame.awayTeam in tie_teams))))
                    for team in tie_teams]
                tie_ranks.sort(reverse=True)
                if len(set(tie_ranks)) != len(tie_ranks):
                    tie_draws.append(tie_teams)
                for tie_rank in tie_ranks:
                    for rank in ranks:
                        if rank.team == tie_rank.team:
                            rank.tie = tie_rank
                            ranks_no_tie.append(rank)
        return ranks_no_tie, tie_draws

class SingleGame(Game):
    fifaId = db.IntegerProperty()
    homeTeam = db.ReferenceProperty(Team,collection_name="homegame_set")
    awayTeam = db.ReferenceProperty(Team,collection_name="awaygame_set")
    location = db.StringProperty()
    group = db.ReferenceProperty(GroupGame,collection_name="singlegame_set")

    @cached
    def results(self):
        results = {}
        for result in self.result_set.all().fetch(LocalUser.all().count()):
            results[str(result.user.key())] = result
        return results

    def get_ranks(self, user):
        result = user.singlegame_result(self)
        return result.get_ranks()

from datetime import datetime

class Result(db.Model):
    user = db.ReferenceProperty(LocalUser, collection_name="result_set", required=True)
    singlegame = db.ReferenceProperty(SingleGame, collection_name="result_set", required=True)
    homeScore = db.IntegerProperty()
    awayScore = db.IntegerProperty()
    locked = db.BooleanProperty(default=False)
    def editable(self):
        """ This probably should be in the control (as it's only valid for regular users Results). """
        return datetime.utcnow() < self.singlegame.time

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
        return TeamGroupRank(team=self.singlegame.homeTeam,
                    w=self.home_w(),d=self.home_d(),l=self.home_l(),
                    gf=self.home_gf(),ga=self.home_ga()
                )
    def get_away_rank(self):
        return TeamGroupRank(team=self.singlegame.awayTeam,
                    w=self.home_l(),d=self.home_d(),l=self.home_w(),
                    gf=self.home_ga(),ga=self.home_gf()
                )
    def get_ranks(self):
        return [self.get_home_rank(),self.get_away_rank()]

class GroupResult(db.Model):
    user = db.ReferenceProperty(LocalUser, collection_name="groupresult_set", required=True)
