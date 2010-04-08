#!/usr/bin/env python
#
# Copyright 2010 Peter Czimmermann  <xczimi@gmail.com>
#
import re
from google.appengine.api import users
from google.appengine.ext import webapp
from google.appengine.ext.webapp import util
from google.appengine.ext.webapp import template

from django.template import TemplateDoesNotExist

class MyRequestHandler(webapp.RequestHandler):
    template_values = None
    def post(self, *args):
        action = self.request.get('action')
        if 'auth/logout' == action:
            self.redirect(users.create_logout_url('/'))
            return True
        elif 'auth/login' == action:
            self.redirect(users.create_login_url(self.request.uri))
            return True
        return False

    def get(self, *args):
        self.template_values = {'user' : users.get_current_user(),'is_admin' : users.is_current_user_admin()}
        
    def render(self, tpl = "index"):
        try:
            self.response.out.write(template.render('view/'+tpl+'.html', self.template_values, debug=True))
        except TemplateDoesNotExist:
            self.template_values['error'] = tpl
            self.response.out.write(template.render('view/error.html', self.template_values, debug=True))
        except:
            raise

class MainHandler(MyRequestHandler):
    def get(self, page):
        MyRequestHandler.get(self, page)
        if '' == page:
            page = "index"
        self.render(page)

from google.appengine.ext import db
from google.appengine.ext.db import polymodel

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


import fifa

class AdminHandler(MyRequestHandler):
    def post(self, admin, *args):
        if MyRequestHandler.post(self):
            return
        print self.request.uri
        
    def init_fifa_team(self, team):
        """Create team if not exists."""
        
        team_stored = Team.all().filter('name =',team['name']).get()
        if team_stored is None:
            team_stored = Team(name=team['name'])

        short_match = re.search(r'([a-z]{3})\.gif$',team['flag'])
        team_stored.flag = team['flag']
        team_stored.short = short_match.group(1)
        team_stored.href = team['href']
        team_stored.put()
        #new_team = Team(name=team['name'])
    
    def init_fifa_tree(self):
        fifa2010 = GroupGame.get_by_key_name("fifa2010")
        if fifa2010 is None:
            fifa2010 = GroupGame.get_or_insert(key_name="fifa2010", name="FIFA 2010")
        groupstage = GroupGame.get_by_key_name("groupstage")
        if groupstage is None:
            groupstage = GroupGame.get_or_insert(key_name="groupstage", name="Group Stage", group=fifa2010)
        kostage = GroupGame.get_by_key_name("kostage")
        if kostage is None:
            kostage = GroupGame.get_or_insert(key_name="kostage", name="KO Stage", group=fifa2010)
        
        games = fifa.get_games("index")
        for game in games:
            self.init_fifa_team(game['home_team'])
            self.init_fifa_team(game['away_team'])

    def get(self, *args):
        MyRequestHandler.get(self)
        if not users.is_current_user_admin():
            self.redirect('/')
            return
        if len(args) > 0:
            admin = args[0]
            if "fifa" == admin:
                self.init_fifa_tree()
            elif "team" == admin:
                if len(args) > 1:
                    self.template_values['team'] = Team.all().filter('short =',args[1]).get()
                    self.render('admin/team')
                else:
                    self.template_values['teams'] = Team.all()
                    self.render('admin/teams')
        else:
            fifa2010 = GroupGame.get_by_key_name("fifa2010")
            self.render('admin/layout')

def main():
    application = webapp.WSGIApplication([('/favicon.ico',webapp.RequestHandler),
                    ('/admin/(team)/(.*)', AdminHandler),
                    ('/admin/(.*)', AdminHandler),
                    ('/admin', AdminHandler),
                    ('/(.*)', MainHandler)],
                                       debug=True)
    util.run_wsgi_app(application)

if __name__ == '__main__':
    main()
