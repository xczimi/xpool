#!/usr/bin/env python
# -*- coding: UTF-8 -*-
#
# Copyright 2010 Peter Czimmermann  <xczimi@gmail.com>
#
import Cookie
import uuid
import re
from datetime import datetime, timedelta

from google.appengine.api import users
from google.appengine.api import memcache
from google.appengine.api import mail

import webapp2
from google.appengine.ext.webapp import template

from django.template import TemplateDoesNotExist
from django.utils import translation
from django.utils.translation import ugettext as _

from google.appengine.ext import db
from google.appengine.ext.db import polymodel

from model import *

import uefa
from fifa2014 import Fifa2014

import pool

import facebook

NOW = datetime.utcnow()
#+timedelta(days=-20)

FACEBOOK_APP_ID = "315323681885269"
FACEBOOK_APP_SECRET = "f5ca1bb05ba16cd8b594d7c2c8e93980"


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


class MyRequestHandler(webapp2.RequestHandler):
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
        return None

    def logout(self):
        if self.current_user():
            self.set_session_email(None)
            if users.get_current_user:
                self.set_session_message(_('Successful google logout'))
                self.redirect(users.create_logout_url(self.request.uri))
            else:
                self.set_session_message(_('Successful logout'))
                self.redirect(self.request.uri)
            return None
        return False

    def get_cookie(self, name):
        return self.request.cookies.get(name)

    def set_cookie(self, name, value):
        session = Cookie.SimpleCookie()
        session[name] = value
        self.response.headers.add_header('Set-Cookie', session[name].OutputString() + "; Path=/")

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

    """Provides access to the active Facebook user in self.fb_current_user

    The property is lazy-loaded on first access, using the cookie saved
    by the Facebook JavaScript SDK to determine the user ID of the active
    user. See http://developers.facebook.com/docs/authentication/ for
    more information.
    """
    @property
    def fb_current_user(self):
        if not hasattr(self, "_fb_current_user"):
            self._fb_current_user = None
            cookie = facebook.get_user_from_cookie(
                self.request.cookies, FACEBOOK_APP_ID, FACEBOOK_APP_SECRET)
            if cookie:
                # Store a local instance of the user data so we don't need
                # a round-trip to Facebook on every request
                fb_user= FacebookUser.get_by_key_name(cookie["uid"])
                if not fb_user:
                    graph = facebook.GraphAPI(cookie["access_token"])
                    profile = graph.get_object("me")
                    fb_user = FacebookUser(key_name=str(profile["id"]),
                                id=str(profile["id"]),
                                name=profile["name"],
                                profile_url=profile["link"],
                                email=profile["email"],
                                access_token=cookie["access_token"])
                    fb_user.put()
                elif fb_user.access_token != cookie["access_token"]:
                    fb_user.access_token = cookie["access_token"]
                    fb_user.put()
                if self.current_user() and str(fb_user.localuser.key()) != str(self.current_user().key()):
                    fb_user.localuser = self.current_user()
                    fb_user.put()
                elif fb_user.localuser is None:
                    self.loggedin_user = LocalUser(email=fb_user.email,
                            nick=fb_user.name,
                            password='',
                            full_name=fb_user.name)
                    self.loggedin_user.put()
                    fb_user.localuser = self.loggedin_user
                    fb_user.put()
                else:
                    self.loggedin_user = fb_user.localuser
                self._fb_current_user = fb_user
        return self._fb_current_user

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

        self.fb_current_user
        return self.loggedin_user
    
    def get_motd(self):
        motdmsg = memcache.get('motd')
        if None == motdmsg:
            motd = Motd.all().get()
            if None == motd:
                motd = Motd(message="Quarter final prediction are available!")
                motd.put()
            motdmsg = motd.message
            memcache.add('motd',motd.message,60)
        return motdmsg

    def get_template_values(self):
        message = self.get_session_message()
        if self.template_values is None:
            self.template_values = {
                'user' : self.current_user(),
                'fb_user' : self.fb_current_user,
                'is_admin' : users.is_current_user_admin(),
                'message': message,
                'motd' : self.get_motd()}
        if message is not None: self.set_session_message(None)
        return self.template_values

    def render(self, tpl = "index"):
        translation.activate(self.get_session_language())
        try:
            self.response.write(template.render('view/'+tpl+'.html', self.template_values, debug=True))
        except TemplateDoesNotExist:
            self.template_values['error'] = tpl
            self.response.write(template.render('view/error.html', self.template_values, debug=True))
        except:
            raise

