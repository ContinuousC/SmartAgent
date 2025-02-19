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
        if 'protocol' == req:
            if not callable(getattr(self, 'protocol', None)):
                raise Exception('Request not implemented: protocol')
            return self.protocol(session, )
        
        if 'version' == req:
            if not callable(getattr(self, 'version', None)):
                raise Exception('Request not implemented: version')
            return self.version(session, )
        
        if 'load_inputs' in req:
            if not callable(getattr(self, 'load_inputs', None)):
                raise Exception('Request not implemented: load_inputs')
            return self.load_inputs(session, req['load_inputs']['input'])
        
        if 'unload_inputs' in req:
            if not callable(getattr(self, 'unload_inputs', None)):
                raise Exception('Request not implemented: unload_inputs')
            return self.unload_inputs(session, req['unload_inputs']['input'])
        
        if 'load_config' in req:
            if not callable(getattr(self, 'load_config', None)):
                raise Exception('Request not implemented: load_config')
            return self.load_config(session, req['load_config']['config'])
        
        if 'unload_config' in req:
            if not callable(getattr(self, 'unload_config', None)):
                raise Exception('Request not implemented: unload_config')
            return self.unload_config(session, req['unload_config']['config'])
        
        if 'show_queries' in req:
            if not callable(getattr(self, 'show_queries', None)):
                raise Exception('Request not implemented: show_queries')
            return self.show_queries(session, req['show_queries']['query'], req['show_queries']['input'], req['show_queries']['config'])
        
        if 'run_queries' in req:
            if not callable(getattr(self, 'run_queries', None)):
                raise Exception('Request not implemented: run_queries')
            return self.run_queries(session, req['run_queries']['query'], req['run_queries']['input'], req['run_queries']['config'])
        
        if 'get_tables' in req:
            if not callable(getattr(self, 'get_tables', None)):
                raise Exception('Request not implemented: get_tables')
            return self.get_tables(session, req['get_tables']['input'])
        
        if 'get_fields' in req:
            if not callable(getattr(self, 'get_fields', None)):
                raise Exception('Request not implemented: get_fields')
            return self.get_fields(session, req['get_fields']['input'])
        
        raise Exception('Request not implemented: ' + req)

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
