#!/usr/bin/env python
#
# Copyright 2010 Peter Czimmermann  <xczimi@gmail.com>
#

import os
from google.appengine.ext import webapp
from google.appengine.ext.webapp import util
from google.appengine.ext.webapp import template

class MainHandler(webapp.RequestHandler):
    def get(self):
        template_values = None
        self.response.out.write(template.render('view/index.html', template_values))

from google.appengine.ext import db
from google.appengine.ext.db import polymodel

class Team(db.Model):
    name = db.StringProperty(required=True)
    flag = db.StringProperty()
    short = db.StringProperty()

class Game(polymodel.PolyModel):
    group = db.SelfReferenceProperty(collection_name="game_set")

class GroupGame(Game):
    name = db.StringProperty(required=True)

class SingleGame(Game):
    homeTeam = db.ReferenceProperty(Team,collection_name="homegame_set")
    awayTeam = db.ReferenceProperty(Team,collection_name="awaygame_set")
   
class TeamHandler(webapp.RequestHandler):
    def get(self, id):
        self.response.out.write("TEAM"+id)

def main():
    application = webapp.WSGIApplication([('/', MainHandler),
                                        ('/team/(.*)', TeamHandler),],
                                       debug=True)
    util.run_wsgi_app(application)


if __name__ == '__main__':
    main()