class MainHandler(MyRequestHandler):
    def get(self, page):
        self.get_template_values()
        if '' == page: page = "index"
        self.render(page)
    def submenu(self, page):
        subgames = GroupGame.everything().values()
        subgames.sort(key=GroupGame.groupstart)
        groupgames = []
        for game in subgames:
            if not game.upgroup() is None and str(game.upgroup().key()) == str(Fifa2014().groupstage.key()):
                groupgames.append(game)
            if not game.upgroup() is None and str(game.upgroup().key()) == str(Fifa2014().kostage.key()):
                groupgames.append(game)

        self.template_values['filtergames'] = groupgames
        self.template_values['filter'] = filter
        self.template_values['filter_uri'] = '/' + page

    def kosubmenu(self, page):
        subgames = GroupGame.everything().values()
        subgames.sort(key=GroupGame.groupstart)
        groupgames = []
        for game in subgames:
            if not game.upgroup() is None and not game.upgroup().upgroup() is None and str(game.upgroup().upgroup().key()) == str(Fifa2014().kostage.key()):
                groupgames.append(game)

        self.template_values['filtergames'] = groupgames
        self.template_values['filter'] = filter
        self.template_values['filter_uri'] = '/' + page

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
        return None

    def refer_user(self, email, nick, full_name):
        if not mail.is_email_valid(email): return
        if LocalUser.all().filter('email =',email).count() > 0:
            self.set_session_message(_('This user is already in the system (based on email)!'))
            return
        referral = LocalUser(email=email,nick=nick,full_name=full_name,referrer=self.current_user(),authcode = str(uuid.uuid1().hex))
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
        if filter == '': filter = 'ko'
        if filter == 'group': filter = Fifa2014().groupstage.key()
        if filter == 'ko': filter = Fifa2014().kostage.key()
        self.template_values['games'] = GroupGame.byKey(filter).widewalk()

        MainHandler.get(self,'games')

class TodayHandler(MainHandler):
    def get(self):
        self.get_template_values()
        singlegames = SingleGame.all().filter('time >=',NOW+timedelta(days=-2)).filter('time <=',NOW+timedelta(days=2)).order('time').fetch(12)
        if self.current_user():
            self.template_values['games'] = [{
                'game':singlegame,
                'bet':self.current_user().singlegame_result(singlegame),
                'result':Fifa2014().result.singlegame_result(singlegame),
                'point':pool.singlegame_result_point(self.current_user().singlegame_result(singlegame), Fifa2014().result.singlegame_result(singlegame))
                } for singlegame in singlegames]
        else:
            self.template_values['games'] = [{
                'game':singlegame,
                'bet':{'locked':False},
                'result':Fifa2014().result.singlegame_result(singlegame)
                } for singlegame in singlegames]
        MainHandler.get(self,'today')


