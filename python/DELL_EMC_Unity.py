################################################################################
# Copyright ContinuousC. Licensed under the "Elastic License 2.0".             #
################################################################################

#!/usr/bin/env python

"""
Check_mk agent for Dell Emc Unity Management Pack.

$Authors$
$Version$
$Modified$
"""

import requests
import json
import argparse
import sys
import csv
import dateutil.parser
import time
import re
import os
import errno
from datetime import datetime
#from $mnChecks_OMD$.agent import *

from protocol_plugin import ProtocolService, ProtocolPlugin, ProtocolError

requests.packages.urllib3.disable_warnings(
    requests.packages.urllib3.exceptions.InsecurePlatformWarning)
requests.packages.urllib3.disable_warnings(
    requests.packages.urllib3.exceptions.InsecureRequestWarning)
requests.packages.urllib3.disable_warnings(
    requests.packages.urllib3.exceptions.SNIMissingWarning)

HEADERS = {
    'X-EMC-REST-CLIENT': 'true',
    'Accept': 'application/json',
    'Content-Type': 'application/json'
}

metrics = {u'sp.*.storage.lun.*.readsRate': u'Read', u'sp.*.blockCache.global.summary.writeMissesRate': u'Block Cache Write Misses', u'sp.*.iscsi.fePort.*.readsRate': u'Read', u'sp.*.storage.filesystem.*.writeBytesRate': u'Internal Write', u'sp.*.storage.filesystem.*.clientReadsRate': u'Client Read', u'sp.*.physical.disk.*.serviceTime': u'Average Service Time', u'sp.*.physical.disk.*.writesRate': u'Writes', u'sp.*.net.basic.outBytesRate': u'Network Out', u'sp.*.physical.disk.*.readsRate': u'Reads', u'sp.*.storage.lun.*.avgReadSize': u'Average Read', u'sp.*.blockCache.global.summary.dirtyBytes': u'Block Cache Dirty Data', u'sp.*.storage.filesystem.*.clientReadTimeAvg': u'Avg Client Read Time', u'sp.*.nfs.basic.readResponseTime': u'NFS Avg Read', u'sp.*.storage.lun.*.avgWriteSize': u'Average Write', u'sp.*.iscsi.fePort.*.writeBytesRate': u'Written', u'sp.*.nfs.totalCallsRate': u'Total NFS', u'sp.*.storage.filesystem.*.readBytesRate': u'Internal Read', u'sp.*.storage.lun.*.queueLength': u'Average Queue Length', u'sp.*.net.device.*.bytesInRate': u'Network In', u'sp.*.physical.disk.*.averageQueueLength': u'Avg Queue Length', u'sp.*.storage.filesystem.*.clientWritesRate': u'Client Write', u'sp.*.physical.disk.*.totalCallsRate': u'Total Calls', u'sp.*.blockCache.global.summary.readHitsRate': u'Block Cache Read Hits', u'sp.*.storage.filesystem.*.clientWriteTimeAvg': u'Avg Client Write Time', u'sp.*.nfs.basic.readBytesRate': u'NFS Read', u'sp.*.nfs.basic.totalIoTimeRate': u'NFS Total I/O', u'sp.*.nfs.basic.writeIoTimeRate': u'NFS Write IO', u'sp.*.storage.filesystem.*.readSizeAvg': u'Avg Internal Read Size', u'sp.*.storage.filesystem.*.clientWriteSizeAvg': u'Avg Client Write Size', u'sp.*.nfs.basic.readIoTimeRate': u'NFS Read IO', u'sp.*.storage.filesystem.*.writeSizeAvg': u'Avg Internal Write Size', u'sp.*.nfs.basic.writeBytesRate': u'NFS Write', u'sp.*.storage.filesystem.*.clientWriteBytesRate': u'Client Write', u'sp.*.physical.disk.*.readBytesRate': u'Read', u'sp.*.physical.disk.*.responseTime': u'Average Response Time', u'sp.*.storage.lun.*.responseTime': u'Response Time', u'sp.*.storage.filesystem.*.writesRate': u'Internal Write', u'sp.*.storage.filesystem.*.readsRate': u'Internal Read', u'sp.*.blockCache.global.summary.writeHitsRate': u'Block Cache Write Hits', u'sp.*.storage.lun.*.readBytesRate': u'Read', u'sp.*.nfs.basic.readAvgSize': u'NFS Avg Read Size', u'sp.*.net.device.*.pktsInRate': u'Network In', u'sp.*.nfs.basic.writeAvgSize': u'NFS Avg Write Size', u'sp.*.iscsi.fePort.*.writesRate': u'Write', u'sp.*.storage.lun.*.totalCallsRate': u'Total Call', u'sp.*.storage.filesystem.*.clientReadSizeAvg': u'Avg Client Read Size', u'sp.*.nfs.basic.writesRate': u'NFS Write', u'sp.*.nfs.basic.readsRate': u'NFS Read', u'sp.*.storage.lun.*.writesRate': u'Write', u'sp.*.storage.lun.*.writeBytesRate': u'Written', u'sp.*.nfs.basic.responseTime': u'NFS Avg IO', u'sp.*.storage.filesystem.*.clientReadBytesRate': u'Client Read', u'sp.*.net.basic.inBytesRate': u'Network In', u'sp.*.blockCache.global.summary.readMissesRate': u'Block Cache Read Misses', u'sp.*.cpu.summary.utilization': u'summary CPU Util', u'sp.*.net.device.*.pktsOutRate': u'Network Out', u'sp.*.iscsi.fePort.*.readBytesRate': u'Read', u'sp.*.physical.disk.*.writeBytesRate': u'Written', u'sp.*.nfs.basic.writeResponseTime': u'NFS Avg Write', u'sp.*.nfs.basic.totalIoCallsRate': u'NFS Total I/O', u'sp.*.net.device.*.bytesOutRate': u'Network Out'}

