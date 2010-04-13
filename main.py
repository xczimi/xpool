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

class MyRequestHandler(webapp.RequestHandler):
    session = {'id':None,'email':None}
    loggedin_user = None
    template_values = None
    def post(self, *args):
        action = self.request.get('action')
        if 'auth/logout' == action:
            if not users.get_current_user() is None:
                self.redirect(users.create_logout_url('/'))
            else:
                self.set_session_email(None)
                self.redirect('/')
        elif 'auth/glogin' == action:
            self.redirect(users.create_login_url(self.request.uri))
        elif 'auth/login' == action:
            self.login_local(self.request.get('email'),self.request.get('password'))
        else:
            return False
        return True
    
    def get_session(self):
        if self.session['id'] is None:
            if self.request.cookies.get('xhomesession') is None:
                self.session['id'] = uuid.uuid1().hex
                session = Cookie.SimpleCookie()
                session['xhomesession'] = self.session['id']
                self.response.headers.add_header('Set-Cookie', session['xhomesession'].OutputString())
            else:
                self.session['id'] = self.request.cookies.get('xhomesession')
        if self.session['email'] is None:
            self.session['email'] = memcache.get(self.session['id'])
        return self.session
        
    def set_session_email(self, local_user_email):
        session = self.get_session()
        session['email'] = local_user_email
        memcache.set(session['id'],session['email'])
    
    def get_session_email(self):
        session = self.get_session()
        return session['email']
    
    def login_local(self, email, password):
        self.loggedin_user = LocalUser.all().filter('email = ',email).filter('password = ',password).get()
        if self.loggedin_user is None:
            self.redirect('/loginerror')
        else:
            self.set_session_email(self.loggedin_user.email)
            self.redirect('/')
    
    def current_user(self):
        if users.get_current_user():
            google_user = users.get_current_user()
            self.loggedin_user = LocalUser.all().filter('google_user = ',google_user).get()
            if self.loggedin_user is None:
                self.loggedin_user = LocalUser(email=google_user.email(),
                    google_user=google_user,
                    nick=google_user.nickname(),
                    password=u'tompika',
                    full_name='')
                self.loggedin_user.put()                
            elif not self.loggedin_user.google_user == google_user:
                self.loggedin_user.google_user = google_user
                self.loggedin_user.put()
        else:
            self.loggedin_user = LocalUser.all().filter('email = ',self.get_session_email()).get()
        return self.loggedin_user
    
    def get_template_values(self):
        self.template_values = {'user' : self.current_user(), 'is_admin' : users.is_current_user_admin()}
        return self.template_values
    
    def get(self, *args):
        self.get_template_values()
        
    def render(self, tpl = "index"):
        try:
            self.response.out.write(template.render('view/'+tpl+'.html', self.template_values, debug=True))
        except TemplateDoesNotExist:
            self.template_values['error'] = tpl
            self.response.out.write(template.render('view/error.html', self.template_values, debug=True))
        except:
            raise

class MainHandler(MyRequestHandler):
    def post(self, *args):
        if MyRequestHandler.post(self):
            return
        action = self.request.get('action')
        if 'user/profile' == action:
            self.save_profile(email = self.request.get('email'),
                                nick = self.request.get('nick'),
                                full_name = self.request.get('full_name'))
            if '' != self.request.get('password') and self.request.get('password') == self.request.get('password_check'):
                self.change_password(new_password = self.request.get('password'))
            self.redirect(self.request.uri)
        elif 'user/refer' == action:
            self.refer_user(email = self.request.get('email'),
                        nick = self.request.get('nick'),
                        full_name = self.request.get('full_name'))
            self.redirect(self.request.uri)
        else:
            return False
        return True
    
    def refer_user(self, email, nick, full_name):
        referral = LocalUser(email=email,nick=nick,full_name=full_name,referrer=self.current_user())
        referral.authcode = str(uuid.uuid1().hex)
        self.get_template_values()
        self.template_values['referral'] = referral
        self.template_values['auth_url'] = self.request.host_url + '/referral/' + referral.authcode
        referral.put()
        mail.send_mail(sender = 'Peter Czimmermann <xczimi@gmail.com>',
                to = referral.full_name + u'<' + referral.email + u'>',
                subject = _(u"xHomePool meghívó"), 
                body = template.render('view/referral.email', self.template_values, debug=True))

    def save_profile(self, email, nick, full_name):
        user = self.current_user()
        user.email = email
        user.nick = nick
        user.full_name = full_name
        user.put()

    def change_password(self, new_password):
        user = self.current_user()
        user.password = new_password
        user.put()
    
    def get(self, page):
        MyRequestHandler.get(self, page)
        if '' == page:
            page = "index"
        self.render(page)

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
        if self.nick:
            return self.nick.encode('utf-8')
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
            elif "user" == admin:
                if len(args) > 1:
                    pass
                else:
                    self.template_values['users'] = LocalUser.all()
                    self.render('admin/users')
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