class MyTipsHandler(GamesHandler):
    def post(self, *args):
        if GamesHandler.post(self): return
        action = self.request.get('action')
        if 'mytips/save' == action:
            self.save_current()
            self.redirect(self.request.uri)
        else:
            return False
        return None

    @need_login
    def save_current(self):
        current_user = self.current_user()
        if not current_user.active:
            current_user.active = True
            current_user.put()
        return self.save(current_user)

    def save(self, user):
        results = user.singleresults()
        for argument in self.request.arguments():
            match = re.match(r'^(homeScore|awayScore)\.(.*)$', argument)
            if match:
                name, key = match.groups()
                try:
                    singlegame = SingleGame.byKey(key)
                    singlegame.results(nocache=True)
                    result = user.singlegame_result(singlegame)
                except db.BadKeyError:
                    continue
                except KeyError:
                    continue
                if self.request.get(argument) != '':
                    if name == 'homeScore' and result.homeScore != int(self.request.get(argument)):
                        result.homeScore = int(self.request.get(argument))
                    elif name == 'awayScore' and result.awayScore != int(self.request.get(argument)):
                        result.awayScore = int(self.request.get(argument))
                    else:
                        continue
                    result.put()
        for argument in self.request.arguments():
            match = re.match(r'^lock\.(.*)$', argument)
            if match:
                key = match.group(1)
                try:
                    singlegame = SingleGame.byKey(key)
                    singlegame.results(nocache=True)
                    pool.flush_singlegame(singlegame)
                    result = user.singlegame_result(singlegame)
                except db.BadKeyError:
                    continue
                except KeyError:
                    continue
                if result.homeScore >= 0 and result.awayScore >= 0 and not result.locked:
                    result.locked = True
                    result.put()

        groupinit = []
        for argument in self.request.arguments():
            match = re.match(r'^draw\.(.*)\.([0-9]+)$', argument)
            if match:
                key, idx = match.groups()
                try:
                    groupgame = GroupGame.byKey(key)
                except KeyError:
                    continue
                except db.BadKeyError:
                    continue
                groupresult = user.groupgame_result(groupgame)
                if groupresult not in groupinit:
                    groupinit.append(groupresult)
                    groupresult.draw_order = [rank.team.name for rank in groupresult.get_ranks()]
                groupresult.draw_order[int(idx)] = self.request.get(argument)
        for argument in self.request.arguments():
            match = re.match(r'grouplock\.(.*)$', argument)
            if match:
                key = match.group(1)
                try:
                    groupgame = GroupGame.byKey(key)
                except KeyError:
                    print "KeyError"
                    continue
                except db.BadKeyError:
                    print "BadKeyError"
                    continue
                pool.flush_groupgame(groupgame)
                groupresult = user.groupgame_result(groupgame)
                groupresult.locked = True
                groupresult.put()
        user.groupresults(nocache=True)
        user.singleresults(nocache=True)

    @need_login
    def get(self, filter=''):
        self.get_template_values()
        self.submenu('mytips')
        if filter == '':
            filtered = [game for game in SingleGame.everything().itervalues() if game.time+timedelta(minutes=120) > NOW]
            filtered.sort(cmp=lambda x,y: cmp(x.time, y.time))
            if len(filtered) > 0:
                if filtered[0] != None:
                    filter = filtered[0].group().key()
        groupgame = GroupGame.get(filter)
        if len(groupgame.singlegames()) == 0:
            groupgames = groupgame.groupgames()
        else:
            groupgames = [groupgame]
        tpl_groupgames = []
        for game in groupgames:
            if len(game.singlegames()) > 0:
                groupgame = {'game':game,'singlegames':[]}
                for singlegame in game.singlegames():
                    bet = self.current_user().singlegame_result(singlegame)
                    result = Fifa2014().result.singlegame_result(singlegame)
                    point = pool.singlegame_result_point(bet, result)
                    groupgame['singlegames'].append({
                        'game':singlegame,
                        'bet':bet,
                        'editable': (str(self.current_user().key()) == str(Fifa2014().result.key()) and NOW > singlegame.time and not bet.locked)
                            or (not bet.locked and not result.locked and NOW < game.groupstart()),
                        'result':result,
                        'point':point})
                groupbet = self.current_user().groupgame_result(game)
                groupresult = Fifa2014().result.groupgame_result(game)
                groupgame['editable'] = (not groupbet.locked and not groupresult.locked and NOW < game.groupstart()) or str(self.current_user().key()) == str(Fifa2014().result.key())
                groupgame['bet'] = groupbet
                groupgame['bet_ranking'] = groupbet.get_ranks()
                groupgame['result_ranking'] = groupresult.get_ranks()
                groupgame['point'] = pool.groupgame_result_point(groupbet, groupresult)
                tpl_groupgames.append(groupgame)

        self.template_values['groupgames'] = tpl_groupgames
        self.template_values['scorelist'] = [''] + Result.score_list()

        MainHandler.get(self,'mytips')

class AllTipsHandler(GamesHandler):
    def singlegame_tips(self, singlegame, users):
        tips = []
        results = singlegame.results()
        for user in users:
            if str(user.key()) in results and (results[str(user.key())] or NOW > singlegame.time):
                tips.append(results[str(user.key())])
            else:
                tips.append({})
        return tips
    def groupgame_tips(self, groupgame, users):
        tips = []
        for user in users:
            tips.append(user.groupgame_result(groupgame).get_ranks())
        return tips
    @need_login
    def get(self, filter=''):
        self.get_template_values()
        self.submenu('alltips')
        if filter == '':
            filtered = [game for game in SingleGame.everything().itervalues() if game.time <= NOW]
            filtered.sort(cmp=lambda x,y: cmp(x.time, y.time), reverse=True)
            if len(filtered) > 0:
                filter = filtered[0].group().key()
            else:
                filter = Fifa2014().groupstage.key() 

        users = [user for user in LocalUser.actives()]
        self.template_values['users'] = users

        groupgame = GroupGame.get(filter)
        if len(groupgame.singlegames()) == 0:
            groupgames = groupgame.groupgames()
        else:
            groupgames = [groupgame]
        tpl_groupgames = []

        for group in groupgames:
            tpl_groupgame = {}
            if(len(group.singlegames()) > 0):
                tpl_groupgame['groupgame'] = group
                tpl_groupgame['alltips'] = []
                for singlegame in group.singlegames():
                    result = self.current_user().singlegame_result(singlegame)
                    if result.locked or NOW > group.groupstart():
                        tpl_groupgame['alltips'].append({'game': singlegame,
                            'tips': self.singlegame_tips(singlegame, users)
                            })
                    else:
                        tpl_groupgame['alltips'].append({'game': singlegame})
                tpl_groupgame['ranks'] = self.groupgame_tips(group, users)
            tpl_groupgames.append(tpl_groupgame)

        self.template_values['groupgames'] = tpl_groupgames
        MainHandler.get(self,'alltips')

