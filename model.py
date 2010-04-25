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

    def point(self):
        point = 0
        for bet in self.results():
            point = point + bet.point()
        return point

class Team(db.Model):
    name = db.StringProperty(required=True)
    flag = db.StringProperty()
    short = db.StringProperty()
    href = db.StringProperty()

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

    def reference(self):
        reference_user = LocalUser.reference()
        return reference_user.singlegame_result(self.singlegame)

    def point(self, bet, multiplier=1):
        point = 0
        if bet.homeScore >=0 and bet.homeScore == self.homeScore: point = point + 1
        elif bet.homeScore > 4 and self.homeScore > 4: point = point + 1
        if bet.awayScore >=0 and bet.awayScore == self.awayScore: point = point + 1
        elif bet.homeScore > 4 and self.homeScore > 4: point = point + 1
        if bet.home_w() and self.home_w(): point = point + 2
        elif bet.home_d() and self.home_d(): point = point + 2
        elif bet.home_l() and self.home_l(): point = point + 2
        return point * multiplier

