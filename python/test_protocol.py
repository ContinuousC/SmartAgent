################################################################################
# Copyright ContinuousC. Licensed under the "Elastic License 2.0".             #
################################################################################

from protocol_plugin import ProtocolPlugin

class TestProtocol(ProtocolPlugin):

    protocol = "test"
    version = "0.1.0"

    def load_inputs(self, inputs):
        return inputs

    def load_config(self, config):
        return config

    def show_queries(self, qry):
        return "queries: ..."

    def run_queries(self, qry, inputs, config):
        return { 'test_%s' % table: {
            "Ok": {
                "value": [ { 'test_%s' % field: { "Ok": { "unicodestring": "test" } }
                             for field in fields } ],
                "warnings": []
            }
        } for table,fields in qry.iteritems() }

if __name__ == '__main__':
    from protocol_plugin import ProtocolService
    ProtocolService(TestProtocol()).run('test.sock')
