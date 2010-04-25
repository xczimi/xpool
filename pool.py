
from model import Result

def pool_point(self, multiplier=1):
    point = 0
    result = self.reference()
    if self.homeScore >=0 and self.homeScore == result.homeScore: point = point + multiplier
    if self.awayScore >=0 and self.awayScore == result.awayScore: point = point + multiplier
    if self.home_w() and result.home_w(): point = point + multiplier * 2
    if self.home_d() and result.home_d(): point = point + multiplier * 2
    if self.home_l() and result.home_l(): point = point + multiplier * 2
    return point

Result.point = pool_point