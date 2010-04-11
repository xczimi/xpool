#!/usr/bin/env python
#
# Copyright 2010 Peter Czimmermann  <xczimi@gmail.com>
#
import Cookie
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

class LocalUser(db.Model):
    email = db.EmailProperty(required=True)
    nick = db.StringProperty(required=True)
    name = db.StringProperty()
    google_user = db.UserProperty()
    password = db.StringProperty()
    authcode = db.StringProperty()

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
        
        return team_stored
    
    def init_fifa_group(self, group_name):
        """Create group if not exists."""

        group_stored = GroupGame.all().filter('name =',group_name).get()
        if group_stored is None:
            group_stored = GroupGame(name=group_name)
        
        group_stored.group=GroupGame.get_by_key_name("groupstage")
        group_stored.put()
        return group_stored
        
    def init_fifa_group_game(self, game):
        """Create game if not exists."""
        
        group = self.init_fifa_group(game['group'])
        game_stored = SingleGame.all().filter('id =',game['id']).get()
        if game_stored is None:
            game_stored = SingleGame(fifaId=int(game['id']),group=self.fifa_groupstage())
        
        game_stored.time = game['time']
        game_stored.location = game['location']
        game_stored.group = self.init_fifa_group(game['group'])
        game_stored.homeTeam = self.init_fifa_team(game['home_team'])
        game_stored.awayTeam = self.init_fifa_team(game['away_team'])
        game_stored.put()
    
    def fifa_root(self):
        fifa2010 = GroupGame.get_by_key_name("fifa2010")
        if fifa2010 is None:
            fifa2010 = GroupGame.get_or_insert(key_name="fifa2010", name="FIFA 2010")
        return fifa2010
    
    def fifa_groupstage(self):
        groupstage = GroupGame.get_by_key_name("groupstage")
        if groupstage is None:
            groupstage = GroupGame.get_or_insert(key_name="groupstage", name="Group Stage", group=self.fifa_root())
        return groupstage

    def fifa_kostage(self):
        kostage = GroupGame.get_by_key_name("kostage")
        if kostage is None:
            kostage = GroupGame.get_or_insert(key_name="kostage", name="KO Stage", group=self.fifa_root())
        return kostage
    
    def init_fifa_tree(self):
        games = fifa.get_games("index")
        for game in games:
            self.init_fifa_group_game(game)

    def get(self, *args):
        MyRequestHandler.get(self)
        if not users.is_current_user_admin():
            self.redirect(users.create_login_url(self.request.uri))
            return
        if len(args) > 0:
            admin = args[0]
            if "fifa" == admin:
                self.init_fifa_tree()
                self.redirect('/admin/team')
            elif "team" == admin:
                if len(args) > 1:
                    self.template_values['team'] = Team.all().filter('short =',args[1]).get()
                    self.render('admin/team')
                else:
                    self.template_values['teams'] = Team.all()
                    self.render('admin/teams')
            elif "game" == admin:
                if len(args) > 1:
                    pass
                else:
                    self.template_values['groupgames'] = GroupGame.all()
                    self.template_values['singlegames'] = SingleGame.all()
                    self.render('admin/games')
            else:
                self.render('admin/layout')
        else:
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
