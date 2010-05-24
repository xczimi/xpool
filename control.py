#!/usr/bin/env python
# -*- coding: UTF-8 -*-
#
# Copyright 2010 Peter Czimmermann  <xczimi@gmail.com>
#
import Cookie
import uuid
import re
from datetime import datetime

from google.appengine.api import users
from google.appengine.api import memcache
from google.appengine.api import mail
from google.appengine.ext import webapp
from google.appengine.ext.webapp import util
from google.appengine.ext.webapp import template

from django.template import TemplateDoesNotExist
from django.utils import translation

from google.appengine.ext import db
from google.appengine.ext.db import polymodel

from model import LocalUser, Team, GroupGame, SingleGame, Result, GroupResult

import fifa
from fifa2010 import Fifa2010

import pool

def need_login(func):
    """ Decorator for MyRequestHandler controller classes to force some login. """
    def with_login(self, *args):
        if not self.current_user():
            self.get_template_values()
            self.template_values['message'] = _("Login required")
            self.render('index')
            return
        func(self, *args)
    return with_login


class MyRequestHandler(webapp.RequestHandler):
    session = None
    loggedin_user = None
    template_values = None
    def post(self, *args):
        action = self.request.get('action')
        if 'auth/logout' == action:
            self.logout()
        elif 'auth/glogin' == action:
            self.set_session_message(_('Sucessful google login'))
            self.redirect(users.create_login_url(self.request.uri))
        elif 'auth/login' == action:
            self.login_local(self.request.get('email'),self.request.get('password'))
        elif 'language' == action:
            self.set_session_language(self.request.get('language'))
            self.redirect(self.request.uri)
        else:
            return False
        return True

    def logout(self):
        if self.current_user():
            self.set_session_email(None)
            if users.get_current_user():
                self.set_session_message(_('Successful google logout'))
                self.redirect(users.create_logout_url(self.request.uri))
            else:
                self.set_session_message(_('Successful logout'))
                self.redirect(self.request.uri)
            return True
        return False

    def get_cookie(self, name):
        return self.request.cookies.get(name)

    def set_cookie(self, name, value):
        session = Cookie.SimpleCookie()
        session[name] = value
        self.response.headers.add_header('Set-Cookie', session[name].OutputString())

    def get_session(self):
        if self.session is None:
            cookie_session_id = self.get_cookie('xhomesessionid')
            if cookie_session_id is None:
                session_id = uuid.uuid1().hex
                self.set_cookie('xhomesessionid',session_id)
            else:
                session_id = cookie_session_id
            self.session = memcache.get(session_id)
        if self.session is None:
            self.session = {'id':session_id}
            self.write_session(self.session)
        return self.session

    def write_session(self, session):
        memcache.set(session['id'],session)

    def set_session_email(self, local_user_email):
        session = self.get_session()
        session['email'] = local_user_email
        self.write_session(session)

    def set_session_message(self, message):
        session = self.get_session()
        session['message'] = message
        self.write_session(session)

    def set_session_language(self, language):
        session = self.get_session()
        session['language'] = language
        self.write_session(session)

    def get_session_email(self):
        session = self.get_session()
        try:
            return session['email']
        except:
            return None

    def get_session_message(self):
        session = self.get_session()
        try:
            if session['message'] is None: return ''
            return session['message']
        except:
            return ''

    def get_session_language(self):
        session = self.get_session()
        try:
            if session['language'] is None: return 'en'
            return session['language']
        except:
            return 'en'

    def login_local(self, email, password):
        self.loggedin_user = LocalUser.all().filter('email = ',email).filter('password = ',password).get()
        if self.loggedin_user is None or password == '':
            self.set_session_message(_('Failed to log you in, try it again!'))
            self.redirect(self.request.uri)
        else:
            self.set_session_email(self.loggedin_user.email)
            self.set_session_message(_('Successful login'))
            self.redirect(self.request.uri)

    def login_authcode(self, authcode):
        self.loggedin_user = LocalUser.all().filter('authcode = ',authcode).get()
        if self.loggedin_user is None or authcode == '':
            self.set_session_message(_('Invalid auth code'))
            self.redirect('/')
        else:
            self.set_session_email(self.loggedin_user.email)
            self.set_session_message(_('Successful referral, set your password, or link your google account!'))
            self.redirect('/profile')

    def current_user(self):
        google_user = users.get_current_user()

        if not self.loggedin_user and self.get_session_email():
            # find session based logged in user
            self.loggedin_user = LocalUser.all().filter('email = ',self.get_session_email()).get()

        if not self.loggedin_user:
            if google_user:
                # find the local user based on google_user link
                self.loggedin_user = LocalUser.all().filter('google_user = ',google_user).get()
                if not self.loggedin_user:
                    # find the local user based on google_user's email
                    self.loggedin_user = LocalUser.all().filter('email =',google_user.email()).get()
                    if not self.loggedin_user:
                        # create a new local user for the google_user
                        self.loggedin_user = LocalUser(email=google_user.email(),
                            nick=google_user.nickname(),
                            google_user=google_user,
                            password='',
                            full_name='')
                    else:
                        # link the google and local users together
                        self.loggedin_user.google_user = google_user
                    self.loggedin_user.put()
        else:
            if google_user:
                # link the google and local users together
                if self.loggedin_user.google_user != google_user:
                    self.loggedin_user.google_user = google_user
                    self.loggedin_user.put()
        return self.loggedin_user

    def get_template_values(self):
        message = self.get_session_message()
        if self.template_values is None:
            self.template_values = {
                'user' : self.current_user(),
                'is_admin' : users.is_current_user_admin(),
                'message': message}
        if message is not None: self.set_session_message(None)
        return self.template_values

    def render(self, tpl = "index"):
        translation.activate(self.get_session_language())
        try:
            self.response.out.write(template.render('view/'+tpl+'.html', self.template_values, debug=True))
        except TemplateDoesNotExist:
            self.template_values['error'] = tpl
            self.response.out.write(template.render('view/error.html', self.template_values, debug=True))
        except:
            raise

