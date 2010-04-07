#!/usr/bin/env python
#
# Copyright 2010 Peter Czimmermann  <xczimi@gmail.com>
#

from google.appengine.api import users
from google.appengine.ext import webapp
from google.appengine.ext.webapp import util
from google.appengine.ext.webapp import template

class MyRequestHandler(webapp.RequestHandler):
    def post(self, *args):
        action = self.request.get('action')
        if 'auth/logout' == action:
            self.redirect(users.create_logout_url('/'))
            return
        elif 'auth/login' == action:
            self.redirect(users.create_login_url(self.request.uri))
            return 
        return False

    def template_values(self):
        return {'user' : users.get_current_user(),'is_admin' : users.is_current_user_admin()}

class MainHandler(MyRequestHandler):
    def get(self, page):
        template_values = self.template_values()
        if '' == page:
            page = "index"
        try:
            self.response.out.write(template.render('view/'+page+'.html', template_values, debug=True))
        #except TemplateDoesNotExist:
        except:
            #self.redirect('/')
            raise

from google.appengine.ext import db
from google.appengine.ext.db import polymodel

class Team(db.Model):
    name = db.StringProperty(required=True)
    flag = db.StringProperty()
    short = db.StringProperty()

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

class FifaImport():
    def list_teams(self, games):
        pass

class AdminHandler(MyRequestHandler):
    def post(self, admin, *args):
        if None == MyRequestHandler.post(self):
            return
        print self.request.uri
    
    def init_fifa_team(self, team):
        """Create team if not exists."""
        
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
        print games
        for game in games:
            self.init_fifa_team(game['home_team'])
            self.init_fifa_team(game['away_team'])
        #self.response.out.write(fifa2010)
    
    def get(self, admin, id):
        if not users.is_current_user_admin():
            self.redirect('/')
            return
        
        template_values = self.template_values()
        if "fifa" == admin:
            self.init_fifa_tree()
        elif "team" == admin:
            if "" == id:
                template_values['teams'] = Team.all()
                self.response.out.write(template.render('view/admin/teams.html', template_values))
            else:
                template_values['team'] = Team.get(id)
                self.response.out.write(template.render('view/admin/team.html', template_values))
        else:
            fifa2010 = GroupGame.get_by_key_name("fifa2010")
            self.response.out.write(template.render('view/admin/layout.html', template_values, debug=True))


def main():
    application = webapp.WSGIApplication([('/admin/?(.*)/?(.*)', AdminHandler),('/favicon.ico',webapp.RequestHandler),('/(.*)', MainHandler)],
                                       debug=True)
    util.run_wsgi_app(application)

if __name__ == '__main__':
    main()