class PoolHandler(MainHandler):
    def get(self, filter = ''):
        self.get_template_values()
        self.submenu('scoreboard')
        if filter == '': filter = Fifa2014().tournament.key()
        groupgame = GroupGame.get(filter)

        self.template_values['groupgame'] = groupgame
        self.template_values['multiplier'] = pool.group_multiplier(groupgame)
        scoreboard = pool.scoreboard(LocalUser.actives(), Fifa2014().result, groupgame)
        self.template_values['scoreboard'] = scoreboard

        subgroups = groupgame.subgames()
        subgroups.sort(key=GroupGame.groupstart)
        self.template_values['subgames'] = subgroups
        self.template_values['subboards'] = [{'game':subgroup,'multiplier':pool.group_multiplier(subgroup),'scorelines':pool.scoreboard(LocalUser.actives(), Fifa2014().result, subgroup)} for subgroup in subgroups]

        MainHandler.get(self,'scoreboard')

class PerfectHandler(MainHandler):
    def get(self):
        self.get_template_values()
        self.template_values['perfects'] = pool.perfects_group(LocalUser.actives(),Fifa2014().result,Fifa2014().tournament)
        self.render('perfect')

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
            if "flush" == admin:
                perm_cached_class(None, flush=True)
            if "fifa" == admin:
                Fifa2014.init_tree()
                perm_cached_class(None, flush=True)
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
                    game = SingleGame.byKey(args[1])
                    if self.request.get('save') == "Submit":
                        game.homeTeam_ref = Team.byKey(self.request.get('homeTeam'))
                        game.awayTeam_ref = Team.byKey(self.request.get('awayTeam'))
                        game.put()
                    self.template_values['game'] = game
                    self.template_values['teams'] = Team.all()
                    self.render('admin/game')
                    pass
                else:
                    self.template_values['groupgames'] = GroupGame.all().order('name')
                    self.template_values['singlegames'] = SingleGame.all().order('time')
                    self.render('admin/games')
            elif "user" == admin:
                if len(args) > 1:
                    pass
                else:
                    self.template_values['users'] = LocalUser.all()
                    #print pool.score_group(self.current_user(),Fifa2014().result,Fifa2014().tournament)
                    self.render('admin/users')
            elif "result" == admin:
                if self.request.get('save') == "Submit":
                    user = LocalUser.get(self.request.get('filter_user'))
                    singlegame = SingleGame.byKey(self.request.get('filter_game'))
                    singlegame.results(nocache=True)
                    result = user.singlegame_result(singlegame)
                    result.homeScore = int(self.request.get('homeScore'))
                    result.awayScore = int(self.request.get('awayScore'))
                    result.locked = True
                    result.put()
                    pool.flush_singlegame(singlegame)
                    user.singleresults(nocache=True)
                self.template_values['users'] = LocalUser.actives()
                self.template_values['singlegames'] = [{'key':game.key(),
                    'groupname':game.group().name,
                    'time':game.time,
                    'homeTeam':game.homeTeam(),
                    'awayTeam':game.awayTeam()} for game in SingleGame.everything().itervalues() if game.homeTeam()]
                self.template_values['singlegames'].sort(cmp=lambda x,y: cmp(x['time'], y['time']))
                self.template_values['singlegames'].sort(cmp=lambda x,y: cmp(x['groupname'], y['groupname']))

                betQuery = Result.all()
                groupbetQuery = GroupResult.all()

                filter_game_key = self.request.get("filter_game")
                if filter_game_key:
                    filter_game = SingleGame.everything()[filter_game_key]
                    self.template_values['filter_game'] = filter_game
                    betQuery = betQuery.filter("singlegame = ",filter_game)
                    groupbetQuery = groupbetQuery.filter("singlegame = ",filter_game)

                filter_user_key = self.request.get("filter_user")
                if filter_user_key:
                    filter_user = LocalUser.get(filter_user_key)
                    self.template_values['filter_user'] = filter_user
                    betQuery = betQuery.filter("user = ",filter_user)
                    groupbetQuery = groupbetQuery.filter("user = ",filter_user)
                self.template_values['bets'] = betQuery.fetch(32)
                self.template_values['groupbets'] = groupbetQuery.fetch(32)
                self.render('admin/bets')
            else:
                self.render('admin/layout')
        else:
            self.render('admin/layout')