class UnityConfig(object):

    def __init__(self, config):
        self.hostname = config["hostname"]
        self.username = config["config"]["username"]
        self.password = config["config"]["password"]

    @classmethod
    def from_args(cls, args):
        return cls({
            "hostname": args.hostname,
            "config": {
                "username": args.username,
                "password": args.password,
            }
        })


class UnitySession(object):

    def __init__(self, config):

        self.config = config
        self.session = requests.Session()
        self.base_url = 'https://%s/api' % config.hostname

        self.timestamps_dir = '%s/var/lib/mnow/state/%s' %  (os.environ.get('OMD_ROOT', ''), config.hostname)
        self.timestamps_file = '%s/unity_timestamps.json' %  self.timestamps_dir
        if not os.path.exists(self.timestamps_dir):
            try:
                os.makedirs(self.timestamps_dir)
            except OSError as exc:
                if exc.errno != errno.EEXIST:
                    raise

        self.timestamps = json.load(open(self.timestamps_file)) \
            if os.path.exists(self.timestamps_file) else {}

    def save_state(self):
        json.dump(self.timestamps, open(self.timestamps_file, 'w+'))

    def query_api(self, url, method='get', data=None):
    
        errors = {
            404: "HTTP Status 404 - Not Found"
        }
        response = getattr(self.session, method.lower())(
            '%s/%s' % (self.base_url, url),
            headers=HEADERS, data=json.dumps(data), verify=False)
		
        if not response.status_code == 200:
            raise Exception("API responded with %d: %s" % (response.status_code,
		 					   errors.get(response.status_code, response.text)))
        try:
            return response.json()
        except Exception:
            raise Exception("API responded with %d: %s" % (response.status_code,
		 					   errors.get(response.status_code, response.text)))

    def login(self):
        url = '%s/types/loginSessionInfo/instances' % self.base_url
        return self.session.get(url, auth=(self.config.username, self.config.password),
                                headers=HEADERS, verify=False)

    def logout(self):
        self.query_api('types/loginSessionInfo/action/logout', 'post')


    def get_all_metrics(self):
        return self.query_api('types/metric/instances')


    def get_metric(self, metric_id):
        return self.query_api('instances/metric/%s' % metric_id)


    def follow_path(self, dictionary, path):
        tree = [dictionary]
        if isinstance(dictionary, list):
            tree = dictionary
        for step in path:
            new_branches = []
            for branch in tree:
                if step == '*':
                    new_branches += branch if branch else []
                else:
                    new_branches.append(branch.get(step, None) if isinstance(branch, dict) else None)
            tree = new_branches
        return tree


    def get_historic_metric_value(self, path, name, parameters):

        ts_entry = self.timestamps.get(path, {})
        old_timestamp = ts_entry.get('timestamp', 0)
        new_timestamp = None
        api_response = self.query_api('types/metricValue/instances?filter=path EQ "%s"' % path)
        updated_timestamp = False
        done = False
        count = 0
        timeseries = {}
        result = []

        while not done:
            entries = api_response['entries']

            for entry in api_response['entries']:
                content = entry['content']
                ts = time.mktime(dateutil.parser.parse(content['timestamp']).timetuple())
                if not new_timestamp:
                    new_timestamp = ts
                if new_timestamp == old_timestamp and not timeseries:
                    result = ts_entry['result']
                    done = True
                    break
                if ts > old_timestamp and 'values' in content and (count <= 5 or old_timestamp != 0):
                            count += 1
                            for sp, value in content['values'].iteritems():
                                    if isinstance(value, dict):
                                            for k,v in value.iteritems():
                                                    timeseries[(sp, k)] = timeseries.setdefault((sp, k), 0) + v
                                    else:
                                            timeseries[(sp, None)] = timeseries.setdefault((sp, None), 0) + value
                else:
                    done = True

            if not done:
                new = False
                for link in api_response['links']:
                    if link['rel'] == 'next':
                        new = True
                        api_response = self.query_api('types/metricValue/instances?filter=path EQ "%s"%s' % (path, link['href']))
                if not new:
                    done = True

        if not result:
            for (sp, k), v in timeseries.iteritems():
                row = {}
                if 'sp' in parameters:
                    row[parameters['sp']] = sp
                if 'key' in parameters and k != None:
                    row[parameters['key']] = k
                if 'value' in parameters:
                    row[parameters['value']] = v / count
                if 'name' in parameters:
                    row[parameters['name']] = name
                result.append(row)

        self.timestamps[path] = {"timestamp": new_timestamp, "result": result}

        return result


    def get_real_time_metric_value(self, resource, name, parameters):
        # real metrics not yet supported. need permissions to create queries on the host
        return []


    def fill_in_row(self, parameters, tree):
        row = {}
        for parameter, datafield in parameters.iteritems():
            branch = self.follow_path(tree, parameter.split('.'))
            if len(branch) == 1 and isinstance(branch[0], unicode) and re.compile(r'\d{1,5}:\d{2}:\d{2}.\d{3}').match(branch[0]):
                fields = branch[0].split(':')
                years = int(fields[0]) / 24 / 365
                days = (int(fields[0]) / 24) % 365
                hours = int(fields[0]) % 24
                minutes = int(fields[1])
                seconds = float(fields[2])
                result = ''
                if years:
                    result += '%d years' % years
                if days:
                    result += (', %d days' if result else '%d days') % days
                if hours:
                    result += (', %d hours' if result else '%d hours') % hours
                if minutes:
                    result += (', %d minutes' if result else '%d minutes') % minutes
                if seconds:
                    result += (', %f seconds' if result else '%f seconds') % seconds

                row[datafield] = result
            elif len(branch) == 1:
                row[datafield] = branch[0]
            else:
                row[datafield] = '\n'.join([str(leaf) for leaf in branch])
        return row


    def get_resource(self, resource, name, parameters):
        path = resource.split('.')
        parameters_to_request = [p.split('.')[0] for p in parameters.keys()] if len(path) == 1 else [path[1]]
        api_response = self.query_api('types/%s/instances?fields=%s' % (path[0], ','.join(parameters_to_request)))
        # print 'url:', '%s/types/%s/instances?fields=%s' % (self.base_url, path[0], ','.join(parameters_to_request))
        if isinstance(api_response, str):
            return api_response
        else:
            result = []
            if 'entries' not in api_response and 'error' in api_response:
                return ', '.join([message['en-US'] for message in api_response['error']['messages']])
            for entry in api_response['entries']:
                tree = entry['content']
                if len(path) > 1:
                    tree = self.follow_path(tree, path[1:])
                if isinstance(tree, dict):
                    result.append(self.fill_in_row(parameters, tree))
                else:
                    for branch in tree:
                        result.append(self.fill_in_row(parameters, branch))
            return result

    def get_pools(self, resource, name, parameters):
        path = resource.split('.')
        parameters_to_request = [p.split('.')[0] for p in parameters.keys()] if len(path) == 1 else [path[1]]
        api_response = self.query_api('types/pool/instances?fields=name,%s' % path[0])
        if isinstance(api_response, str):
            return api_response
        else:
            result = []
            if 'entries' not in api_response and 'error' in api_response:
                return ', '.join([message['en-US'] for message in api_response['error']['messages']])
            for entry in api_response['entries']:
                tree = entry['content']
                name = tree['name']
                if len(path) > 1:
                    tree = self.follow_path(tree, path)
                else:
                    tree = [tree]
                sub_table = []
                for branch in tree:
                    row = {}
                    for parameter, datatableid in parameters.iteritems():
                        leaf = self.follow_path(branch, parameter.split('.'))
                        if parameter == 'name':
                            row[datatableid] = name
                        elif len(leaf) == 1:
                            row[datatableid] = leaf[0]
                        else:
                            row[datatableid] = '\n'.join([str(vein) for vein in leaf])

                    sub_table.append(row)
                result += sub_table
            return result

    @classmethod
    def generate_input(cls):
        self = cls()
        self.__init__(UnityConfig.from_args(args))

        from progress.bar import Bar
        paths = set()
        names = {}

        self.login()
        with open(args.configuration_file, 'w+') as input_file:
            headers = ['CommandName', 'CommandLine',
                       'CommandDescription', 'ParameterName', 'ParameterHeader']
            input_file.write('%s\n' % ';'.join(headers))
            metric_links = self.get_all_metrics()['entries']
            with Bar('metrics', max=len(metric_links)) as bar:
                for metric_link in metric_links:
                    metric_summary = self.get_metric(
                        metric_link['content']['id'])['content']
                    if metric_summary['path'] not in paths:
                        paths.add(metric_summary['path'])
                        value = None
                        if metric_summary['isHistoricalAvailable']:
                            content = self.get_historic_metric_value(
                                metric_summary['path'])['entries'][0]['content']
                            if 'values' in content:
                                value = content['values'].values()[0]
                        if metric_summary['isRealtimeAvailable']:
                            # real metrics not yet supported. need permissions to create queries on the hostcontent = self.get_historic_metric_value(
                            pass
                        if value:
                            input_file.write('%s;%s;%s;%s;%s\n' % ('get_historic_metric_value' if metric_summary['isHistoricalAvailable'] else 'get_real_time_metric',
                                                                   metric_summary['path'], metric_summary['description'], 'sp', 'Storage Processor'))
                            if isinstance(value, dict):
                                input_file.write('%s;%s;%s;%s;%s\n' % ('get_historic_metric_value' if metric_summary['isHistoricalAvailable'] else 'get_real_time_metric',
                                                                       metric_summary['path'], metric_summary['description'], 'key', 'Key'))
                            input_file.write('%s;%s;%s;%s;%s\n' % ('get_historic_metric_value' if metric_summary['isHistoricalAvailable'] else 'get_real_time_metric',
                                                                   metric_summary['path'], metric_summary['description'], 'value', 'Value'))
                            input_file.write('%s;%s;%s;%s;%s\n' % ('get_historic_metric_value' if metric_summary['isHistoricalAvailable'] else 'get_real_time_metric',
                                                                   metric_summary['path'], metric_summary['description'], 'name', 'Name'))
                            names[metric_summary['path']] = metric_summary['name']
                    bar.next()
        with open(args.metric_names, 'w+') as f:
                    f.write(str(names))

    @classmethod
    def generate_output(cls, args):

        self = cls()
        self.__init__(UnityConfig.from_args(args))

        # Read config file
        config = []  # [row_in_configfile]
        # {Check : {(CommandName, CommandLine) : {ParameterName : DataFieldId}}
        checks = {}
        # {(CommandName, CommandLine) : {ParameterName : DataFieldId}}
        commands = {}
            # {CommandLine: name}
        try:
            with open(args.configuration_file, mode="r") as csv_file:
                for row in csv.DictReader(csv_file, delimiter=";"):
                    # Save raw config
                    config.append(row)

                    field = zip(row['ParameterName'].split(','), row['DataFieldId'].split(','))

                    # Save requested commands and parameters (global)
                    commands.setdefault((row['CommandName'], row['CommandLine']), {}) \
                        .update(field)

                    # Save requested commands and parameters (per check)
                    checks.setdefault(row["Check"], {}) \
                        .setdefault((row['CommandName'], row['CommandLine']), {}) \
                        .update(field)
        except Exception as e:
            print >>sys.stderr, "Failed to read configuration file %s\n%s" % (
                args.configuration_file, e)
            sys.exit(1)

        exception = None
        try:
            response = login(args.username, args.password)
            if response.status_code == 404: exception = 'HTTP Status 404 - Not Found'
            elif 'You are not authorized to view this page.' in response.text:
                exception = 'You are not authorized to view this page with user: %s' % args.username
        except Exception as e:
            exception = str(e)

        if exception:
            # {Check: {(CommandName, CommandLine): {protocol, state, error: value}}}
            dep_results = {}
            for check, commands in checks.iteritems():
                for command in commands:
                    dep_results.setdefault(check, {}).setdefault(str(command), {
                        'protocol': 'API',
                        'state': False,
                        'error': 'Failed to log in with user %s on host %s' % (args.hostname, args.username)
                    })
                print '<<<%s>>>' % check
            UpdateCache("Dell Emc Unity", args.hostname, dep_results)
            sys.exit(0)

        # Run commands
        data = {}  # {(CommandName, CommandLine) : [{ParameterName : Value}]}
        errors = {}  # {(CommandName, CommandLine) : String}
        for command, parameters in commands.iteritems():
            command_name, command_line = command
            try:
                output = getattr(self, command_name)(command_line, metrics.get(command_line), parameters)
                data[command] = output
            except Exception as e:
                errors[command] = str(e)
        # {Check: {(CommandName, CommandLine), [{DataFieldId : Value}]}}
        # output = {check: {command: data[command]} for check, commands in checks.iteritems() for command in commands}
        output = {}
        for check, check_dict in checks.iteritems():
            for command in check_dict:
                if command in data:
                    output.setdefault(check, {}).setdefault(command, data[command])
        # {Check: {(CommandName, CommandLine): {protocol, state, error: value}}}
        dep_results = {}
        for check, commands in checks.iteritems():
            for command in commands:
                dep_results.setdefault(check, {}).setdefault(str(command), {
                    'protocol': 'API',
                    'state': command not in errors,
                    'error': errors[command] if command in errors else None
                })

        for check, result in output.iteritems():
            output = '<<<%s>>>\n%s' % (check, agent_escape(result))
            print output

        UpdateCache("Dell Emc Unity", args.hostname, dep_results)

    def get_table(self, command_name, command_line, parameters):
        return { "warnings": [],
                 "value": [ { field: { "Ok": val }
                              for field, val in row.iteritems() }
                            for row in getattr(self, command_name)(
                                    command_line, metrics.get(command_line), parameters) ] }