class MainHandler(MyRequestHandler):
    def get(self, page):
        self.get_template_values()
        if '' == page: page = "index"
        self.render(page)

class UserHandler(MainHandler):
    @need_login
    def get(self, page):
        MainHandler.get(self, page)

    @need_login
    def post(self, *args):
        if MyRequestHandler.post(self): return
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
        if not mail.is_email_valid(email): return
        if LocalUser.all().filter('email =',email).count() > 0:
            self.set_session_message(_('This user is already in the system (based on email)!'))
            return
        referral = LocalUser(email=email,nick=nick,full_name=full_name,referrer=self.current_user())
        referral.authcode = str(uuid.uuid1().hex)
        self.get_template_values()
        self.template_values['referral'] = referral
        self.template_values['auth_url'] = self.request.host_url + '/referral/' + referral.authcode
        referral.put()
        translation.activate(self.get_session_language())
        mail.send_mail(sender = 'Peter Czimmermann <xczimi@gmail.com>',
                to = referral.full_name + u' <' + referral.email + u'>',
                subject = _(u"xHomePool invitation"),
                body = template.render('view/referral.email.html', self.template_values, debug=True))
        self.set_session_message(_('Invitation sent')+' '+self.template_values['auth_url'])

    def save_profile(self, email, nick, full_name):
        user = self.current_user()
        user.email = email
        user.nick = nick
        user.full_name = full_name
        user.put()

    def change_password(self, new_password):
        user = self.current_user()
        user.password = new_password
        user.authcode = None
        user.put()

class GamesHandler(MainHandler):
    def get(self, filter=''):
        self.get_template_values()
        if filter == '': filter = Fifa2010().tournament.key()
        self.template_values['games'] = GroupGame.get(filter).widewalk()
        MainHandler.get(self,'games')


