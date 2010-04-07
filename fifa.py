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
                        'home_team' : xpath.findvalue('td[contains(@class,"homeTeam")]',match) ,
                        'away_team' : xpath.findvalue('td[contains(@class,"awayTeam")]',match)
                        }
            games = games + [new_game]
    return games

def get_all_games():
    return get_games("index") + get_games("kostage")