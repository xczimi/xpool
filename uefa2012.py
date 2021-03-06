from google.appengine.ext import db
from google.appengine.ext.db import polymodel

from model import Team, GroupGame, SingleGame, LocalUser, Result

import uefa
import re


class Uefa2012(object):
    """ Uefa2012 class should hold all the UEFA 2012 EC specific knowledge. Was copied from Fifa2010 """
    __shared_state = {}

    tournament = None
    groupstage = None
    kostage = None
    result = None

    def __init__(self):
        self.__dict__ = self.__shared_state

        # try to get stuff by key
        if self.tournament is None: self.tournament = GroupGame.get_by_key_name("uefa2012")
        if self.groupstage is None: self.groupstage = GroupGame.get_by_key_name("uefa2012groupstage")
        if self.kostage is None: self.kostage = GroupGame.get_by_key_name("uefa2012kostage")
        if self.result is None: self.result = LocalUser.get_by_key_name("result")

    @classmethod
    def get_group(cls, key):
        return GroupGame.all().filter('name = ',key).get()

    @classmethod
    def init_team(self, team):
        """Create team if not exists."""

        team_stored = Team.all().filter('name =',team['name']).get()
        if team_stored is None:
            team_stored = Team(name=team['name'])
            short_match = re.search(r'([a-z]{3})\.png$',team['flag'])
            team_stored.flag = team['flag']
            team_stored.short = short_match.group(1)
            team_stored.href = "http://www.uefa.com/" + team['href']
            team_stored.teamId = re.match(r'/uefaeuro/season=2012/teams/team=([0-9]+)/index.html', team['href']).group(1)
            team_stored.put()

        return team_stored

    @classmethod
    def init_group(self, group_name, upgroup):
        """Create group if not exists."""

        group_stored = GroupGame.all().filter('name =',group_name).get()
        if group_stored is None:
            group_stored = GroupGame(name=group_name, upgroup_ref = upgroup)
            group_stored.put()
        return group_stored

    @classmethod
    def init_group_game(self, game):
        """Create game if not exists."""

        group = self.init_group(game['group'],Uefa2012().groupstage)
        game_stored = SingleGame.all().filter('fifaId =',int(game['id'])).get()
        if game_stored is None:
            game_stored = SingleGame(fifaId=int(game['id']),group = group)
        game_stored.time = game['time']
        game_stored.location = game['location']
        game_stored.group_ref = group
        game_stored.homeTeam_ref = self.init_team(game['home_team'])
        game_stored.awayTeam_ref = self.init_team(game['away_team'])
        game_stored.put()

    @classmethod
    def init_ko_game(self, game):
        """Create ko game if not exists."""
        group = self.init_group(game['group'],Uefa2012().kostage)
        kogroup = self.init_group('KO ' + str(game['id']), group)

        game_stored = SingleGame.all().filter('fifaId =',int(game['id'])).get()
        if game_stored is None:
            game_stored = SingleGame(fifaId = int(game['id']), group = kogroup)
        game_stored.time = game['time']
        game_stored.location = game['location']
        game_stored.group_ref = kogroup
        game_stored.homeTeam_ref = self.init_team(game['home_team'])
        game_stored.awayTeam_ref = self.init_team(game['away_team'])
        game_stored.put()

    @classmethod
    def init_tree(self):
        # try to create that stuff safely
        if Uefa2012().tournament is None: Uefa2012().tournament = GroupGame.get_or_insert(key_name="uefa2012", name="UEFA 2012")
        if Uefa2012().groupstage is None: Uefa2012().groupstage = GroupGame.get_or_insert(key_name="uefa2012groupstage", name="Group Stage", upgroup_ref = Uefa2012().tournament)
        if Uefa2012().kostage is None: Uefa2012().kostage = GroupGame.get_or_insert(key_name="uefa2012kostage", name="KO Stage", upgroup_ref = Uefa2012().tournament)
        if Uefa2012().result is None: Uefa2012().result = LocalUser.get_or_insert(key_name="result", email="uefa@uefa.com", password='uefa')
        #groupgames = uefa.get_games("group")
        #for game in groupgames: self.init_group_game(game)
        kogames = uefa.get_games("kostage")
        for kogame in kogames: self.init_ko_game(kogame)
