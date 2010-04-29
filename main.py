#!/usr/bin/env python
# -*- coding: UTF-8 -*-
#
# Copyright 2010 Peter Czimmermann  <xczimi@gmail.com>
#
from google.appengine.ext import webapp
from google.appengine.ext.webapp import util

import control 

import os
from django.conf import settings
try:
    settings.configure()
except:
    pass
settings.LANGUAGE_CODE = 'en'
settings.USE_I18N = True
appdir = os.path.abspath( os.path.dirname( __file__ ) )
settings.LOCALE_PATHS = ( 
    os.path.join( appdir, 'locale' ),
 )
from django.utils.translation import *

def main():
    application = webapp.WSGIApplication([('/favicon.ico',webapp.RequestHandler),
                    ('/admin/(team)/(.*)', control.AdminHandler),
                    ('/admin/(.*)', control.AdminHandler),
                    ('/admin', control.AdminHandler),
                    ('/referrer/(.*)', control.ReferralHandler),
                    ('/games/(.*)', control.GamesHandler),
                    ('/games', control.GamesHandler),
                    ('/scoreboard/?(.*)',control.PoolHandler),
                    ('/mytips', control.MyTipsHandler),
                    ('/mytips/(.*)', control.MyTipsHandler),
                    ('/(profile|invite)', control.UserHandler),
                    ('/(.*)', control.MainHandler)],debug=True)
    util.run_wsgi_app(application)

if __name__ == '__main__':
    main()
