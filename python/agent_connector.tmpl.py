################################################################################
# Copyright ContinuousC. Licensed under the "Elastic License 2.0".             #
################################################################################

from utils import AgentException

class AgentConnector(object):
    def __init__(self, logger, agent):
        self.logger = logger
        self.agent = agent

    def __request(self, command, options={}):
        self.logger.terminal('Send command to agent: {}'.format(command), 3)
        if options: self.logger.terminal('With options: {}'.format(str(options)), 3)
        #self.logger.progress(True)
        res = self.logger.agent(self.agent, command, options)
        #self.logger.progress(False)
        if 'Ok' in res:
            self.logger.terminal('Response: {}'.format(str(res['Ok'])), 3)
            return res['Ok']
        elif 'Err' in res:
            self.logger.terminal('Error: {}'.format(str(res['Err'])), 3)
            raise AgentException(self.logger, res['Err'])
        else:
            self.logger.error(res)
            raise AgentException(self.logger, str(res))

    {{AgentService}}
