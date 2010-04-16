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
    group = db.SelfReferenceProperty(collection_name="game_set")
    time = db.DateTimeProperty()

class GroupGame(Game):
    name = db.StringProperty(required=True)

class SingleGame(Game):
    fifaId = db.IntegerProperty()
    homeTeam = db.ReferenceProperty(Team,collection_name="homegame_set")
    awayTeam = db.ReferenceProperty(Team,collection_name="awaygame_set")
    location = db.StringProperty()