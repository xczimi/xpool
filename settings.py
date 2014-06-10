#!/usr/bin/env python
# -*- coding: UTF-8 -*-
#
# Copyright 2010 Peter Czimmermann  <xczimi@gmail.com>
#

import os

LANGUAGE_CODE = 'en'
USE_I18N = True
appdir = os.path.abspath( os.path.dirname( __file__ ) )
LOCALE_PATHS = ( 
    os.path.join( appdir, 'locale' ),
)

TEMPLATE_DIRS = (
	os.path.join( appdir, 'views' ),
)
