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
            line = re.sub(r"&nbsp;","",line)
            line = re.sub(r"(<img[^>]+)>","\\1 />",line)
            clean = clean + line
    finally:
        f.close()
    #print clean
    return clean

def get_dom(stage):
    str = read_clean_file('uefa/'+stage+'.html')
    return xpath.find("//tbody[starts-with(@class,'m_')]", parseString(str))

def timestring_to_datetime(str):
    #the title attribute has the GMT time as YYYYMMDDHHiiss after the comma @TODO
    return datetime(int(str[0:4]),int(str[4:6]),int(str[6:8]),int(str[8:10]),int(str[10:12]),0,0)

def get_team(match, home_or_away):
    new_team = {}
    td_xpath = 'tr/td[contains(@class,"'+home_or_away+'")]'
    team_href = xpath.findvalue(td_xpath+'/a/@href',match)
    td_value = xpath.findvalue(td_xpath, match).strip()
    if not team_href is None:
        new_team['name'] = td_value
        new_team['flag'] = xpath.findvalue('//tr/td/a[@href="'+team_href+'"]/img/@src',match)
        new_team['href'] = team_href
    elif re.match(r'^[12][A-H]$', td_value):
        new_team['reference'] = {'rank': int(td_value[0]) , 'game_ref' : "Group "+td_value[1]}
    elif re.match(r'W([0-9]+)$', td_value):
        new_team['reference'] = {'rank': 1 , 'game_ref' : "KO "+td_value[1:]}
    elif re.match(r'L([0-9]+)$', td_value):
        new_team['reference'] = {'rank': 2 , 'game_ref' : "KO "+td_value[1:]}
    else:
        new_team['reference'] = td_value
    return new_team

def get_games(stage):
    dom = get_dom(stage)
    games = []
    for matchdom in dom:
        group_name = xpath.findvalue('tr/td/div/span[@class="gname"]//a',matchdom)
        if None == group_name:
            group_name = ""
        else:
            group_name = group_name.strip(' \t\n\r')
        dayvalue = xpath.findvalue('tr/td/div/span[@class="b dateT"]',matchdom).strip().split(' ')[0].encode('utf-8')
        hourvalue = xpath.findvalue('tr/td[@class="c b score nob"]//a',matchdom).strip().encode('utf-8')
        matchtime = datetime(2012,6,int(dayvalue),int(hourvalue.split('.')[0])-1,int(hourvalue.split('.')[1]),0,0)
        stadium = re.match(r".*Stadium:.*,(.*)",xpath.findvalue('tr[@class="referee_stadium"]/td',matchdom).encode('utf-8').strip(), flags = re.DOTALL).group(1).strip()
        hournode = xpath.findvalue('tr/td[@class="c b score nob"]//a/@href',matchdom)
        matchid = int(re.match(r"/uefaeuro/season=2012/matches/round=[0-9]+/match=([0-9]+)/index.html",hournode).group(1)) - 2003318
        #print [ group_name , dayvalue , hourvalue , matchtime, stadium , matchid, hournode ]
        new_game = {'id' : matchid ,
                    'group' : group_name ,
                    'time' : matchtime ,
                    'location' : stadium ,
                    }
        for home_or_away in ['home','away']:
            new_game[home_or_away + '_team'] = get_team(matchdom, home_or_away)
        print new_game 
        games.append(new_game)
    return games

def get_all_games():
    return get_games("group") + get_games("kostage")

#get_all_games()