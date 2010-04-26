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

class TeamRanking:
    team = None
    w = 0
    d = 0
    l = 0
    gf = 0
    ga = 0
    def __init__(self, *args, **namedargs):
        self.__dict__ = namedargs
    def pt(self): return 3 * w + d

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
        """ List groupgames with depth information.

        This method is implemented here to remove the recursion from the view.
        Practically a wide tree search. """
        games = [self]
        for game in self.game_set.order('name'):
            games.extend(game.widewalk())
        return games

    def get_ranking(self, user, singlegames = None):
        rankings = []
        if singlegames is None: singlegames = self.singlegames()
        for singlegame in singlegames:
            result = user.singlegame_result(singlegame)
            for team in [singlegame.homeTeam, singlegame.awayTeam]:
                rankings.append(result.team_ranking(team))
        return rankings

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

    def home_w(self): return self.homeScore >=0 and self.homeScore > self.awayScore
    def home_d(self): return self.homeScore >=0 and self.homeScore == self.awayScore
    def home_l(self): return self.homeScore >=0 and self.homeScore < self.awayScore
    def team_w(self, team):
        if self.singlegame.homeTeam == team: return self.home_w()
        if self.singlegame.awayTeam == team: return self.home_l()
        return False
    def team_d(self, team):
        if self.singlegame.homeTeam == team: return self.home_d()
        if self.singlegame.awayTeam == team: return self.home_d()
        return False
    def team_l(self, team):
        if self.singlegame.homeTeam == team: return self.home_l()
        if self.singlegame.awayTeam == team: return self.home_w()
        return False
    def team_gf(self, team):
        if self.singlegame.homeTeam == team: return self.homeScore
        if self.singlegame.awayTeam == team: return self.awayScore
        return None
    def team_ga(self, team):
        if self.singlegame.homeTeam == team: return self.awayScore
        if self.singlegame.awayTeam == team: return self.homeScore
        return None
    def team_ranking(self, team):
        return TeamRanking(team=team,
            w=self.team_w(team),
            d=self.team_d(team),
            l=self.team_l(team),
            gf=self.team_gf(team),
            ga=self.team_ga(team))