class UnityProtocol(ProtocolPlugin):

    protocol = "DELL EMC Unity"
    version = "0.1.0"

    def load_inputs(self, inputs):
        return inputs[0]

    def load_config(self, config):
        return UnityConfig(config)

    def show_queries(self, qry):
        return "queries: ..."

    def run_queries(self, qry, inputs, config):

        session = UnitySession(config)

        try:
            response = session.login()
            if response.status_code == 404:
                raise ProtocolError('HTTP Status 404 - Not Found')
            elif 'You are not authorized to view this page.' in response.text:
                raise ProtocolError('You are not authorized to view this page with user: %s' % args.username)
        except Exception as e:
            raise ProtocolError(str(e))

        # Run commands
        res = { table: try_run(lambda: session.get_table(
            inputs['DataTables']['API_%s' % table]['CommandName'],
            inputs['DataTables']['API_%s' % table]['CommandLine'],
            { inputs['DataFields']['API_%s' % field]['ParameterName']:
              field for field in fields }))
                for table,fields in qry.iteritems() }

        session.logout()
        session.save_state()
        return res

def try_run(fun):
#    try:
        return { "Ok": fun() }
#    except Exception as e:
#        return { "Err": { "origin": "Protocol", "error": str(e) } }


if __name__ == '__main__':
    parser = argparse.ArgumentParser(
        description='Agent for the DELL EMC Unity MP')
    # parser.add_argument('-H', '--hostname', required=True,
    #                     help='Hostname / IP of the array')
    # parser.add_argument('-u', '--username', required=True,
    #                     help='username of the user used to log in')
    # parser.add_argument('-p', '--password', required=True,
    #                     help='password of the user used to log in')
    # parser.add_argument('-f', '--configuration_file', required=True,
    #                     help='configuration file of the MP. When used to generate input, write the input to this file')
    # parser.add_argument('-i', '--generate_input', action='store_true', default=False,
    #                     help='generate an inputfile for API2ETC')
    parser.add_argument('-s', '--server', default = "unity.sock",
                        help = 'Run as daemon on this socket')
    parser.add_argument('-D', '--debug', action = 'store_true',
                        help = 'Provide debug output')
    args = parser.parse_args()

    if args.server:
        ProtocolService(UnityProtocol()).run(args.server, debug = args.debug)

    else:
        session = UnitySession(UnityConfig.from_args(args))
        if args.generate_input:
            generate_input(args)
        else:
            generate_output(args)
        try:
            session.logout()
            session.save_state()
        except:
            pass

    sys.exit(0)
