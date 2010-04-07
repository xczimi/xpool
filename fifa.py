import os
import re
import xpath
from xml.dom.minidom import parse, parseString
from datetime import datetime 

def read_clean_file(filename):
    """Read the filename and make sure it's parsable as xml.
    
    This currently means removing lines containing document.write"""
    clean = ''
    f = open(filename)
    try:
        for line in f:
            if None == re.search(r"document.write",line):
                clean = clean + line
    finally:
        f.close()
    return clean

def get_dom(stage):
    return xpath.find("//table[@class='fixture']", parseString(read_clean_file('fifa/'+stage+'.html')))

def timestring_to_datetime(str):
    #the title attribute has the GMT time as YYYYMMDDHHiiss after the comma @TODO
    return datetime(int(str[0:4]),int(str[4:6]),int(str[6:8]),int(str[8:10]),int(str[10:12]),0,0)

def get_team(match, home_or_away):
    new_team = {}
    td_xpath = 'td[contains(@class,"'+home_or_away+'Team")]'
    team_href = xpath.findvalue(td_xpath+'/a/@href',match)
    td_value = xpath.findvalue(td_xpath, match)
    if not team_href is None:
        new_team['name'] = td_value
        new_team['flag'] = xpath.findvalue('td/a[@href="'+team_href+'"]/img/@src',match)
    elif re.match(r'^[12][A-H]$', td_value):
        new_team['reference'] = {'rank': int(td_value[0]) , 'game_ref' : "Group "+td_value[1]}
    elif re.match(r'W([0-9]+)$', td_value):
        new_team['reference'] = {'rank': 1 , 'game_id' : int(td_value[1:])}
    else:
        new_team['reference'] = td_value
    return new_team
    
def get_games(stage):
    dom = get_dom(stage)
    games = []
    for groupdom in dom:
        group_name = xpath.findvalue('caption',groupdom)
        for match in xpath.find('tbody/tr',groupdom):
            new_game = {'id' : xpath.findvalue('td[contains(@class,"mNum")]',match), 
                        'group' : group_name ,
                        'time' : timestring_to_datetime(xpath.findvalue('td[contains(@class,"dt")]/span/@title',match).split(",")[1].encode('utf-8')) ,
                        'location' : xpath.findvalue('td/a[contains(@href,"destination")]',match).strip() ,
                        }
            for home_or_away in ['home','away']:
                new_game[home_or_away + '_team'] = get_team(match, home_or_away)
            print new_game
            games = games + [new_game]
    return games

def get_all_games():
    return get_games("index") + get_games("kostage")
    
#get_all_games()