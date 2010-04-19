from google.appengine.ext import db
from google.appengine.ext.db import polymodel

from model import Team, Game, GroupGame, SingleGame, LocalUser, Result

import fifa
import re


class Fifa2010(object):
    """ Fifa2010 class should hold all the FIFA 2010 WC specific knowledge. """
    __shared_state = {}

    tournament = None
    groupstage = None
    kostage = None
    result = None

    def __init__(self):
        self.__dict__ = self.__shared_state
        
        # try to get stuff by key
        if self.tournament is None: self.tournament = GroupGame.get_by_key_name("fifa2010")
        if self.groupstage is None: self.groupstage = GroupGame.get_by_key_name("groupstage")
        if self.kostage is None: self.kostage = GroupGame.get_by_key_name("kostage")
        if self.result is None: self.result = LocalUser.get_by_key_name("result")
        
    @classmethod
    def get_group(cls, key):
        return GroupGame.all().filter('name = ',key).get()

    @classmethod
    def init_team(self, team):
        """Create team if not exists."""
        
        team_stored = Team.all().filter('name =',team['name']).get()
        if team_stored is None: team_stored = Team(name=team['name'])
        short_match = re.search(r'([a-z]{3})\.gif$',team['flag'])
        team_stored.flag = team['flag']
        team_stored.short = short_match.group(1)
        team_stored.href = team['href']
        team_stored.put()
        
        return team_stored
    
    @classmethod
    def init_group(self, group_name):
        """Create group if not exists."""

        group_stored = GroupGame.all().filter('name =',group_name).get()
        if group_stored is None: 
            group_stored = GroupGame(name=group_name, upgroup = Fifa2010().groupstage)
            group_stored.put()
        return group_stored
        
    @classmethod
    def init_group_game(self, game):
        """Create game if not exists."""
        
        group = self.init_group(game['group'])
        game_stored = SingleGame.all().filter('fifaId =',int(game['id'])).get()
        if game_stored is None: game_stored = SingleGame(fifaId=int(game['id']),group = group)
        
        game_stored.time = game['time']
        game_stored.location = game['location']
        game_stored.group = group
        game_stored.homeTeam = self.init_team(game['home_team'])
        game_stored.awayTeam = self.init_team(game['away_team'])
        game_stored.put()

    @classmethod
    def init_tree(self):
        # try to create that stuff safely
        if Fifa2010().tournament is None: Fifa2010().tournament = GroupGame.get_or_insert(key_name="fifa2010", name="FIFA 2010")
        if Fifa2010().groupstage is None: Fifa2010().groupstage = GroupGame.get_or_insert(key_name="groupstage", name="Group Stage", upgroup = Fifa2010().tournament)
        if Fifa2010().kostage is None: Fifa2010().kostage = GroupGame.get_or_insert(key_name="kostage", name="KO Stage", upgroup = Fifa2010().tournament)
        if Fifa2010().result is None: Fifa2010().result = LocalUser.get_or_insert(key_name="result", email="fifa@fifa.com", password='fifa')
        
        groupgames = fifa.get_games("index")
        for game in groupgames: self.init_group_game(game)

    @classmethod
    def single_game_result(self, user):
        def get_result(self):
            result = self.result_set.filter('user = ',user).get()
            if result is None:
                result = Result(user=user,singlegame=self)
            return result
        return get_result
