#!/usr/bin/env python
#
# Copyright 2007 Google Inc.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
import os
import urllib

import webapp2
import control

app = webapp2.WSGIApplication([
                    ('/admin/(team)/(.*)', control.AdminHandler),
                    ('/admin/(game)/(.*)', control.AdminHandler),
                    ('/admin/(.*)', control.AdminHandler),
                    ('/admin', control.AdminHandler),
                    ('/referral/(.*)', control.ReferralHandler),
                    ('/referrer/(.*)', control.ReferralHandler),
                    ('/games/(.*)', control.GamesHandler),
                    ('/games', control.GamesHandler),
                    ('/today', control.TodayHandler),
                    ('/scoreboard/?(.*)',control.PoolHandler),
                    ('/perfect',control.PerfectHandler),
                    ('/alltips', control.AllTipsHandler),
                    ('/alltips/(.*)', control.AllTipsHandler),
                    ('/mytips', control.MyTipsHandler),
                    ('/mytips/(.*)', control.MyTipsHandler),
                    ('/(profile|invite)', control.UserHandler),
                    ('/(.*)', control.MainHandler)
                ], debug=True)
