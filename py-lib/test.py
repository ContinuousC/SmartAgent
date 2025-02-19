################################################################################
# Copyright ContinuousC. Licensed under the "Elastic License 2.0".             #
################################################################################

from smart_agent import Expr

e = Expr("Value = {substitute(@^5 + 3 > $var && @ < ${other}, ' ', '_')}")
print e 
print "%r" % e.data()
