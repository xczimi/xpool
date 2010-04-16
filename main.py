#!/usr/bin/env python
# -*- coding: UTF-8 -*-
#
# Copyright 2010 Peter Czimmermann  <xczimi@gmail.com>
#
from google.appengine.ext import webapp
from google.appengine.ext.webapp import util

import control 

def main():
    application = webapp.WSGIApplication([('/favicon.ico',webapp.RequestHandler),
                    ('/admin/(team)/(.*)', control.AdminHandler),
                    ('/admin/(.*)', control.AdminHandler),
                    ('/admin', control.AdminHandler),
                    ('/referrer/(.*)', control.ReferralHandler),
                    ('/(.*)', control.MainHandler)],debug=True)
    util.run_wsgi_app(application)

if __name__ == '__main__':
    main()