class MyTipsHandler(GamesHandler):
    def post(self, *args):
        if GamesHandler.post(self): return
        action = self.request.get('action')
        if 'mytips/save' == action:
            self.save_current()
            self.redirect(self.request.uri)
        else:
            return False
        return True

    @need_login
    def save_current(self):
        return self.save(self.current_user())

    def save(self, user):
        results = user.singleresults()
        for argument in self.request.arguments():
            match = re.match(r'^(homeScore|awayScore)\.(.*)$', argument)
            if match:
                name, key = match.groups()
                try:
                    result = user.singlegame_result(SingleGame.get(key))
                except db.BadKeyError:
                    continue
                if self.request.get(argument) != '':
                    if name == 'homeScore':
                        result.homeScore = int(self.request.get(argument))
                    elif name == 'awayScore':
                        result.awayScore = int(self.request.get(argument))
                    result.put()
        for argument in self.request.arguments():
            match = re.match(r'^lock\.(.*)$', argument)
            if match:
                key = match.group(1)
                result = self.current_user().singlegame_result(SingleGame.get(key))
                if result.homeScore >= 0 and result.awayScore >= 0:
                    result.locked = True
                    result.put()

        groupinit = []
        for argument in self.request.arguments():
            match = re.match(r'^draw\.(.*)\.([0-9]+)$', argument)
            if match:
                key, idx = match.groups()
                try:
                    groupresult = user.groupgame_result(GroupGame.get(key))
                except db.BadKeyError:
                    continue
                if groupresult not in groupinit:
                    groupinit.append(groupresult)
                    groupresult.draw_order = [rank.team.name for rank in groupresult.get_ranks()]
                groupresult.draw_order[int(idx)] = self.request.get(argument)
        for groupresult in groupinit:
            if len(set(groupresult.draw_order)) == len(groupresult.draw_order):
                if "grouplock." + str(groupresult.groupgame.key()) in self.request.arguments():
                    groupresult.locked = True
                groupresult.put()
            else:
                self.set_session_message(_('Illegal draw'))

    @need_login
    def get(self, filter=''):
        self.get_template_values()
        if filter == '': filter = Fifa2010().tournament.key()

        mytips_games = []
        for game in GroupGame.get(filter).widewalk():
            groupgame = {'game':game,'singlegames':[]}
            for singlegame in game.singlegames():
                bet = self.current_user().singlegame_result(singlegame)
                result = Fifa2010().result.singlegame_result(singlegame)
                point = pool.singlegame_result_point(bet, result)
                groupgame['singlegames'].append({
                    'game':singlegame,
                    'bet':bet,
                    'editable': not bet.locked and not result.locked and datetime.utcnow() < game.groupstart(),
                    'result':result,
                    'point':point})
            mytips_games.append(groupgame)
            groupbet = self.current_user().groupgame_result(game)
            groupresult = Fifa2010().result.groupgame_result(game)
            groupgame['editable'] = not groupbet.locked and not groupresult.locked and datetime.utcnow() < game.groupstart()
            groupgame['bet'] = groupbet
            groupgame['bet_ranking'] = groupbet.get_ranks()
            groupgame['result_ranking'] = groupresult.get_ranks()
            groupgame['point'] = pool.groupgame_result_point(groupbet, groupresult)

        self.template_values['games'] = mytips_games
        self.template_values['scorelist'] = [''] + Result.score_list()

        MainHandler.get(self,'mytips')

class AllTipsHandler(GamesHandler):
    def singlegame_tips(self, singlegame, users):
        tips = []
        results = singlegame.results()
        for user in users:
            if str(user.key()) in results:
                tips.append(results[str(user.key())])
            else:
                tips.append({})
        return tips
    @need_login
    def get(self, filter=''):
        self.get_template_values()
        if filter == '': filter = Fifa2010().tournament.key()

        group = GroupGame.get(filter)
        if(len(group.singlegames()) > 0):
            users = [user for user in LocalUser.all().fetch(LocalUser.all().count())]
            self.template_values['groupgame'] = group
            self.template_values['users'] = users
            self.template_values['alltips'] = [{
                'game':singlegame,
                'tips': self.singlegame_tips(singlegame, users)
                } for singlegame in group.singlegames() if self.current_user().singlegame_result(singlegame)]
            MainHandler.get(self,'alltips')
        else:
            games = []
            for game in group.widewalk():
                games.append({'game':game,'singlegames':[]})
            self.template_values['games'] = games
            MainHandler.get(self,'alltips_tree')
    
class PoolHandler(MainHandler):
    def get(self, filter = ''):
        self.get_template_values()
        if filter == '': filter = Fifa2010().tournament.key()
        groupgame = GroupGame.get(filter)
        self.template_values['groupgame'] = groupgame
        self.template_values['scoreboard'] = pool.scoreboard(LocalUser.all().fetch(100), Fifa2010().result, groupgame)
        MainHandler.get(self,'scoreboard')

class ReferralHandler(MainHandler):
    def get(self, authcode):
        if self.logout():
            return
        self.login_authcode(authcode)

class AdminHandler(MyRequestHandler):
    @need_login
    def get(self, *args):
        self.get_template_values()
        if not users.is_current_user_admin():
            self.redirect(users.create_login_url(self.request.uri))
            return
        if len(args) > 0:
            admin = args[0]
            if "fifa" == admin:
                Fifa2010.init_tree()
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
                    #print pool.score_group(self.current_user(),Fifa2010().result,Fifa2010().tournament)
                    self.render('admin/users')
            elif "result" == admin:
                if len(args) > 1:
                    pass
                else:
                    self.template_values['bets'] = Result.all()
                    self.template_values['groupbets'] = GroupResult.all()
                    self.render('admin/bets')
            else:
                self.render('admin/layout')
        else:
            self.render('admin/layout')
