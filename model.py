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

class SingleGame(Game):
    fifaId = db.IntegerProperty()
    homeTeam = db.ReferenceProperty(Team,collection_name="homegame_set")
    awayTeam = db.ReferenceProperty(Team,collection_name="awaygame_set")
    location = db.StringProperty()
    group = db.ReferenceProperty(GroupGame,collection_name="singlegame_set")

from datetime import datetime
class Result(db.Model):
    user = db.ReferenceProperty(LocalUser, collection_name="result_set", required=True)
    singlegame = db.ReferenceProperty(SingleGame, collection_name="result_set", required=True)
    homeScore = db.IntegerProperty()
    awayScore = db.IntegerProperty()
    locked = db.BooleanProperty(default=False)
    def editable(self):
        return datetime.utcnow() < self.singlegame.time
    @classmethod
    def score_list(self):
        return range(10)
