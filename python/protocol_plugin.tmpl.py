################################################################################
# Copyright ContinuousC. Licensed under the "Elastic License 2.0".             #
################################################################################

import sys,signal,socket,uuid
from rpc.server import RpcServer, RpcService, RpcException
from rpc.transport import UnixTransport
from rpc.json_encoding import JsonEncoding

class ProtocolService(RpcService):

    def __init__(self, plugin):
        self.__plugin = plugin

    def run(self, path, debug = False):

        server = RpcServer(self, transport = UnixTransport(path),
                           encoding = JsonEncoding(), debug=  debug)

        def handler(signum,frame):
            signal.signal(signal.SIGINT, signal.SIG_DFL)

        signal.signal(signal.SIGINT, handler)
        signal.signal(signal.SIGTERM, handler)

        server.start()
        signal.pause()
        server.shutdown()

    def session(self, info):
        return ProtocolSession()
        
    def request(self, session, req):
        {{ProtocolService}}

    def protocol(self, session):
        return self.__plugin.protocol

    def version(self, session):
        return self.__plugin.version

    def load_inputs(self, session, inputs):
        ref = str(uuid.uuid4())
        session.inputs[ref] = self.__plugin.load_inputs(inputs)
        return ref
        
    def load_config(self, session, config):
        ref = str(uuid.uuid4())
        session.configs[ref] = self.__plugin.load_config(config)
        return ref

    def show_queries(self, session, qry):
        return self.__plugin.show_queries(qry)

    def run_queries(self, session, qry, inputs, config):
        return self.__plugin.run_queries(qry, session.inputs[inputs], session.configs[config])

class ProtocolSession(object):
    def __init__(self):
        self.inputs = {}
        self.configs = {}
    
class ProtocolError(RpcException):
    pass

class ProtocolPlugin(object):
    pass
